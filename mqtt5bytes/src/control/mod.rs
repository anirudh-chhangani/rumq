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
pub use self::auth::Auth;
pub use self::connack::ConnAck;
pub use self::connect::Connect;
pub use self::disconnect::Disconnect;
pub use self::puback::PubAck;
pub use self::pubcomp::PubComp;
pub use self::publish::Publish;
pub use self::pubrec::PubRec;
pub use self::pubrel::PubRel;
pub use self::suback::SubAck;
pub use self::subscribe::Subscribe;
pub use self::unsuback::UnsubAck;
pub use self::unsubscribe::Unsubscribe;

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
