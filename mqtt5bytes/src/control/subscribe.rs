use crate::{extract_mqtt_string, qos, Error, FixedHeader, QoS, SubscribeTopic};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bytes::{Buf, Bytes};
use core::fmt;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct SubscribeProperties {
    pub subscription_identifier: Option<u32>,
    pub user_property: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct Subscribe {
    pub pkid: u16,
    pub topics: Vec<SubscribeTopic>,
    pub properties: Option<SubscribeProperties>
}

impl Subscribe {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();

        // variable header size = 2 (packet identifier)
        let mut payload_bytes = fixed_header.remaining_len - 2;
        let mut topics = Vec::new();

        while payload_bytes > 0 {
            let topic_filter = extract_mqtt_string(&mut bytes)?;
            let requested_qos = bytes.get_u8();
            payload_bytes -= topic_filter.len() + 3;
            topics.push(SubscribeTopic {
                topic_path: topic_filter,
                qos: qos(requested_qos)?,
            });
        }
        let _props = extract_properties(&mut bytes)?;
        let subscribe = match _props {
            Some(props) => {
                let properties = Some(
                    SubscribeProperties {
                        subscription_identifier: props.subscription_identifier,
                        user_property: props.user_property,
                    }
                );
                Subscribe { pkid, topics, properties }
            }
            None => Subscribe { pkid, topics, properties: None }
        };

        Ok(subscribe)
    }
}

impl Subscribe {
    pub fn new<S: Into<String>>(topic: S, qos: QoS) -> Subscribe {
        let topic = SubscribeTopic {
            topic_path: topic.into(),
            qos,
        };

        Subscribe {
            pkid: 0,
            topics: vec![topic],
            properties: None
        }
    }

    pub fn empty_subscribe() -> Subscribe {
        Subscribe {
            pkid: 0,
            topics: Vec::new(),
            properties: None
        }
    }

    pub fn add(&mut self, topic: String, qos: QoS) -> &mut Self {
        let topic = SubscribeTopic { topic_path: topic, qos };
        self.topics.push(topic);
        self
    }
}

impl fmt::Debug for Subscribe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Filters = {:?}, Packet id = {:?}", self.pkid, self.topics)
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
    fn subscribe_stitching_works_correctly() {
        let stream = &[
            0b1000_0010,
            20, // packet type, flags and remaining len
            0x01,
            0x04, // variable header. pkid = 260
            0x00,
            0x03,
            b'a',
            b'/',
            b'+', // payload. topic filter = 'a/+'
            0x00, // payload. qos = 0
            0x00,
            0x01,
            b'#', // payload. topic filter = '#'
            0x01, // payload. qos = 1
            0x00,
            0x05,
            b'a',
            b'/',
            b'b',
            b'/',
            b'c', // payload. topic filter = 'a/b/c'
            0x02, // payload. qos = 2
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];
        let mut stream = BytesMut::from(&stream[..]);

        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::Subscribe(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            Subscribe {
                pkid: 260,
                topics: vec![
                    SubscribeTopic {
                        topic_path: "a/+".to_owned(),
                        qos: QoS::AtMostOnce,
                    },
                    SubscribeTopic {
                        topic_path: "#".to_owned(),
                        qos: QoS::AtLeastOnce,
                    },
                    SubscribeTopic {
                        topic_path: "a/b/c".to_owned(),
                        qos: QoS::ExactlyOnce,
                    }
                ],
                properties: None
            }
        );
    }
}
