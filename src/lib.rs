//! An implementation of the dnstap protocol
//!
//! This crate implements the sender part of the [dnstap](http://dnstap.info/) protocol,
//! a flexible, structured binary log format for DNS software.

#![allow(deprecated)]

mod context;
mod dns_message;
mod dnstap_builder;
mod dnstap_pb;
mod dnstap_writer;

pub use crate::dnstap_pb::message::Type as MessageType;
pub use crate::dnstap_pb::SocketFamily;
pub use crate::dnstap_pb::SocketProtocol;

pub use crate::dns_message::*;
pub use crate::dnstap_builder::*;
pub use crate::dnstap_writer::{DNSTapPendingWriter, DNSTapWriter, Sender};
