use crate::{Error, Utf8Pair};
use crate::FixedHeader;

use bytes::Bytes;
use crate::control::properties::extract_properties;
use alloc::string::String;

#[derive(Debug, Clone, PartialEq)]
pub struct DisconnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub reason_string: Option<String>,
    pub user_property: Option<Utf8Pair>,
    pub server_reference: Option<String>,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Disconnect {
    pub properties: Option<DisconnectProperties>
}

impl Disconnect {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let _props = extract_properties(&mut bytes)?;
        let disconnect = match _props {
            Some(props) => {
                let properties = Some(
                    DisconnectProperties {
                        session_expiry_interval: props.session_expiry_interval,
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                        server_reference: props.server_info,
                    }
                );
                Disconnect { properties }
            }
            None => Disconnect { properties: None }
        };
        Ok(disconnect)
    }
}
