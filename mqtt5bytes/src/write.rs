use crate::enums::Packet;
use crate::Error;
use bytes::Bytes;

pub fn mqtt_write(packet: Packet) -> Result<Bytes, Error> {
    match packet {
        Packet::Connect(pkt) => pkt.disassemble(),
        _ => Err(Error::InvalidPacketType(0))
    }
}
