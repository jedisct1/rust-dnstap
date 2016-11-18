use dns_message::*;
use framestream::EncoderWriter;
use mio::*;
use mio::deprecated::{UnixSocket, UnixStream};
use protobuf::*;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time;
use std::usize;

pub const BUFFER_SIZE: usize = 262144;
pub const CONTENT_TYPE: &'static str = "protobuf:dnstap.Dnstap";
pub const RETRY_DELAY_SECS: u64 = 1;

pub const NOTIFY_TOK: Token = Token(usize::MAX - 1);
pub const TIMER_TOK: Token = Token(usize::MAX - 2);
pub const UNIX_SOCKET_TOK: Token = Token(usize::MAX - 3);

pub struct Context {
    pub mio_poll: Poll,
    pub mio_timers: timer::Timer<Token>,
    pub dnstap_rx: channel::Receiver<DNSMessage>,
    pub unix_socket_path: Option<PathBuf>,
    pub unix_stream: Option<UnixStream>,
    pub frame_stream: Option<EncoderWriter<BufWriter<UnixStream>>>,
}

impl Context {
    pub fn message_cb(&mut self) {
        if let Some(ref unix_stream) = self.unix_stream {
            self.mio_poll
                .reregister(unix_stream,
                            UNIX_SOCKET_TOK,
                            Ready::writable(),
                            PollOpt::edge() | PollOpt::oneshot())
                .unwrap();
        }
    }

    pub fn write_cb(&mut self, event: Event) {
        if self.frame_stream.is_none() {
            debug_assert!(self.unix_stream.is_none());
            return;
        }
        if event.kind().is_hup() || event.kind().is_error() {
            self.unix_stream = None;
            self.frame_stream = None;
            self.mio_timers
                .set_timeout(time::Duration::from_secs(RETRY_DELAY_SECS), TIMER_TOK)
                .unwrap();
            return;
        }
        let frame_stream = self.frame_stream.as_mut().unwrap();
        while let Ok(dns_message) = self.dnstap_rx.try_recv() {
            let dns_message_bytes = dns_message.to_protobuf().write_to_bytes().unwrap();
            match frame_stream.write_all(&dns_message_bytes).or_else(|_| {
                let _ = frame_stream.flush();
                frame_stream.write_all(&dns_message_bytes)
            }) {
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock ||
                              e.kind() == io::ErrorKind::Interrupted => {
                    self.mio_poll
                        .reregister(self.unix_stream.as_ref().unwrap(),
                                    UNIX_SOCKET_TOK,
                                    Ready::writable(),
                                    PollOpt::edge() | PollOpt::oneshot())
                        .unwrap();
                    break;
                }
                Err(e) => {
                    let _ = frame_stream.flush();
                    panic!("Cannot write to the frame stream any more: {}", e)
                }
                _ => {}
            }
        }
        let _ = frame_stream.flush();
        self.mio_poll
            .reregister(&self.dnstap_rx,
                        NOTIFY_TOK,
                        Ready::readable(),
                        PollOpt::edge() | PollOpt::oneshot())
            .unwrap();
    }

    pub fn connect(&mut self) {
        if self.frame_stream.is_some() {
            debug_assert!(self.unix_stream.is_some());
            return;
        }
        assert!(self.unix_socket_path.is_some());
        let unix_socket = UnixSocket::stream().unwrap();
        let unix_stream = match unix_socket.connect(&self.unix_socket_path.clone().unwrap()) {
            Ok((unix_stream, _connected)) => unix_stream,
            Err(_) => {
                self.mio_timers
                    .set_timeout(time::Duration::from_secs(RETRY_DELAY_SECS), TIMER_TOK)
                    .unwrap();
                return;
            }
        };
        let frame_stream = EncoderWriter::new(BufWriter::with_capacity(BUFFER_SIZE,
                                                                       unix_stream.try_clone()
                                                                           .unwrap()),
                                              Some(CONTENT_TYPE.to_owned()));
        self.mio_poll
            .register(&unix_stream,
                      UNIX_SOCKET_TOK,
                      Ready::writable(),
                      PollOpt::edge() | PollOpt::oneshot())
            .unwrap();
        self.unix_stream = Some(unix_stream);
        self.frame_stream = Some(frame_stream);
    }
}
