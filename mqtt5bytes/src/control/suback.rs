use crate::{qos, Error, FixedHeader, SubscribeReturnCodes, Utf8Pair};
use alloc::vec::Vec;
use bytes::{Buf, Bytes};
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct SubAckProperties {
    pub reason_string: Option<String>,
    pub user_property: Option<Utf8Pair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubAck {
    pub pkid: u16,
    pub return_codes: Vec<SubscribeReturnCodes>,
    properties: Option<SubAckProperties>,
}

impl SubAck {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);

        let pkid = bytes.get_u16();
        let mut payload_bytes = fixed_header.remaining_len - 2;
        let mut return_codes = Vec::with_capacity(payload_bytes);

        while payload_bytes > 0 {
            let return_code = bytes.get_u8();
            if return_code >> 7 == 1 {
                return_codes.push(SubscribeReturnCodes::Failure)
            } else {
                return_codes.push(SubscribeReturnCodes::Success(qos(return_code & 0x3)?));
            }
            payload_bytes -= 1
        }

        let _props = extract_properties(&mut bytes)?;
        let suback = match _props {
            Some(props) => {
                let properties = Some(
                    SubAckProperties {
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                    }
                );
                SubAck { pkid, return_codes, properties }
            }
            None => SubAck { pkid, return_codes, properties: None }
        };

        Ok(suback)
    }
}

impl SubAck {
    pub fn new(pkid: u16, return_codes: Vec<SubscribeReturnCodes>, properties: Option<SubAckProperties>) -> SubAck {
        SubAck { pkid, return_codes, properties }
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
    fn suback_stitching_works_correctly() {
        let stream = vec![
            0x90, 4, // packet type, flags and remaining len
            0x00, 0x0F, // variable header. pkid = 15
            0x01, 0x80, // payload. return codes [success qos1, failure]
            0xDE, 0xAD, 0xBE, 0xEF, // extra control in the stream
        ];
        let mut stream = BytesMut::from(&stream[..]);

        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::SubAck(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            SubAck {
                pkid: 15,
                return_codes: vec![SubscribeReturnCodes::Success(QoS::AtLeastOnce), SubscribeReturnCodes::Failure],
                properties: None,
            }
        );
    }
}
