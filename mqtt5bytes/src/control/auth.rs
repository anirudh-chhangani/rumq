use crate::Error;
use crate::FixedHeader;

use bytes::Bytes;
use alloc::string::String;
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct AuthProperties {
    pub authentication_method: Option<String>,
    pub authentication_data: Option<String>,
    pub reason_string: Option<String>,
    pub user_property: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Auth {
    pub properties: Option<AuthProperties>
}

impl Auth {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let _props = extract_properties(&mut bytes)?;
        let auth = match _props {
            Some(props) => {
                let properties = Some(
                    AuthProperties {
                        authentication_method: props.authentication_method,
                        authentication_data: props.authentication_data,
                        reason_string: props.reason_string,
                        user_property: props.user_property,
                    }
                );
                Auth { properties }
            }
            None => Auth { properties: None }
        };
        Ok(auth)
    }
}
