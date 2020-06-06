use crate::{extract_mqtt_string, Error, FixedHeader};
use alloc::string::String;
use alloc::vec::Vec;
use bytes::{Buf, Bytes};
use crate::control::properties::extract_properties;

#[derive(Debug, Clone, PartialEq)]
pub struct UnsubscribeProperties {
    pub user_property: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unsubscribe {
    pub pkid: u16,
    pub topics: Vec<String>,
    pub properties: Option<UnsubscribeProperties>,
}

impl Unsubscribe {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);
        let pkid = bytes.get_u16();
        let mut payload_bytes = fixed_header.remaining_len - 2;
        let mut topics = Vec::with_capacity(1);

        while payload_bytes > 0 {
            let topic_filter = extract_mqtt_string(&mut bytes)?;
            payload_bytes -= topic_filter.len() + 2;
            topics.push(topic_filter);
        }

        let _props = extract_properties(&mut bytes)?;
        let unsubscribe= match _props {
            Some(props)=>{
                let properties = Some(
                    UnsubscribeProperties{
                        user_property: props.user_property
                    }
                );
                Unsubscribe { pkid, topics, properties }
            }
            None => Unsubscribe { pkid, topics, properties: None }
        };

        Ok(unsubscribe)
    }
}
