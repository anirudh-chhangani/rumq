use crate::{Error, FixedHeader, ByteLengths, Utf8Pair};
use bytes::{Buf, Bytes};
use crate::reasoncodes::ReasonCode;
use alloc::string::String;
use crate::control::properties::{PropertyIdentifiers, Properties, extract_properties};

#[derive(Debug, Clone, PartialEq)]
pub struct ConnackProperties {
    pub session_expiry_interval: Option<u32>,
    pub assigned_client_identifier: Option<String>,
    pub server_keep_alive: Option<u16>,
    pub authentication_method: Option<String>,
    pub authentication_data: Option<String>,
    pub response_info: Option<String>,
    pub server_info: Option<String>,
    pub reason_string: Option<String>,
    pub user_property: Option<Utf8Pair>,
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
    pub properties: Option<ConnackProperties>,
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

        let _props = extract_properties(&mut bytes)?;

        let connack = match _props {
            Some(props) => {
                let properties = Some(
                    ConnackProperties {
                        session_expiry_interval: props.session_expiry_interval,
                        assigned_client_identifier: props.assigned_client_identifier,
                        server_keep_alive: props.server_keep_alive,
                        authentication_method: props.authentication_method,
                        authentication_data: props.authentication_data,
                        response_info: props.response_info,
                        server_info: props.server_info,
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                        receive_maximum: props.receive_maximum,
                        topic_alias_maximum: props.topic_alias_maximum,
                        maximum_qos: props.maximum_qos,
                        retain_available: props.retain_available,
                        maximum_packet_size: props.maximum_packet_size,
                        wildcard_subscription_available: props.wildcard_subscription_available,
                        subscription_identifier_available: props.subscription_identifier_available,
                        shared_subscription_available: props.shared_subscription_available,
                    }
                );

                ConnAck { session_present, reason_code, properties }
            }
            None => ConnAck { session_present, reason_code, properties: None }

        };

        Ok(connack)
    }
}

impl ConnAck {
    pub fn new(reason_code: u8, session_present: bool, properties: Option<ConnackProperties>) -> ConnAck {
        ConnAck { session_present, reason_code, properties }
    }
}

// #[cfg(test)]
// mod test_connack {
//     use crate::*;
//     use alloc::borrow::ToOwned;
//     use alloc::vec;
//     use bytes::{Bytes, BytesMut};
//     use pretty_assertions::assert_eq;
//     use crate::reasoncodes::ReasonCode;
//     use crate::control::connack::Properties;
//
//     #[test]
//     fn connack_stitching_works_correctly() {
//         let mut stream = bytes::Bytes::new();
//         let packet_stream = &[
//             0b0010_0000,
//             0x02, // packet type, flags and remaining len
//             0x01,
//             0x00, // variable header. connack flags, reason code
//             0xDE,
//             0xAD,
//             0xBE,
//             0xEF, // extra control in the stream
//         ];
//
//         stream.extend_from_slice(&packet_stream[..]);
//         let packet = mqtt_read(&mut stream, 100).unwrap();
//         let packet = match packet {
//             Packet::ConnAck(packet) => packet,
//             packet => panic!("Invalid packet = {:?}", packet),
//         };
//
//         assert_eq!(
//             packet,
//             ConnAck {
//                 session_present: true,
//                 reason_code: ReasonCode::SUCCESS,
//                 properties: None,
//             }
//         );
//     }
// }
