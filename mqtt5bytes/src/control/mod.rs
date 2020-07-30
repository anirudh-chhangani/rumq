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
use bytes::{Buf, Bytes, BytesMut};
use alloc::vec::Vec;

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

pub(crate) type Utf8Pair = (String, String);

/// Byte length for deterministic types defined in v5.0 spec
pub(crate) struct ByteLengths;

impl ByteLengths {
    pub const BYTE_INT: u32 = 1;
    pub const TWO_BYTE_INT: u32 = 2;
    pub const FOUR_BYTE_INT: u32 = 4;
}

/// Get the value of a multi-byte integer from a buffer
/// Return the value, and the number of bytes used.
/// [MQTT-1.5.5-1] the encoded value MUST use the minimum number of bytes necessary to represent the value
pub(crate) fn decode_variable_byte(stream: &mut Bytes) -> (Result<u32, Error>, u32) {
    let mut multiplier: u32 = 1;
    let mut value: u32 = 0;
    let mut byte_len = 0;
    let mut encoded_byte = 128;
    while (encoded_byte & 128) != 0 {
        encoded_byte = stream.get_u8() as u32;
        byte_len += ByteLengths::BYTE_INT;
        value += (encoded_byte & 127) * multiplier;
        if multiplier > 128*128*128 {
            return (Err(Error::MalformedVariableByteInteger), byte_len);
        }
        multiplier *= 128;
    }
    return (Ok(value), byte_len);
}

/// encode variable byte integer defined in MQTT v5.0 spec
/// returns the encoded bytes and length of bytes encoded.
/// Convert an integer 0 <= x <= 268435455 into multi-byte format.
///  returns the buffer converted from the integer.
pub(crate) fn encode_variable_byte(mut value: i32) -> Result<Bytes, Error> {
    let mut encoded_byte = 0;
    let mut buf = BytesMut::new();
    while value > 0 {
        encoded_byte = value % 128;
        value /= 128;
        if value > 0 {
            encoded_byte |= 128;
        }
        buf.extend_from_slice(
            encoded_byte.to_ne_bytes().as_ref()
        );
    }
    return Ok(Bytes::from(buf));
}

/// decode utf-8 string defined in MQTT v5.0 spec
/// returns the decoded utf-8 string and the length of bytes decoded
pub(crate) fn decode_utf_string(stream: &mut Bytes) -> (Result<String, Error>, u32) {
    let mut strlen = stream.get_u16();
    let bytelen = ByteLengths::TWO_BYTE_INT + strlen as u32; // as two bytes have been read above for strlen
    let mut data: Vec<u8> = Vec::new();
    while strlen != 0 {
        data.extend_from_slice(&[stream.get_u8()]);
    }
    let decoded = String::from_utf8(data);
    match decoded {
        Ok(val) => {
            (Ok(val), bytelen)
        }
        Err(utf_err) => {
            (Err(Error::UnexpectedEof), bytelen)
        }
    }
}

/// encode utf-8 string defined in MQTT v5.0 spec
/// returns the encoded utf-8 string as bytes that can be added to MQTT stream.
pub(crate) fn encode_utf_string(value: String) -> Result<Bytes, Error> {
    let mut buf = BytesMut::from("");
    let bts = value.as_bytes();
    let blen = bts.len().to_ne_bytes();
    buf.extend_from_slice(blen.as_ref()); // 2 byte len + characters
    buf.extend_from_slice(bts.as_ref());
    return Ok(buf.to_bytes());
}

/// decode utf-8 string pair defined in MQTT v5.0 spec
/// returns the decoded utf-8 string pair and the length of bytes decoded
pub(crate) fn decode_utf_string_pair(stream: &mut Bytes) -> (Result<Utf8Pair, Error>, u32) {
    let (K, l1) = decode_utf_string(stream);
    let (V, l2) = decode_utf_string(stream);
    let (k, v) = (K.unwrap(), V.unwrap());
    (Ok((k, v)), l1 + l2)
}

/// encode utf-8 string pair defined in MQTT v5.0 spec
/// returns the encoded utf-8 string pair that can be added to mqtt stream.
pub(crate) fn encode_utf_string_pair(value: Utf8Pair) -> Result<Bytes, Error> {
    let (k, v) = value;
    let mut data = BytesMut::new();
    let (klen, vlen) = (k.len() as i16, v.len() as i16);
    data.extend_from_slice(klen.to_ne_bytes().as_ref());
    data.extend_from_slice(k.as_bytes());
    data.extend_from_slice(vlen.to_ne_bytes().as_ref());
    data.extend_from_slice(v.as_bytes());
    Ok(data.to_bytes())
}

/// decode binary data defined in MQTT v5.0 spec
/// returns the decoded binary data and the length of bytes decoded
pub(crate) fn decode_binary_data(stream: &mut Bytes) -> (Result<Bytes, Error>, u32)
{
    let mut blen = stream.get_u16();
    let mut data = BytesMut::new();
    while blen != 0 {
        blen -= 1;
        data.extend_from_slice(stream.get_u8().to_ne_bytes().as_ref())
    }

    (Ok(data.to_bytes()), ByteLengths::TWO_BYTE_INT + data.len() as u32)
}

/// encode binary data defined in MQTT v5.0 spec
/// returns the binary data string and length of bytes encoded.
pub(crate) fn encode_binary_data(value: Bytes) -> Result<Bytes, Error> {
    let blen = value.len() as i16;
    let mut data = BytesMut::new();
    data.extend_from_slice(blen.to_ne_bytes().as_ref());
    data.extend_from_slice(value.as_ref());
    Ok(data.to_bytes())
}

#[cfg(test)]
mod test_decoding {
    use crate::{decode_variable_byte, Error, encode_variable_byte};
    use bytes::{Buf, Bytes};

    #[test]
    fn decode_variable_byte_int() {
        let vbi = &[
            0x7F,
            0x7F
        ];
        let mut stream = bytes::Bytes::from(&vbi[..]);
        let (res, len) = decode_variable_byte(&mut stream);
        assert_eq!(len, 1);
        match res {
            Ok(val) => assert_eq!(val, 127),
            _ => {}
        }
    }

    #[test]
    fn encode_variable_byte_int() {
        let res = encode_variable_byte(912);
        match res {
            Ok(raw) => {
                assert_eq!(raw, Bytes::from("lal"));
            }
            _ => {}
        }
    }
}