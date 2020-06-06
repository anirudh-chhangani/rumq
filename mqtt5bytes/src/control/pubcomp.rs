use crate::Error;
use crate::FixedHeader;
use bytes::{Buf, Bytes};
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct PubCompProperties {
    pub reason_string: Option<String>,
    pub user_property: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PubComp {
    pub pkid: u16,
    pub properties: Option<PubCompProperties>
}

impl PubComp {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();
        let _props = extract_properties(&mut bytes)?;
        let pubcomp = match _props {
            Some(props) => {
                let properties = Some(
                    PubCompProperties {
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                    }
                );
                PubComp { pkid, properties }
            }
            None => PubComp { pkid, properties: None }
        };


        Ok(pubcomp)
    }
}

impl PubComp {
    pub fn new(pkid: u16, properties: Option<PubCompProperties>) -> PubComp {
        PubComp { pkid, properties }
    }
}
