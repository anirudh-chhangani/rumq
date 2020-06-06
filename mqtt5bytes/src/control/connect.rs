use crate::{extract_mqtt_string, qos, Error, FixedHeader, LastWill, Protocol};
use alloc::string::String;
use bytes::{Buf, Bytes};
use core::fmt;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub receive_maximum: Option<u16>,
    pub maximum_packet_size: Option<u32>,
    pub topic_alias_maximum: Option<u16>,
    pub request_response_information: Option<u8>,
    pub request_problem_information: Option<u8>,
    pub user_property: Option<String>,
    pub authentication_methods: Option<String>,
    pub authentication_data: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct Connect {
    /// Mqtt protocol version
    pub protocol: Protocol,
    /// Mqtt keep alive time
    pub keep_alive: u16,
    /// Client Id
    pub client_id: String,
    /// Clean session. Asks the broker to clear previous state
    pub clean_session: bool,
    /// Will that broker needs to publish when the client disconnects
    pub last_will: Option<LastWill>,
    /// Username of the client
    pub username: Option<String>,
    /// Password of the client
    pub password: Option<String>,
    /// Connect properties
    pub properties: Option<ConnectProperties>,
}

impl Connect {
    pub fn new<S: Into<String>>(id: S) -> Connect {
        Connect {
            protocol: Protocol::MQTT(4),
            keep_alive: 10,
            client_id: id.into(),
            clean_session: true,
            last_will: None,
            username: None,
            password: None,
            properties: None
        }
    }

    pub fn set_username<S: Into<String>>(&mut self, u: S) -> &mut Connect {
        self.username = Some(u.into());
        self
    }

    pub fn set_password<S: Into<String>>(&mut self, p: S) -> &mut Connect {
        self.password = Some(p.into());
        self
    }

    pub fn len(&self) -> usize {
        let mut len = 8 + "MQTT".len() + self.client_id.len();

        // lastwill len
        if let Some(ref last_will) = self.last_will {
            len += 4 + last_will.topic.len() + last_will.message.len();
        }

        // username len
        if let Some(ref username) = self.username {
            len += 2 + username.len();
        }

        // password len
        if let Some(ref password) = self.password {
            len += 2 + password.len();
        }

        len
    }
}

impl Connect {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Connect, Error> {

        let extract_last_will = |connect_flags: u8, mut bytes: &mut Bytes| -> Result<Option<LastWill>, Error> {
            let last_will = match connect_flags & 0b100 {
                0 if (connect_flags & 0b0011_1000) != 0 => {
                    return Err(Error::IncorrectPacketFormat);
                }
                0 => None,
                _ => {
                    let will_topic = extract_mqtt_string(&mut bytes)?;
                    let will_message = extract_mqtt_string(&mut bytes)?;
                    let will_qos = qos((connect_flags & 0b11000) >> 3)?;
                    Some(LastWill {
                        topic: will_topic,
                        message: will_message,
                        qos: will_qos,
                        retain: (connect_flags & 0b0010_0000) != 0,
                    })
                }
            };

            Ok(last_will)
        };

        let extract_username_password =
            |connect_flags: u8, mut bytes: &mut Bytes| -> Result<(Option<String>, Option<String>), Error> {
                let username = match connect_flags & 0b1000_0000 {
                    0 => None,
                    _ => Some(extract_mqtt_string(&mut bytes)?),
                };

                let password = match connect_flags & 0b0100_0000 {
                    0 => None,
                    _ => Some(extract_mqtt_string(&mut bytes)?),
                };

                Ok((username, password))
            };

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let protocol_name = extract_mqtt_string(&mut bytes)?;
        let protocol_level = bytes.get_u8();
        if protocol_name != "MQTT" {
            return Err(Error::InvalidProtocol);
        }

        let protocol = match protocol_level {
            4 => Protocol::MQTT(4),
            num => return Err(Error::InvalidProtocolLevel(num)),
        };

        let connect_flags = bytes.get_u8();
        let keep_alive = bytes.get_u16();
        let clean_session = (connect_flags & 0b10) != 0;
        let client_id = extract_mqtt_string(&mut bytes)?;
        let last_will = extract_last_will(connect_flags, &mut bytes)?;
        let (username, password) = extract_username_password(connect_flags, &mut bytes)?;

        let _props = extract_properties(&mut bytes)?;

        let connect = match _props {
            Some(props) => {
                let properties = Some(
                    ConnectProperties{
                        session_expiry_interval: props.session_expiry_interval,
                        receive_maximum: props.receive_maximum,
                        maximum_packet_size: props.maximum_packet_size,
                        topic_alias_maximum: props.topic_alias_maximum,
                        request_response_information: props.request_response_info,
                        request_problem_information: props.request_problem_info,
                        user_property: props.user_property,
                        authentication_methods: props.authentication_method,
                        authentication_data: props.authentication_data
                    }
                );
                Connect {
                    protocol,
                    keep_alive,
                    client_id,
                    clean_session,
                    last_will,
                    username,
                    password,
                    properties
                }
            }
            None => Connect {
                protocol,
                keep_alive,
                client_id,
                clean_session,
                last_will,
                username,
                password,
                properties: None
            }
        };

        Ok(connect)
    }
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Protocol = {:?}, Keep alive = {:?}, Client id = {}, Clean session = {}",
            self.protocol, self.keep_alive, self.client_id, self.clean_session,
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
        let mut stream = bytes::BytesMut::new();
        let packet_stream = &[
            0x10,
            39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x04,        // variable header
            0b1100_1110, // variable header. +username, +password, -will retain, will qos=1, +last_will, +clean_session
            0x00,
            0x0a, // variable header. keep alive = 10 sec
            0x00,
            0x04,
            b't',
            b'e',
            b's',
            b't', // payload. client_id
            0x00,
            0x02,
            b'/',
            b'a', // payload. will topic = '/a'
            0x00,
            0x07,
            b'o',
            b'f',
            b'f',
            b'l',
            b'i',
            b'n',
            b'e', // payload. variable header. will msg = 'offline'
            0x00,
            0x04,
            b'r',
            b'u',
            b'm',
            b'q', // payload. username = 'rumq'
            0x00,
            0x02,
            b'm',
            b'q', // payload. password = 'mq'
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];

        stream.extend_from_slice(&packet_stream[..]);
        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Connect(connect) => connect,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            Connect {
                protocol: Protocol::MQTT(4),
                keep_alive: 10,
                client_id: "test".to_owned(),
                clean_session: true,
                last_will: Some(LastWill {
                    topic: "/a".to_owned(),
                    message: "offline".to_owned(),
                    retain: false,
                    qos: QoS::AtLeastOnce,
                }),
                username: Some("rumq".to_owned()),
                password: Some("mq".to_owned()),
                properties: None
            }
        );
    }
}
