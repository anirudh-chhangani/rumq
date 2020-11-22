use crate::{Error, Utf8Pair};
use crate::FixedHeader;
use bytes::{Buf, Bytes};
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct PubRelProperties {
    pub reason_string: Option<String>,
    pub user_property: Option<Utf8Pair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PubRel {
    pub pkid: u16,
    pub properties: Option<PubRelProperties>,
}

impl PubRel {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();
        let _props = extract_properties(&mut bytes)?;
        let pubrel = match _props {
            Some(props) => {
                let properties = Some(
                    PubRelProperties {
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                    }
                );
                PubRel { pkid, properties }
            }
            None => PubRel { pkid, properties: None }
        };


        Ok(pubrel)
    }
}

impl PubRel {
    pub fn new(pkid: u16, properties: Option<PubRelProperties>) -> PubRel {
        PubRel { pkid, properties }
    }
}