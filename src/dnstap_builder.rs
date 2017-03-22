use std::path::{Path, PathBuf};
use std::usize;

use dnstap_writer::DNSTapPendingWriter;

const DEFAULT_BACKLOG: usize = 4096;

/// Builds a `DNSTapPendingWriter` object, to eventually start a
/// `DNSTapWriter` service.
#[derive(Clone, Hash)]
pub struct DNSTapBuilder {
    pub backlog: usize,
    pub unix_socket_path: Option<PathBuf>,
}

impl DNSTapBuilder {
    /// Returns a `DNSTapBuilder` boilerplate
    pub fn default() -> DNSTapBuilder {
        DNSTapBuilder {
            backlog: DEFAULT_BACKLOG,
            unix_socket_path: None,
        }
    }

    /// Maximum number of messages to keep in queue.
    pub fn backlog(mut self, backlog: usize) -> Self {
        self.backlog = backlog;
        self
    }

    /// Path to the UNIX socket to send dnstap data to.
    pub fn unix_socket_path<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.unix_socket_path = Some(PathBuf::from(path.as_ref()));
        self
    }

    /// Creates a DNSTapPendingWriter object. The communication channel is established at this
    /// point, and the `sender()` function can be used in order to get `Sender` objects.
    pub fn listen(self) -> Result<DNSTapPendingWriter, &'static str> {
        DNSTapPendingWriter::listen(self)
    }
}
