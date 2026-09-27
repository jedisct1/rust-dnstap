use crate::context::*;
use crate::dns_message::*;
use crate::dnstap_builder::*;
use mio::{Events, Poll, Waker};
use std::any::Any;
use std::io;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread;

pub struct DNSTapPendingWriter {
    dnstap_tx: SyncSender<DNSMessage>,
    waker: Arc<Waker>,
    context: Context,
}

impl DNSTapPendingWriter {
    /// Creates a `DNSTapPendingWriter` object. The communication channel is established at this
    /// point, and the `sender()` function can be used in order to get `Sender` objects.
    pub fn listen(builder: DNSTapBuilder) -> Result<DNSTapPendingWriter, &'static str> {
        let (dnstap_tx, dnstap_rx) = mpsc::sync_channel(builder.backlog);
        let mio_poll = Poll::new().map_err(|_| "Unable to create a poll instance")?;
        let waker = Arc::new(
            Waker::new(mio_poll.registry(), NOTIFY_TOK).map_err(|_| "Unable to create a waker")?,
        );
        assert!(builder.unix_socket_path.is_some());
        let context = Context {
            mio_poll,
            retry_deadline: None,
            dnstap_rx,
            unix_socket_path: builder.unix_socket_path,
            unix_stream: None,
            frame_stream: None,
        };
        Ok(DNSTapPendingWriter {
            dnstap_tx,
            waker,
            context,
        })
    }

    /// Spawns a new task handling writes to the socket.
    pub fn start(self) -> io::Result<DNSTapWriter> {
        DNSTapWriter::start(self)
    }

    /// Returns a cloneable `Sender` object that can used to send DNS messages.
    #[inline]
    pub fn sender(&self) -> Sender {
        Sender {
            dnstap_tx: self.dnstap_tx.clone(),
            waker: self.waker.clone(),
        }
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
    dnstap_tx: SyncSender<DNSMessage>,
    waker: Arc<Waker>,
    tid: thread::JoinHandle<()>,
}

impl DNSTapWriter {
    /// Spawns a new task handling writes to the socket.
    pub fn start(dnstap_pending_writer: DNSTapPendingWriter) -> io::Result<DNSTapWriter> {
        let DNSTapPendingWriter {
            dnstap_tx,
            waker,
            mut context,
        } = dnstap_pending_writer;
        context.connect();
        let mut events = Events::with_capacity(512);
        let tid = thread::Builder::new()
            .name("dnstap".to_owned())
            .spawn(move || {
                loop {
                    let timeout = context.poll_timeout();
                    if context.mio_poll.poll(&mut events, timeout).is_err() {
                        break;
                    }
                    context.maybe_retry();
                    for event in events.iter() {
                        match event.token() {
                            UNIX_SOCKET_TOK => context.write_cb(event),
                            NOTIFY_TOK => context.message_cb(),
                            _ => unreachable!(),
                        }
                    }
                }
                if let Some(frame_stream) = context.frame_stream {
                    frame_stream.finish().unwrap();
                }
            })?;
        Ok(DNSTapWriter {
            dnstap_tx,
            waker,
            tid,
        })
    }

    pub fn join(self) -> Result<(), Box<dyn Any + Send + 'static>> {
        self.tid.join()
    }

    /// Returns a cloneable `Sender` object that can used to send DNS messages.
    #[inline]
    pub fn sender(&self) -> Sender {
        Sender {
            dnstap_tx: self.dnstap_tx.clone(),
            waker: self.waker.clone(),
        }
    }
}

/// `Sender` is a cloneable structure to send DNS messages.
#[derive(Clone)]
pub struct Sender {
    dnstap_tx: SyncSender<DNSMessage>,
    waker: Arc<Waker>,
}

impl Sender {
    /// Sends a DNS message.
    #[inline]
    pub fn send(&self, dns_message: DNSMessage) -> Result<(), TrySendError<DNSMessage>> {
        self.dnstap_tx.try_send(dns_message)?;
        let _ = self.waker.wake();
        Ok(())
    }
}
