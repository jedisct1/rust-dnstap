use std::net::IpAddr;
use std::time;

use crate::dnstap_pb;
use crate::{MessageType, SocketFamily, SocketProtocol};

/// A DNS message.
///
/// All properties are optional except the message type.
///
/// Although `socket_family` can be explicitly set, it can also be automatically
/// inferred from `query_address` or `response_address` if these are present.
#[derive(Clone, Hash)]
pub struct DNSMessage {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub message_type: MessageType,
    pub socket_family: Option<SocketFamily>,
    pub socket_protocol: Option<SocketProtocol>,
    pub query_address: Option<IpAddr>,
    pub query_port: Option<u16>,
    pub query_time: Option<time::Duration>,
    pub query_packet: Option<Vec<u8>>,
    pub response_address: Option<IpAddr>,
    pub response_port: Option<u16>,
    pub response_time: Option<time::Duration>,
    pub response_packet: Option<Vec<u8>>,
    pub bailiwick: Option<String>,
}

impl DNSMessage {
    /// Returns a minimal DNS message
    pub fn new(
        identity: Option<Vec<u8>>,
        version: Option<Vec<u8>>,
        message_type: MessageType,
    ) -> DNSMessage {
        DNSMessage {
            identity,
            version,
            message_type,
            socket_family: None,
            socket_protocol: None,
            query_address: None,
            query_port: None,
            query_time: None,
            query_packet: None,
            response_address: None,
            response_port: None,
            response_time: None,
            response_packet: None,
            bailiwick: None,
        }
    }

    #[doc(hidden)]
    pub fn into_protobuf(self) -> dnstap_pb::Dnstap {
        let mut d = dnstap_pb::Dnstap::new();
        if let Some(identity) = self.identity {
            d.set_identity(identity);
        }
        if let Some(version) = self.version {
            d.set_version(version);
        }
        d.set_field_type(dnstap_pb::Dnstap_Type::MESSAGE);
        let mut msg = dnstap_pb::Message::new();
        msg.set_field_type(self.message_type);
        let mut socket_family = self.socket_family;
        socket_family = match self.query_address {
            Some(IpAddr::V4(ip4)) => {
                msg.set_query_address(ip4.octets().to_vec());
                Some(SocketFamily::INET)
            }
            Some(IpAddr::V6(ip6)) => {
                msg.set_query_address(ip6.octets().to_vec());
                Some(SocketFamily::INET6)
            }
            None => socket_family,
        };
        socket_family = match self.response_address {
            Some(IpAddr::V4(ip4)) => {
                msg.set_response_address(ip4.octets().to_vec());
                debug_assert_eq!(
                    socket_family.unwrap_or(SocketFamily::INET),
                    SocketFamily::INET
                );
                Some(SocketFamily::INET)
            }
            Some(IpAddr::V6(ip6)) => {
                msg.set_response_address(ip6.octets().to_vec());
                debug_assert_eq!(
                    socket_family.unwrap_or(SocketFamily::INET6),
                    SocketFamily::INET6
                );
                Some(SocketFamily::INET6)
            }
            None => socket_family,
        };
        if let Some(socket_family) = socket_family {
            msg.set_socket_family(socket_family);
        }
        if let Some(socket_protocol) = self.socket_protocol {
            msg.set_socket_protocol(socket_protocol);
        }
        if let Some(query_port) = self.query_port {
            msg.set_query_port(u32::from(query_port));
        }
        if let Some(response_port) = self.response_port {
            msg.set_response_port(u32::from(response_port));
        }
        if let Some(query_packet) = self.query_packet {
            msg.set_query_message(query_packet);
        }
        if let Some(response_packet) = self.response_packet {
            msg.set_query_message(response_packet);
        }
        if let Some(query_time) = self.query_time {
            msg.set_query_time_sec(query_time.as_secs());
            msg.set_query_time_nsec(query_time.subsec_nanos());
        }
        if let Some(response_time) = self.response_time {
            msg.set_response_time_sec(response_time.as_secs());
            msg.set_response_time_nsec(response_time.subsec_nanos());
        }
        d.set_message(msg);
        d
    }
}

#[derive(Clone, Hash)]
pub struct AuthQuery {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub socket_protocol: SocketProtocol,
    pub query_address: IpAddr,
    pub query_port: u16,
    pub query_time: time::Duration,
    pub query_packet: Vec<u8>,
}

impl Into<DNSMessage> for AuthQuery {
    fn into(self) -> DNSMessage {
        let mut dns_message = DNSMessage::new(self.identity, self.version, MessageType::AUTH_QUERY);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.query_address = Some(self.query_address);
        dns_message.query_port = Some(self.query_port);
        dns_message.query_time = Some(self.query_time);
        dns_message.query_packet = Some(self.query_packet);
        dns_message
    }
}

#[derive(Clone, Hash)]
pub struct AuthResponse {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub message_type: MessageType,
    pub socket_protocol: SocketProtocol,
    pub query_address: IpAddr,
    pub query_port: u16,
    pub query_time: time::Duration,
    pub response_packet: Vec<u8>,
}

impl Into<DNSMessage> for AuthResponse {
    fn into(self) -> DNSMessage {
        let mut dns_message =
            DNSMessage::new(self.identity, self.version, MessageType::AUTH_RESPONSE);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.query_address = Some(self.query_address);
        dns_message.query_port = Some(self.query_port);
        dns_message.query_time = Some(self.query_time);
        dns_message.response_packet = Some(self.response_packet);
        dns_message
    }
}

#[derive(Clone, Hash)]
pub struct ResolverQuery {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub socket_protocol: SocketProtocol,
    pub query_time: time::Duration,
    pub query_packet: Vec<u8>,
    pub response_address: IpAddr,
    pub response_port: u16,
    pub bailiwick: String,
}

impl Into<DNSMessage> for ResolverQuery {
    fn into(self) -> DNSMessage {
        let mut dns_message =
            DNSMessage::new(self.identity, self.version, MessageType::RESOLVER_QUERY);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.query_time = Some(self.query_time);
        dns_message.query_packet = Some(self.query_packet);
        dns_message.response_address = Some(self.response_address);
        dns_message.response_port = Some(self.response_port);
        dns_message.bailiwick = Some(self.bailiwick);
        dns_message
    }
}

#[derive(Clone, Hash)]
pub struct ResolverResponse {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub socket_protocol: SocketProtocol,
    pub query_time: time::Duration,
    pub response_address: IpAddr,
    pub response_port: u16,
    pub response_packet: Vec<u8>,
    pub response_time: time::Duration,
    pub bailiwick: String,
}

impl Into<DNSMessage> for ResolverResponse {
    fn into(self) -> DNSMessage {
        let mut dns_message =
            DNSMessage::new(self.identity, self.version, MessageType::RESOLVER_RESPONSE);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.query_time = Some(self.query_time);
        dns_message.response_address = Some(self.response_address);
        dns_message.response_port = Some(self.response_port);
        dns_message.response_packet = Some(self.response_packet);
        dns_message.response_time = Some(self.response_time);
        dns_message.bailiwick = Some(self.bailiwick);
        dns_message
    }
}

#[derive(Clone, Hash)]
pub struct ClientQuery {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub socket_family: SocketFamily,
    pub socket_protocol: SocketProtocol,
    pub query_time: time::Duration,
    pub query_packet: Vec<u8>,
}

impl Into<DNSMessage> for ClientQuery {
    fn into(self) -> DNSMessage {
        let mut dns_message =
            DNSMessage::new(self.identity, self.version, MessageType::CLIENT_QUERY);
        dns_message.socket_family = Some(self.socket_family);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.query_time = Some(self.query_time);
        dns_message.query_packet = Some(self.query_packet);
        dns_message
    }
}

#[derive(Clone, Hash)]
pub struct ClientResponse {
    pub identity: Option<Vec<u8>>,
    pub version: Option<Vec<u8>>,
    pub socket_family: SocketFamily,
    pub socket_protocol: SocketProtocol,
    pub response_time: time::Duration,
    pub response_packet: Vec<u8>,
}

impl Into<DNSMessage> for ClientResponse {
    fn into(self) -> DNSMessage {
        let mut dns_message =
            DNSMessage::new(self.identity, self.version, MessageType::CLIENT_RESPONSE);
        dns_message.socket_family = Some(self.socket_family);
        dns_message.socket_protocol = Some(self.socket_protocol);
        dns_message.response_time = Some(self.response_time);
        dns_message.response_packet = Some(self.response_packet);
        dns_message
    }
}
