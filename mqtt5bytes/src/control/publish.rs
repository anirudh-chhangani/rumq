use crate::{extract_mqtt_string, qos, Error, FixedHeader, QoS, Utf8Pair};
use alloc::string::String;
use alloc::vec::Vec;
use bytes::{Buf, Bytes};
use core::fmt;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct PublishProperties {
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub topic_alias: Option<u16>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<String>,
    pub user_property: Option<Utf8Pair>,
    pub subscription_identifier: Option<u32>,
    pub content_type: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct Publish {
    pub qos: QoS,
    pub pkid: u16,
    pub topic: String,
    pub payload: Bytes,
    pub dup: bool,
    pub retain: bool,
    pub bytes: Bytes,
    pub properties: Option<PublishProperties>,
}

impl Publish {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let mut payload = bytes.clone();
        let qos = qos((fixed_header.byte1 & 0b0110) >> 1)?;
        let dup = (fixed_header.byte1 & 0b1000) != 0;
        let retain = (fixed_header.byte1 & 0b0001) != 0;

        let variable_header_index = fixed_header.header_len;
        payload.advance(variable_header_index);
        let topic = extract_mqtt_string(&mut payload)?;

        // Packet identifier exists where QoS > 0
        let pkid = match qos {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce | QoS::ExactlyOnce => payload.get_u16(),
        };

        if qos != QoS::AtMostOnce && pkid == 0 {
            return Err(Error::PacketIdZero);
        }

        let _props = extract_properties(&mut bytes)?;

        let publish = match _props {
            Some(props) => {
                let properties = Some(
                    PublishProperties {
                        payload_format_indicator: props.payload_format_indicator,
                        message_expiry_interval: props.message_expiry_interval,
                        topic_alias: props.topic_alias,
                        response_topic: props.response_topic,
                        correlation_data: props.correlation_data,
                        user_property: props.user_property,
                        subscription_identifier: props.subscription_identifier,
                        content_type: props.content_type,
                    }
                );

                Publish {
                    qos,
                    pkid,
                    topic,
                    payload,
                    dup,
                    retain,
                    bytes,
                    properties,
                }
            }
            None => Publish {
                qos,
                pkid,
                topic,
                payload,
                dup,
                retain,
                bytes,
                properties: None,
            }
        };

        Ok(publish)
    }
}

impl Publish {
    // TODO Take AsRef slice instead?
    pub fn new<S: Into<String>, P: Into<Vec<u8>>>(topic: S, qos: QoS, payload: P) -> Publish {
        Publish {
            dup: false,
            qos,
            retain: false,
            pkid: 0,
            topic: topic.into(),
            payload: bytes::Bytes::from(payload.into()),
            bytes: Bytes::new(),
            properties: None,
        }
    }

    pub fn set_pkid(&mut self, pkid: u16) -> &mut Self {
        self.pkid = pkid;
        self
    }
}

impl fmt::Debug for Publish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Topic = {}, Qos = {:?}, Retain = {}, Pkid = {:?}, Payload Size = {}",
            self.topic,
            self.qos,
            self.retain,
            self.pkid,
            self.payload.len()
        )
    }
}

#[cfg(test)]
mod test_publish {
    use crate::*;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;

    #[test]
    fn qos1_publish_stitching_works_correctly() {
        let stream = &[
            0b0011_0010,
            11, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x00,
            0x0a, // variable header. pkid = 10
            0xF1,
            0xF2,
            0xF3,
            0xF4, // publish payload
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];

        let bytes = &[
            0b0011_0010,
            11, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x00,
            0x0a, // variable header. pkid = 10
            0xF1,
            0xF2,
            0xF3,
            0xF4, // publish payload
        ];

        let mut stream = BytesMut::from(&stream[..]);
        let bytes = Bytes::from(&bytes[..]);

        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Publish(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        let payload = &[0xF1, 0xF2, 0xF3, 0xF4];
        assert_eq!(
            packet,
            Publish {
                dup: false,
                qos: QoS::AtLeastOnce,
                retain: false,
                topic: "a/b".to_owned(),
                pkid: 10,
                payload: Bytes::from(&payload[..]),
                bytes,
                properties: None,
            }
        );
    }

    #[test]
    fn qos0_publish_stitching_works_correctly() {
        let stream = &[
            0b0011_0000,
            7, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x01,
            0x02, // payload
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];
        let bytes = &[
            0b0011_0000,
            7, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x01,
            0x02, // payloa/home/tekjar/.local/share/JetBrains/Toolbox/bin/cliond
        ];
        let mut stream = BytesMut::from(&stream[..]);
        let bytes = Bytes::from(&bytes[..]);

        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Publish(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            Publish {
                dup: false,
                qos: QoS::AtMostOnce,
                retain: false,
                topic: "a/b".to_owned(),
                pkid: 0,
                payload: Bytes::from(&[0x01, 0x02][..]),
                bytes,
                properties: None,
            }
        );
    }
}
