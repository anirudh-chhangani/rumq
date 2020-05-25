#![no_std]

use cfg_if::cfg_if;

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
mod codec;
mod control;
mod enums;
mod err;
mod header;
mod read;
mod reasoncodes;
mod topic;
mod will;
mod write;

use alloc::string::String;
use bytes::{Buf, Bytes};
#[cfg(feature = "std")]
pub use codec::*;
pub use control::*;
pub use enums::*;
pub use err::*;
pub use header::*;
pub use read::*;
pub use topic::*;
pub use will::*;
pub use write::*;

///          7                          3                          0
///          +--------------------------+--------------------------+
/// byte 1   | MQTT Control Packet Type | Flags for each type      |
///          +--------------------------+--------------------------+
///          |         Remaining Bytes Len  (1 - 4 bytes)          |
///          +-----------------------------------------------------+
///
/// https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021

pub fn packet_type(num: u8) -> Result<PacketType, Error> {
    match num {
        1 => Ok(PacketType::Connect),
        2 => Ok(PacketType::ConnAck),
        3 => Ok(PacketType::Publish),
        4 => Ok(PacketType::PubAck),
        5 => Ok(PacketType::PubRec),
        6 => Ok(PacketType::PubRel),
        7 => Ok(PacketType::PubComp),
        8 => Ok(PacketType::Subscribe),
        9 => Ok(PacketType::SubAck),
        10 => Ok(PacketType::Unsubscribe),
        11 => Ok(PacketType::UnsubAck),
        12 => Ok(PacketType::PingReq),
        13 => Ok(PacketType::PingResp),
        14 => Ok(PacketType::Disconnect),
        15 => Ok(PacketType::Auth),
        _ => Err(Error::InvalidPacketType(num)),
    }
}
