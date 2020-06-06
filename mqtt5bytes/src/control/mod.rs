mod auth;
mod connack;
mod connect;
mod disconnect;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;
mod properties;

// Re-Exports
pub use self::auth::*;
pub use self::connack::*;
pub use self::connect::*;
pub use self::disconnect::*;
pub use self::puback::*;
pub use self::pubcomp::*;
pub use self::publish::*;
pub use self::pubrec::*;
pub use self::pubrel::*;
pub use self::suback::*;
pub use self::subscribe::*;
pub use self::unsuback::*;
pub use self::unsubscribe::*;

use crate::{Error, QoS};
use alloc::string::String;
use bytes::{Buf, Bytes};

pub(crate) fn qos(num: u8) -> Result<QoS, Error> {
    match num {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        qos => Err(Error::InvalidQoS(qos)),
    }
}

// extract methods
pub(crate) fn extract_mqtt_string(stream: &mut Bytes) -> Result<String, Error> {
    let len = stream.get_u16() as usize;
    // Invalid control which reached this point (simulated invalid control actually triggered this)
    // should not cause the split to cross boundaries
    if len > stream.len() {
        return Err(Error::BoundaryCrossed);
    }

    let s = stream.split_to(len);
    match String::from_utf8(s.to_vec()) {
        Ok(v) => Ok(v),
        Err(_e) => Err(Error::TopicNotUtf8),
    }
}

/// Byte length for deterministic types defined in v5.0 spec
pub(crate) struct ByteLengths;

impl ByteLengths {
    pub const BYTE_INT: u8 = 1;
    pub const TWO_BYTE_INT: u8 = 2;
    pub const FOUR_BYTE_INT: u8 = 4;
}

/// decode variable byte integer defined in MQTT v5.0 spec
/// returns the decoded variable byte integer and the length of bytes decoded
pub(crate) fn decode_variable_byte(stream: &mut Bytes) -> (Result<i32, Error>, u8) {
    (Ok(0), 0)
}

/// encode variable byte integer defined in MQTT v5.0 spec
/// returns the encoded bytes and length of bytes encoded.
pub(crate) fn encode_variable_byte(value: i32) -> (Result<Bytes, Error>, u8) {
    (Ok(Bytes::from("wip")), 0)
}

/// decode utf-8 string defined in MQTT v5.0 spec
/// returns the decoded utf-8 string and the length of bytes decoded
pub(crate) fn decode_utf_string(stream: &mut Bytes) -> (Result<String, Error>, u8) {
    (Ok(String::from("wip")), 0)
}

/// encode utf-8 string defined in MQTT v5.0 spec
/// returns the encoded utf-8 string and length of bytes encoded.
pub(crate) fn encode_utf_string(value: String) -> (Result<Bytes, Error>, u8) {
    (Ok(Bytes::from("wip")), 0)
}
