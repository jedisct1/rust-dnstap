use std::path::{Path, PathBuf};
use std::usize;

use dnstap_writer::DNSTapWriter;

const DEFAULT_BACKLOG: usize = 4096;

#[derive(Clone, Hash)]
pub struct DNSTapBuilder {
    pub backlog: usize,
    pub unix_socket_path: Option<PathBuf>,
}

impl DNSTapBuilder {
    pub fn default() -> DNSTapBuilder {
        DNSTapBuilder {
            backlog: DEFAULT_BACKLOG,
            unix_socket_path: None,
        }
    }

    pub fn backlog(mut self, backlog: usize) -> Self {
        self.backlog = backlog;
        self
    }

    pub fn unix_socket_path<P>(mut self, path: P) -> Self
        where P: AsRef<Path>
    {
        self.unix_socket_path = Some(PathBuf::from(path.as_ref()));
        self
    }

    pub fn start(self) -> DNSTapWriter {
        DNSTapWriter::start(self)
    }
}
