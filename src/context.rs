use crate::dns_message::*;
use framestream::EncoderWriter;
use mio::event::Event;
use mio::net::UnixStream as MioUnixStream;
use mio::{Interest, Poll, Token};
use protobuf::Message;
use std::io::{self, BufWriter, Write};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const BUFFER_SIZE: usize = 262_144;
pub const CONTENT_TYPE: &str = "protobuf:dnstap.Dnstap";
pub const RETRY_DELAY_SECS: u64 = 1;

pub const NOTIFY_TOK: Token = Token(usize::MAX - 1);
pub const UNIX_SOCKET_TOK: Token = Token(usize::MAX - 3);

pub struct Context {
    pub mio_poll: Poll,
    pub retry_deadline: Option<Instant>,
    pub dnstap_rx: std::sync::mpsc::Receiver<DNSMessage>,
    pub unix_socket_path: Option<PathBuf>,
    pub unix_stream: Option<MioUnixStream>,
    pub frame_stream: Option<EncoderWriter<BufWriter<StdUnixStream>>>,
}

impl Context {
    /// Duration to pass to `poll`, so that the event loop wakes up when it is time to retry a
    /// connection. `None` means there is nothing pending and `poll` can block indefinitely.
    pub fn poll_timeout(&self) -> Option<Duration> {
        self.retry_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    /// Reconnect once the retry delay has elapsed.
    pub fn maybe_retry(&mut self) {
        if let Some(deadline) = self.retry_deadline {
            if Instant::now() >= deadline {
                self.retry_deadline = None;
                self.connect();
            }
        }
    }

    fn schedule_retry(&mut self) {
        self.retry_deadline = Some(Instant::now() + Duration::from_secs(RETRY_DELAY_SECS));
    }

    pub fn message_cb(&mut self) {
        if let Some(unix_stream) = self.unix_stream.as_mut() {
            self.mio_poll
                .registry()
                .reregister(unix_stream, UNIX_SOCKET_TOK, Interest::WRITABLE)
                .unwrap();
        }
    }

    pub fn write_cb(&mut self, event: &Event) {
        if self.frame_stream.is_none() {
            debug_assert!(self.unix_stream.is_none());
            return;
        }
        if event.is_write_closed() || event.is_read_closed() || event.is_error() {
            self.unix_stream = None;
            self.frame_stream = None;
            self.schedule_retry();
            return;
        }
        let frame_stream = self.frame_stream.as_mut().unwrap();
        while let Ok(dns_message) = self.dnstap_rx.try_recv() {
            let dns_message_bytes = dns_message.into_protobuf().write_to_bytes().unwrap();
            match frame_stream.write_all(&dns_message_bytes).or_else(|_| {
                let _ = frame_stream.flush();
                frame_stream.write_all(&dns_message_bytes)
            }) {
                Err(ref e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::Interrupted =>
                {
                    self.mio_poll
                        .registry()
                        .reregister(
                            self.unix_stream.as_mut().unwrap(),
                            UNIX_SOCKET_TOK,
                            Interest::WRITABLE,
                        )
                        .unwrap();
                    return;
                }
                Err(e) => {
                    let _ = frame_stream.flush();
                    panic!("Cannot write to the frame stream any more: {}", e)
                }
                _ => {}
            }
        }
        let _ = frame_stream.flush();
    }

    pub fn connect(&mut self) {
        if self.frame_stream.is_some() {
            debug_assert!(self.unix_stream.is_some());
            return;
        }
        assert!(self.unix_socket_path.is_some());
        let path = self.unix_socket_path.clone().unwrap();
        let std_stream = match StdUnixStream::connect(&path) {
            Ok(std_stream) => std_stream,
            Err(_) => {
                self.schedule_retry();
                return;
            }
        };
        if std_stream.set_nonblocking(true).is_err() {
            self.schedule_retry();
            return;
        }
        let writer_stream = match std_stream.try_clone() {
            Ok(writer_stream) => writer_stream,
            Err(_) => {
                self.schedule_retry();
                return;
            }
        };
        let mut unix_stream = MioUnixStream::from_std(std_stream);
        let frame_stream = EncoderWriter::new(
            BufWriter::with_capacity(BUFFER_SIZE, writer_stream),
            Some(CONTENT_TYPE.to_owned()),
        );
        if self
            .mio_poll
            .registry()
            .register(&mut unix_stream, UNIX_SOCKET_TOK, Interest::WRITABLE)
            .is_err()
        {
            self.schedule_retry();
            return;
        }
        self.unix_stream = Some(unix_stream);
        self.frame_stream = Some(frame_stream);
    }
}
