use std::net::IpAddr;
use std::time;

use ::{MessageType, SocketFamily, SocketProtocol};
use ::dnstap_pb;

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
    pub fn new(identity: Option<Vec<u8>>,
               version: Option<Vec<u8>>,
               message_type: MessageType)
               -> DNSMessage {
        DNSMessage {
            identity: identity,
            version: version,
            message_type: message_type,
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

    pub fn to_protobuf(self) -> dnstap_pb::Dnstap {
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
                debug_assert_eq!(socket_family.unwrap_or(SocketFamily::INET),
                                 SocketFamily::INET);
                Some(SocketFamily::INET)
            }
            Some(IpAddr::V6(ip6)) => {
                msg.set_response_address(ip6.octets().to_vec());
                debug_assert_eq!(socket_family.unwrap_or(SocketFamily::INET6),
                                 SocketFamily::INET6);
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
            msg.set_query_port(query_port as u32);
        }
        if let Some(response_port) = self.response_port {
            msg.set_response_port(response_port as u32);
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
            msg.set_query_time_nsec(response_time.subsec_nanos());
        }
        d.set_message(msg);
        d
    }
}
