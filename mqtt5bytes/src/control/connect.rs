use alloc::string::String;
use core::fmt;

use bytes::{Buf, Bytes, BytesMut, BufMut};

use crate::{decode_variable_byte, Error, extract_mqtt_string, FixedHeader, LastWill, Protocol, qos, Utf8Pair, PacketType, encode_utf_string};
use crate::control::properties::extract_properties;
use crate::Protocol::MQTT;

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

impl ConnectProperties {
    pub(crate) fn disassemble(self) -> Result<Bytes, Error> {
        Ok(Bytes::new())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WillProperties {
    pub will_delay_interval: Option<u32>,
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<String>,
    pub user_property: Option<Utf8Pair>,
}

impl WillProperties {
    pub(crate) fn disassemble(self) -> Result<Bytes, Error> {
        Ok(Bytes::new())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectPayload {
    pub client_id: String,
    pub will_props: Option<WillProperties>,
    pub will_topic: Option<String>,
    pub will_payload: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}


impl ConnectPayload {
    pub(crate) fn disassemble(self) -> Result<Bytes, Error> {
        Ok(Bytes::new())
    }
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


impl ConnectFlags {
    pub(crate) fn disassemble(self) -> Result<Bytes, Error> {
        Ok(Bytes::new())
    }
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
    /// Connect properties
    pub properties: Option<ConnectProperties>,
    /// Connect payload
    pub payload: ConnectPayload,
}

impl Connect {
    pub fn new<S: Into<String>>(id: S) -> Connect {
        Connect {
            protocol: Protocol::MQTT(5),
            proto_name: String::from("MQTT"),
            keep_alive: 60,
            flags: ConnectFlags {
                username: false,
                password: false,
                will_retain: false,
                will_qos: 0,
                will_flag: false,
                clean_session: true,
                reserved: 0,
            },
            properties: None,
            payload: ConnectPayload {
                client_id: String::new(),
                will_props: None,
                will_topic: None,
                will_payload: None,
                username: None,
                password: None,
            },
        }
    }
}

impl Connect {
    pub(crate) fn disassemble(self) -> Result<Bytes, Error> {
        let mut fixed_header = BytesMut::new();
        fixed_header.reserve(3);
        fixed_header.put_u8(1);
        fixed_header.put_u8(1); // packet len

        let mut var_header = BytesMut::new();
        var_header.reserve(1);
        var_header.put(encode_utf_string(String::from("MQTT"))?);
        var_header.put_u8(5); // proto version
        var_header.put(self.flags.disassemble()?);
        var_header.put_u16(self.keep_alive);

        let _ = match self.properties {
            Some(p) => { var_header.put(p.disassemble()?); }
            None => ()
        };


        let mut payload = BytesMut::new();
        payload.reserve(1);
        payload.put(self.payload.disassemble()?);

        let mut packet = BytesMut::new();
        packet.extend(fixed_header);
        packet.extend(var_header);
        packet.extend(payload);
        Ok(packet.to_bytes())
    }

    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Connect, Error> {
        bytes.advance(fixed_header.header_len);
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

        let flags = ConnectFlags {
            username: flag_bytes & (1 << 0b111) != 0,
            password: flag_bytes & (1 << 0b110) != 0,
            will_retain: flag_bytes & (1 << 0b101) != 0,
            will_qos: 0,
            will_flag: flag_bytes & (1 << 0b10) != 0,
            clean_session: flag_bytes & (1 << 0b1) != 0,
            reserved: flag_bytes & (1 << 0b0),
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

        // client id is compulsory
        let client_id = extract_mqtt_string(&mut bytes)?;

        // checking rest of the payload
        let mut will_props: WillProperties = WillProperties {
            will_delay_interval: None,
            payload_format_indicator: None,
            message_expiry_interval: None,
            content_type: None,
            response_topic: None,
            correlation_data: None,
            user_property: None,
        };
        let mut will_topic: String = String::new();
        let mut will_payload: String = String::new();

        if flags.will_flag {
            let _props = extract_properties(&mut bytes)?;
            let will_prop = match _props {
                Some(props) => {
                    WillProperties {
                        will_delay_interval: props.will_delay_interval,
                        payload_format_indicator: props.payload_format_indicator,
                        message_expiry_interval: props.message_expiry_interval,
                        content_type: props.content_type,
                        response_topic: props.response_topic,
                        correlation_data: props.correlation_data,
                        user_property: props.user_property,
                    }
                }
                _ => WillProperties {
                    will_delay_interval: None,
                    payload_format_indicator: None,
                    message_expiry_interval: None,
                    content_type: None,
                    response_topic: None,
                    correlation_data: None,
                    user_property: None,
                }
            };
            // below two props will be also present when the will flag is set
            will_topic = extract_mqtt_string(&mut bytes)?;
            will_payload = extract_mqtt_string(&mut bytes)?;
        }

        let mut username: String = String::new();
        if flags.username {
            username = extract_mqtt_string(&mut bytes)?;
        }

        let mut password: String = String::new();
        if flags.password {
            password = extract_mqtt_string(&mut bytes)?;
        }

        let payload = ConnectPayload {
            client_id,
            will_props: Some(will_props),
            will_topic: Some(will_topic),
            will_payload: Some(will_payload),
            username: Some(username),
            password: Some(password),
        };

        let connect = Connect {
            protocol,
            proto_name: String::from("MQTT"),
            keep_alive,
            flags,
            properties: conn_props,
            payload,
        };

        Ok(connect)
    }
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Protocol = {:?}, Keep alive = {:?}, Client id = {}, Clean session = {}",
            self.protocol, self.keep_alive, self.payload.client_id, self.flags.clean_session,
        )
    }
}

#[cfg(test)]
mod test_connect {
    use alloc::borrow::ToOwned;
    use alloc::vec;

    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;

    use crate::*;

    #[test]
    fn connect_read_works_correctly() {
        let packet_stream = &[
            0x10, 0x16, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x02, 0x00, 0x3c, 0x00,
            0x00, 0x09, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x2d, 0x69, 0x64,
        ];

        let mut stream = bytes::BytesMut::from(&packet_stream[..]);
        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Connect(connect) => connect,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(packet.protocol, Protocol::MQTT(5));
        assert_eq!(packet.proto_name, "MQTT");
        assert_eq!(packet.keep_alive, 60);
        assert_eq!(packet.payload.client_id, "client-id");
        assert_eq!(packet.flags.username, false);
        assert_eq!(packet.flags.password, false);
        assert_eq!(packet.flags.will_retain, false);
        assert_eq!(packet.flags.will_flag, false);
        assert_eq!(packet.flags.will_qos, 0);
        assert_eq!(packet.flags.clean_session, true);
        assert_eq!(packet.flags.reserved, 0);
    }

    #[test]
    fn connect_write_works_correctly() -> Result<(), Error> {
        let conn = Connect {
            protocol: Protocol::MQTT(5),
            proto_name: String::from("MQTT"),
            keep_alive: 60,
            flags: ConnectFlags {
                username: false,
                password: false,
                will_retain: false,
                will_flag: false,
                will_qos: 0,
                clean_session: true,
                reserved: 0,
            },
            properties: None,
            payload: ConnectPayload {
                client_id: String::from("client-id"),
                will_props: None,
                will_topic: None,
                will_payload: None,
                username: None,
                password: None,
            },
        };

        let conn_stream = mqtt_write(Packet::Connect(conn))?;

        let packet_stream = &[
            0x10, 0x16, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x02, 0x00, 0x3c, 0x00,
            0x00, 0x09, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x2d, 0x69, 0x64,
        ];

        assert_eq!(
            packet_stream.to_vec(),
            conn_stream.to_vec()
        );
        Ok(())
    }
}
