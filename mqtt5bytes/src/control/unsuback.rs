use crate::Error;
use crate::FixedHeader;
use bytes::{Buf, Bytes};
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct UnsubAckProperties {
    pub reason_string: Option<String>,
    pub user_property: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnsubAck {
    pub pkid: u16,
    pub properties: Option<UnsubAckProperties>,
}

impl UnsubAck {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();

        let _props = extract_properties(&mut bytes)?;
        let unsuback = match _props {
            Some(props) => {
                let properties = Some(
                    UnsubAckProperties {
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                    }
                );
                UnsubAck { pkid, properties }
            }
            None => UnsubAck { pkid, properties: None }
        };

        Ok(unsuback)
    }
}
