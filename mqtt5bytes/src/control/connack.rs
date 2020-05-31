use crate::{Error, FixedHeader};
use bytes::{Buf, Bytes};
use crate::reasoncodes::ReasonCode;
use alloc::string::String;

struct PropertyIdentifiers;

impl PropertyIdentifiers {
    const SESSION_EXPIRY_INTERVAL: u8 = 17;
    const RECEIVE_MAXIMUM: u8 = 33;
    const MAXIMUM_QOS: u8 = 36;
    const RETAIN_AVAILABLE: u8 = 37;
    const MAXIMUM_PACKET_SIZE: u8 = 39;
    const ASSIGNED_CLIENT_IDENTIFIER: u8 = 18;
    const TOPIC_ALIAS_MAXIMUM: u8 = 34;
    const REASON_STRING: u8 = 31;
    const USER_PROPERTY: u8 = 38;
    const WILDCARD_SUBSCRIPTION_AVAILABLE: u8 = 40;
    const SUBSCRIPTION_IDENTIFIER_AVAILABLE: u8 = 41;
    const SHARED_SUBSCRIPTION_AVAILABLE: u8 = 42;
    const SERVER_KEEP_ALIVE: u8 = 19;
    const RESPONSE_INFO: u8 = 26;
    const SERVER_INFO: u8 = 28;
    const AUTHENTICATION_METHOD: u8 = 21;
    const AUTHENTICATION_DATA: u8 = 22;
}


#[derive(Debug, Clone, PartialEq)]
pub struct Properties {
    pub session_expiry_interval: Option<u32>,
    pub assigned_client_identifier: Option<String>,
    pub server_keep_alive: Option<u16>,
    pub authentication_method: Option<String>,
    pub authentication_data: Option<String>,
    pub response_info: Option<String>,
    pub server_info: Option<String>,
    pub reason_string: Option<String>,
    pub user_property: Option<String>,
    pub receive_maximum: Option<u16>,
    pub topic_alias_maximum: Option<u16>,
    pub maximum_qos: Option<u8>,
    pub retain_available: Option<u8>,
    pub maximum_packet_size: Option<u32>,
    pub wildcard_subscription_available: Option<u8>,
    pub subscription_identifier_available: Option<u8>,
    pub shared_subscription_available: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnAck {
    pub session_present: bool,
    pub reason_code: u8,
    pub properties: Option<Properties>,
}

impl ConnAck {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);

        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let flags = bytes.get_u8();
        let session_present = (flags & 0x01) == 1;

        let reason_code = bytes.get_u8();

        let prop_length: u8 = bytes.get_u8();

// get properties if avail
        let mut properties: Option<Properties> = None;

        if prop_length > 0 {
            let mut session_expiry_interval: Option<u32> = None;
            let mut assigned_client_identifier: Option<String> = None;
            let mut server_keep_alive: Option<u16> = None;
            let mut authentication_method: Option<String> = None;
            let mut authentication_data = None;
            let mut response_info = None;
            let mut server_info = None;
            let mut reason_string = None;
            let mut user_property = None;
            let mut receive_maximum = None;
            let mut topic_alias_maximum = None;
            let mut maximum_qos = None;
            let mut retain_available = None;
            let mut maximum_packet_size = None;
            let mut wildcard_subscription_available = None;
            let mut subscription_identifier_available = None;
            let mut shared_subscription_available = None;

            // WIP : parse loop for properties
            {
                let ident = bytes.get_u8();
                // match identifier to extract properties
                match ident {
                    PropertyIdentifiers::SESSION_EXPIRY_INTERVAL => {
                        session_expiry_interval = Some(bytes.get_u32());
                    }
                    PropertyIdentifiers::RECEIVE_MAXIMUM => {
                    }
                    PropertyIdentifiers::MAXIMUM_QOS => {}
                    PropertyIdentifiers::RETAIN_AVAILABLE => {}
                    PropertyIdentifiers::MAXIMUM_PACKET_SIZE => {}
                    PropertyIdentifiers::ASSIGNED_CLIENT_IDENTIFIER => {}
                    PropertyIdentifiers::TOPIC_ALIAS_MAXIMUM => {}
                    PropertyIdentifiers::REASON_STRING => {}
                    PropertyIdentifiers::USER_PROPERTY => {}
                    PropertyIdentifiers::WILDCARD_SUBSCRIPTION_AVAILABLE => {}
                    PropertyIdentifiers::SUBSCRIPTION_IDENTIFIER_AVAILABLE => {}
                    PropertyIdentifiers::SHARED_SUBSCRIPTION_AVAILABLE => {}
                    PropertyIdentifiers::SERVER_KEEP_ALIVE => {}
                    PropertyIdentifiers::RESPONSE_INFO => {}
                    PropertyIdentifiers::SERVER_INFO => {}
                    PropertyIdentifiers::AUTHENTICATION_METHOD => {}
                    PropertyIdentifiers::AUTHENTICATION_DATA => {}
                    _ => {}
                }
            }

            Properties {
                session_expiry_interval,
                assigned_client_identifier,
                server_keep_alive,
                authentication_method,
                authentication_data,
                response_info,
                server_info,
                reason_string,
                user_property,
                receive_maximum,
                topic_alias_maximum,
                maximum_qos,
                retain_available,
                maximum_packet_size,
                wildcard_subscription_available,
                subscription_identifier_available,
                shared_subscription_available,
            };
        }

        let connack = ConnAck { session_present, reason_code, properties };

        Ok(connack)
    }
}

impl ConnAck {
    pub fn new(reason_code: u8, session_present: bool, properties: Option<Properties>) -> ConnAck {
        ConnAck { session_present, reason_code, properties }
    }
}

#[cfg(test)]
mod test_connack {
    use crate::*;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;
    use crate::reasoncodes::ReasonCode;
    use crate::control::connack::Properties;

    #[test]
    fn connack_stitching_works_correctly() {
        let mut stream = bytes::BytesMut::new();
        let packet_stream = &[
            0b0010_0000,
            0x02, // packet type, flags and remaining len
            0x01,
            0x00, // variable header. connack flags, reason code
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];

        stream.extend_from_slice(&packet_stream[..]);
        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::ConnAck(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            ConnAck {
                session_present: true,
                reason_code: ReasonCode::SUCCESS,
                properties: None,
            }
        );
    }
}
