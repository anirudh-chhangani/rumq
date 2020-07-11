use crate::{Error, Utf8Pair};
use crate::FixedHeader;
use bytes::{Buf, Bytes};
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct PubAckProperties {
    pub reason_string: Option<String>,
    pub user_property: Option<Utf8Pair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PubAck {
    pub pkid: u16,
    pub properties: Option<PubAckProperties>
}

impl PubAck {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();

        let _props = extract_properties(&mut bytes)?;
        let puback = match _props {
            Some(props)=>{
                let properties = Some(
                    PubAckProperties{
                        reason_string: props.reason_string,
                        user_property: props.user_property
                    }
                );
                PubAck { pkid, properties }
            }
            None => PubAck { pkid, properties: None }
        };

        Ok(puback)
    }
}

impl PubAck {
    pub fn new(pkid: u16, properties: Option<PubAckProperties>) -> PubAck {
        PubAck { pkid, properties }
    }
}

#[cfg(test)]
mod test_publish {
    use crate::*;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use bytes::{Bytes};
    use pretty_assertions::assert_eq;

    #[test]
    fn puback_stitching_works_correctly() {
        let stream = &[
            0b0100_0000,
            0x02, // packet type, flags and remaining len
            0x00,
            0x0A, // fixed header. packet identifier = 10
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];
        let mut stream = Bytes::from(&stream[..]);

        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::PubAck(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(packet, PubAck { pkid: 10, properties: None });
    }
}
