
//! An implementation of the dnstap protocol
//!
//! This crate implements the sender part of the [dnstap](http://dnstap.info/) protocol,
//! a flexible, structured binary log format for DNS software.

extern crate framestream;
extern crate mio;
extern crate protobuf;

mod context;
mod dns_message;
mod dnstap_builder;
mod dnstap_pb;
mod dnstap_writer;

pub use dnstap_pb::Message_Type as MessageType;
pub use dnstap_pb::SocketFamily;
pub use dnstap_pb::SocketProtocol;

pub use dns_message::*;
pub use dnstap_builder::*;
pub use dnstap_writer::{DNSTapWriter, Sender};
