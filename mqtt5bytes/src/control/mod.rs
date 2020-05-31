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
