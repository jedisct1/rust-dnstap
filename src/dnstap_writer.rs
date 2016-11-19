use context::*;
use dns_message::*;
use dnstap_builder::*;
use mio::*;
use std::any::Any;
use std::thread;

/// DNSTapWriter is responsible for receiving DNS messages, connecting (and automatically
/// reconnecting) to a UNIX socket, and asynchronously pushing the serialized data using
/// frame stream protocol.
///
/// # Example
/// ```
/// let dnstap_writer =
///     DNSTapWriter::build().backlog(4096).unix_socket_path("/tmp/dnstap.sock").start();
/// dnstap_writer.join().unwrap();
/// ```
pub struct DNSTapWriter {
    dnstap_tx: channel::SyncSender<DNSMessage>,
    tid: thread::JoinHandle<()>,
}

impl DNSTapWriter {
    pub fn build() -> DNSTapBuilder {
        DNSTapBuilder::default()
    }

    /// Spawns a new task handling writes to the socket
    pub fn start(builder: DNSTapBuilder) -> DNSTapWriter {
        let (dnstap_tx, dnstap_rx) = channel::sync_channel(builder.backlog);
        let mio_poll = Poll::new().unwrap();
        mio_poll.register(&dnstap_rx,
                      NOTIFY_TOK,
                      Ready::readable(),
                      PollOpt::edge() | PollOpt::oneshot())
            .unwrap();
        let mio_timers = timer::Timer::default();
        mio_poll.register(&mio_timers, TIMER_TOK, Ready::readable(), PollOpt::edge()).unwrap();
        assert!(builder.unix_socket_path.is_some());
        let mut context = Context {
            mio_poll: mio_poll,
            mio_timers: mio_timers,
            dnstap_rx: dnstap_rx,
            unix_socket_path: builder.unix_socket_path,
            unix_stream: None,
            frame_stream: None,
        };
        context.connect();
        let mut events = Events::with_capacity(512);
        let tid = thread::Builder::new()
            .name("dnstap".to_owned())
            .spawn(move || {
                while context.mio_poll.poll(&mut events, None).is_ok() {
                    for event in events.iter() {
                        match event.token() {
                            UNIX_SOCKET_TOK => context.write_cb(event),
                            NOTIFY_TOK => context.message_cb(),
                            TIMER_TOK => context.connect(),
                            _ => unreachable!(),
                        }
                    }
                }
                if let Some(frame_stream) = context.frame_stream {
                    frame_stream.finish().unwrap();
                }
            })
            .unwrap();
        DNSTapWriter {
            dnstap_tx: dnstap_tx,
            tid: tid,
        }
    }

    pub fn join(self) -> Result<(), Box<Any + Send + 'static>> {
        self.tid.join()
    }

    /// Sends a DNS message
    #[inline]
    pub fn send(&self, dns_message: DNSMessage) -> Result<(), channel::TrySendError<DNSMessage>> {
        self.dnstap_tx.try_send(dns_message)
    }

    /// Returns a cloneable `Sender` object that can used to send DNS messages
    #[inline]
    pub fn sender(&self) -> Sender {
        Sender(self.dnstap_tx.clone())
    }
}

/// Sender is a cloneable structure that to send DNS messages
#[derive(Clone)]
pub struct Sender(channel::SyncSender<DNSMessage>);

impl Sender {
    /// Sends a DNS message
    #[inline]
    pub fn send(&self, dns_message: DNSMessage) -> Result<(), channel::TrySendError<DNSMessage>> {
        self.0.try_send(dns_message)
    }
}
