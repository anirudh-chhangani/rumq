use crate::{extract_mqtt_string, qos, Error, FixedHeader, LastWill, Protocol, Utf8Pair};
use crate::control::properties::extract_properties;
use alloc::string::String;
use bytes::{Buf, Bytes};
use core::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub receive_maximum: Option<u16>,
    pub maximum_packet_size: Option<u32>,
    pub topic_alias_maximum: Option<u16>,
    pub request_response_information: Option<u8>,
    pub request_problem_information: Option<u8>,
    pub user_property: Option<Utf8Pair>,
    pub authentication_methods: Option<String>,
    pub authentication_data: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectFlags {
    pub username: bool,
    pub password: bool,
    pub will_retain: bool,
    pub will_qos: u8,
    pub will_flag: bool,
    pub clean_session: bool,
    pub reserved: u8, // must be set to 0, otherwise malformed packet
}

#[derive(Clone, PartialEq)]
pub struct Connect {
    /// Mqtt protocol version
    pub protocol: Protocol,
    /// Mqtt protocol version
    pub proto_name: String,
    /// Mqtt keep alive time
    pub keep_alive: u16,
    /// Packet Flags
    pub flags: ConnectFlags,
    /// Client Id
    pub client_id: String,
    /// Connect properties
    pub properties: Option<ConnectProperties>,
}

impl Connect {
    pub fn new<S: Into<String>>(id: S) -> Connect {
        Connect {
            protocol: Protocol::MQTT(5),
            proto_name: String::from("MQTT"),
            keep_alive: 60,
            client_id: id.into(),
            flags: ConnectFlags{
                username: false,
                password: false,
                will_retain: false,
                will_qos: 0,
                will_flag: false,
                clean_session: true,
                reserved: 0
            },
            properties: None,
        }
    }
}

impl Connect {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Connect, Error> {

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let protocol_name = extract_mqtt_string(&mut bytes)?;
        let protocol_level = bytes.get_u8();
        if protocol_name != "MQTT" {
            return Err(Error::InvalidProtocol);
        }

        let protocol = match protocol_level {
            5 => Protocol::MQTT(5),
            num => return Err(Error::InvalidProtocolLevel(num)),
        };

        let flag_bytes = bytes.get_u8();
        let keep_alive = bytes.get_u16();
        let client_id = extract_mqtt_string(&mut bytes)?;

        let flags = ConnectFlags{
            username: flag_bytes & (1 << 0b111) != 0,
            password: flag_bytes & (1 << 0b110) != 0,
            will_retain: flag_bytes & (1 << 0b101) != 0,
            will_qos: 0,
            will_flag: flag_bytes & (1 << 0b10) != 0,
            clean_session: flag_bytes & (1 << 0b1) != 0,
            reserved: flag_bytes & (1 << 0b0)
        };

        let _props = extract_properties(&mut bytes)?;

        let conn_props = match _props {
            Some(props) => {
                Some(
                    ConnectProperties {
                        session_expiry_interval: props.session_expiry_interval,
                        receive_maximum: props.receive_maximum,
                        maximum_packet_size: props.maximum_packet_size,
                        topic_alias_maximum: props.topic_alias_maximum,
                        request_response_information: props.request_response_info,
                        request_problem_information: props.request_problem_info,
                        user_property: props.user_property,
                        authentication_methods: props.authentication_method,
                        authentication_data: props.authentication_data,
                    }
                )
            }
            None => None
        };

        let connect = Connect {
            protocol,
            proto_name: String::from("MQTT"),
            keep_alive,
            client_id,
            flags,
            properties: conn_props,
        };

        Ok(connect)
    }
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Protocol = {:?}, Keep alive = {:?}, Client id = {}, Clean session = {}",
            self.protocol, self.keep_alive, self.client_id, self.flags.clean_session,
        )
    }
}

#[cfg(test)]
mod test_connect {
    use crate::*;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;

    #[test]
    fn connect_stitching_works_correctly() {
        let packet_stream = &[
            0x10, 16, 00, 04, 0x4d, 51, 54, 54, 05, 02, 00, 0x3c,
            00, 00, 09, 63, 00, 10, 0x6c, 69, 65, 0x6e, 74, 0x2d, 69, 64,
        ];

        let mut stream = bytes::Bytes::from(&packet_stream[..]);
        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Connect(connect) => connect,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(packet.protocol, Protocol::MQTT(5));
        assert_eq!(packet.proto_name, "MQTT");
        assert_eq!(packet.keep_alive, 60);
        assert_eq!(packet.client_id, "client-id");
        assert_eq!(packet.flags.username, false);
        assert_eq!(packet.flags.password, false);
        assert_eq!(packet.flags.will_retain, false);
        assert_eq!(packet.flags.will_flag, false);
        assert_eq!(packet.flags.will_qos, 0);
        assert_eq!(packet.flags.clean_session, false);
        assert_eq!(packet.flags.reserved, 0);

        assert_eq!(
            packet,
            Connect {
                protocol: Protocol::MQTT(5),
                proto_name: String::from("MQTT"),
                keep_alive: 60,
                client_id: "client-id".to_owned(),
                flags: ConnectFlags{
                    username: false,
                    password: false,
                    will_retain: false,
                    will_flag: false,
                    will_qos: 0,
                    clean_session: true,
                    reserved: 0
                },
                properties: None,
            }
        );
    }
}
