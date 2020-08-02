use crate::control::*;
use crate::enums::Packet;
use crate::{packet_type, Error, FixedHeader, PacketType};
use bytes::{Buf, Bytes, BytesMut};

pub fn mqtt_read(stream: &mut BytesMut, max_payload_size: usize) -> Result<Packet, Error> {
    // Read the initial bytes necessary from the stream with out mutating the stream cursor
    let (byte1, remaining_len) = parse_fixed_header(stream.clone())?;
    let header_len = header_len(remaining_len);
    let len = header_len + remaining_len;


    // Don't let rogue connections attack with huge payloads. Disconnect them before reading all
    // that data
    if remaining_len > max_payload_size {
        return Err(Error::PayloadSizeLimitExceeded);
    }

    // If the current call fails due to insufficient bytes in the stream, after calculating
    // remaining length, we extend the stream
    if stream.len() < len {
        stream.reserve(remaining_len + 2);
        return Err(Error::UnexpectedEof);
    }

    // Test with a stream with exactly the size to check border panics
    let mut packet = stream.split_to(len);
    let control_type = packet_type(byte1 >> 4)?;

    if remaining_len == 0 {
        // no payload control
        return match control_type {
            PacketType::PingReq => Ok(Packet::PingReq),
            PacketType::PingResp => Ok(Packet::PingResp),
            _ => Err(Error::PayloadRequired),
        };
    }

    let fixed_header = FixedHeader {
        byte1,
        header_len,
        remaining_len,
    };

    // Always reserve size for next max possible payload
    if stream.len() < 2 {
        stream.reserve(6 + max_payload_size)
    }

    let mut packet = match control_type {
        PacketType::Connect => Packet::Connect(Connect::assemble(fixed_header, packet.to_bytes())?),
        PacketType::ConnAck => Packet::ConnAck(ConnAck::assemble(fixed_header, packet.to_bytes())?),
        PacketType::Publish => Packet::Publish(Publish::assemble(fixed_header, packet.to_bytes())?),
        PacketType::PubAck => Packet::PubAck(PubAck::assemble(fixed_header, packet.to_bytes())?),
        PacketType::PubRec => Packet::PubRec(PubRec::assemble(fixed_header, packet.to_bytes())?),
        PacketType::PubRel => Packet::PubRel(PubRel::assemble(fixed_header, packet.to_bytes())?),
        PacketType::PubComp => Packet::PubComp(PubComp::assemble(fixed_header, packet.to_bytes())?),
        PacketType::Subscribe => Packet::Subscribe(Subscribe::assemble(fixed_header, packet.to_bytes())?),
        PacketType::SubAck => Packet::SubAck(SubAck::assemble(fixed_header, packet.to_bytes())?),
        PacketType::Unsubscribe => Packet::Unsubscribe(Unsubscribe::assemble(fixed_header, packet.to_bytes())?),
        PacketType::UnsubAck => Packet::UnsubAck(UnsubAck::assemble(fixed_header, packet.to_bytes())?),
        PacketType::PingReq => Packet::PingReq,
        PacketType::PingResp => Packet::PingResp,
        PacketType::Disconnect => Packet::Disconnect(Disconnect::assemble(fixed_header, packet.to_bytes())?),
        PacketType::Auth => Packet::Auth(Auth::assemble(fixed_header, packet.to_bytes())?),
    };

    Ok(packet)
}

fn parse_fixed_header(mut stream: BytesMut) -> Result<(u8, usize), Error> {
    if stream.is_empty() {
        return Err(Error::UnexpectedEof);
    }

    let byte1 = stream.get_u8();
    let (rl, _) = decode_variable_byte(
        &mut Bytes::from(stream.to_bytes())
    )?;
    Ok((byte1, rl as usize))
}

fn header_len(remaining_len: usize) -> usize {
    if remaining_len >= 2_097_152 {
        4 + 1
    } else if remaining_len >= 16_384 {
        3 + 1
    } else if remaining_len >= 128 {
        2 + 1
    } else {
        1 + 1
    }
}

#[cfg(test)]
mod test {
    use super::{mqtt_read, parse_fixed_header};
    use crate::{Error, Packet};
    use alloc::vec;
    use pretty_assertions::assert_eq;
    use bytes::{BytesMut, Bytes};

    #[test]
    fn fixed_header_is_parsed_as_expected() {
        let mut data = BytesMut::new();

        let (_, remaining_len) = parse_fixed_header(BytesMut::from("\x10\x00")).unwrap();
        assert_eq!(remaining_len, 0);
        data.clear();

        let (_, remaining_len) = parse_fixed_header(BytesMut::from("\x10\x7f")).unwrap();
        assert_eq!(remaining_len, 127);
        data.clear();

        data.extend_from_slice(b"\x10\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 128);
        data.clear();

        data.extend_from_slice(b"\x10\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 16383);
        data.clear();

        data.extend_from_slice(b"\x10\x80\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 16384);
        data.clear();

        data.extend_from_slice(b"\x10\xff\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 2_097_151);
        data.clear();

        data.extend_from_slice(b"\x10\x80\x80\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 2_097_152);
        data.clear();

        data.extend_from_slice(b"\x10\xff\xff\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data.clone()).unwrap();
        assert_eq!(remaining_len, 268_435_455);
        data.clear();
    }
}
