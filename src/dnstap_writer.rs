use context::*;
use dns_message::*;
use dnstap_builder::*;
use mio::*;
use std::any::Any;
use std::io;
use std::thread;

pub struct DNSTapPendingWriter {
    dnstap_tx: channel::SyncSender<DNSMessage>,
    context: Context,
}

impl DNSTapPendingWriter {
    /// Creates a `DNSTapPendingWriter` object. The communication channel is established at this
    /// point, and the `sender()` function can be used in order to get `Sender` objects.
    pub fn listen(builder: DNSTapBuilder) -> Result<DNSTapPendingWriter, &'static str> {
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
        let context = Context {
            mio_poll: mio_poll,
            mio_timers: mio_timers,
            retry_timeout: None,
            dnstap_rx: dnstap_rx,
            unix_socket_path: builder.unix_socket_path,
            unix_stream: None,
            frame_stream: None,
        };
        Ok(DNSTapPendingWriter {
               dnstap_tx: dnstap_tx,
               context: context,
           })
    }

    /// Spawns a new task handling writes to the socket.
    pub fn start(self) -> io::Result<DNSTapWriter> {
        DNSTapWriter::start(self)
    }

    /// Returns a cloneable `Sender` object that can used to send DNS messages.
    #[inline]
    pub fn sender(&self) -> Sender {
        Sender(self.dnstap_tx.clone())
    }
}

/// `DNSTapWriter` is responsible for receiving DNS messages, connecting (and automatically
/// reconnecting) to a UNIX socket, and asynchronously pushing the serialized data using
/// frame stream protocol.
///
/// # Example
/// ```no_run
/// use dnstap::DNSTapBuilder;
///
/// let dnstap_pending_writer = DNSTapBuilder::default()
///     .backlog(4096)
///     .unix_socket_path("/tmp/dnstap.sock")
///     .listen().unwrap();
///
/// let dnstap_writer = dnstap_pending_writer.start().unwrap();
///
/// dnstap_writer.join().unwrap();
/// ```
pub struct DNSTapWriter {
    dnstap_tx: channel::SyncSender<DNSMessage>,
    tid: thread::JoinHandle<()>,
}

impl DNSTapWriter {
    /// Spawns a new task handling writes to the socket.
    pub fn start(mut dnstap_pending_writer: DNSTapPendingWriter) -> io::Result<DNSTapWriter> {
        dnstap_pending_writer.context.connect();
        let mut events = Events::with_capacity(512);
        let dnstap_tx = dnstap_pending_writer.dnstap_tx.clone();
        let tid = try!(thread::Builder::new().name("dnstap".to_owned()).spawn(move || {
            while dnstap_pending_writer.context
                      .mio_poll
                      .poll(&mut events, None)
                      .is_ok() {
                for event in events.iter() {
                    match event.token() {
                        UNIX_SOCKET_TOK => dnstap_pending_writer.context.write_cb(event),
                        NOTIFY_TOK => dnstap_pending_writer.context.message_cb(),
                        TIMER_TOK => dnstap_pending_writer.context.connect(),
                        _ => unreachable!(),
                    }
                }
            }
            if let Some(frame_stream) = dnstap_pending_writer.context.frame_stream {
                frame_stream.finish().unwrap();
            }
        }));
        Ok(DNSTapWriter {
               dnstap_tx: dnstap_tx,
               tid: tid,
           })
    }

    pub fn join(self) -> Result<(), Box<Any + Send + 'static>> {
        self.tid.join()
    }

    /// Returns a cloneable `Sender` object that can used to send DNS messages.
    #[inline]
    pub fn sender(&self) -> Sender {
        Sender(self.dnstap_tx.clone())
    }
}

/// `Sender` is a cloneable structure to send DNS messages.
#[derive(Clone)]
pub struct Sender(channel::SyncSender<DNSMessage>);

impl Sender {
    /// Sends a DNS message.
    #[inline]
    pub fn send(&self, dns_message: DNSMessage) -> Result<(), channel::TrySendError<DNSMessage>> {
        self.0.try_send(dns_message)
    }
}
