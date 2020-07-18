use crate::control::*;
use crate::enums::Packet;
use crate::{packet_type, Error, FixedHeader, PacketType};
use bytes::{Buf, Bytes, BytesMut};

pub fn mqtt_read(stream: &mut BytesMut, max_payload_size: usize) -> Result<Packet, Error> {
    // Read the initial bytes necessary from the stream with out mutating the stream cursor
    let (byte1, remaining_len) = parse_fixed_header(stream)?;
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

fn parse_fixed_header(stream: &mut BytesMut) -> Result<(u8, usize), Error> {
    if stream.is_empty() {
        return Err(Error::UnexpectedEof);
    }
    let byte1 = stream.get_u8();
    let (_len, _) = decode_variable_byte(&mut stream.to_bytes());
    let len = _len?;
    Ok((byte1, len as usize))
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

        let (_, remaining_len) = parse_fixed_header(&mut BytesMut::from("\x10\x00")).unwrap();
        assert_eq!(remaining_len, 0);

        let (_, remaining_len) = parse_fixed_header(&mut BytesMut::from("\x10\x7f")).unwrap();
        assert_eq!(remaining_len, 127);

        let data = &mut BytesMut::new();

        data.extend_from_slice(b"\x10\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 128);
        data.clear();

        data.extend_from_slice(b"\x10\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 16383);
        data.clear();

        data.extend_from_slice(b"\x10\x80\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 16384);
        data.clear();

        data.extend_from_slice(b"\x10\xff\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 2_097_151);
        data.clear();

        data.extend_from_slice(b"\x10\x80\x80\x80\x01");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 2_097_152);
        data.clear();

        data.extend_from_slice(b"\x10\xff\xff\xff\x7f");
        let (_, remaining_len) = parse_fixed_header(data).unwrap();
        assert_eq!(remaining_len, 268_435_455);
        data.clear();
    }
    //
    // #[test]
    // fn read_packet_connect_mqtt_protocol() {
    //     let mut stream = bytes::Bytes::new();
    //     let packetstream = vec![
    //         0x10,
    //         39, // packet type, flags and remaining len
    //         0x00,
    //         0x04,
    //         b'M',
    //         b'Q',
    //         b'T',
    //         b'T',
    //         0x04,        // variable header
    //         0b1100_1110, // variable header. +username, +password, -will retain, will qos=1, +last_will, +clean_session
    //         0x00,
    //         0x0a, // variable header. keep alive = 10 sec
    //         0x00,
    //         0x04,
    //         b't',
    //         b'e',
    //         b's',
    //         b't', // payload. client_id
    //         0x00,
    //         0x02,
    //         b'/',
    //         b'a', // payload. will topic = '/a'
    //         0x00,
    //         0x07,
    //         b'o',
    //         b'f',
    //         b'f',
    //         b'l',
    //         b'i',
    //         b'n',
    //         b'e', // payload. variable header. will msg = 'offline'
    //         0x00,
    //         0x04,
    //         b'r',
    //         b'u',
    //         b'm',
    //         b'q', // payload. username = 'rumq'
    //         0x00,
    //         0x02,
    //         b'm',
    //         b'q', // payload. password = 'mq'
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     // Reads till the end of connect packet leaving the extra bytes
    //     stream.extend_from_slice(&packetstream);
    //
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let _connect = match packet {
    //         Packet::Connect(connect) => connect,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    // }

    // #[test]
    // fn read_packet_connack_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let packetstream = vec![
    //         0b0010_0000,
    //         0x02, // packet type, flags and remaining len
    //         0x01,
    //         0x00, // variable header. connack flags, connect return code
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let _packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    // }
    //
    // #[test]
    // fn read_packet_publish_qos1_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let mut packetstream = vec![
    //         0b0011_0010,
    //         11, // packet type, flags and remaining len
    //         0x00,
    //         0x03,
    //         b'a',
    //         b'/',
    //         b'b', // variable header. topic name = 'a/b'
    //         0x00,
    //         0x0a, // variable header. pkid = 10
    //         0xF1,
    //         0xF2,
    //         0xF3,
    //         0xF4, // publish payload
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let packet = match packet {
    //         Packet::Publish(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    //
    //     assert_eq!(packet.bytes.len(), packetstream.len() - 4);
    //     // remove extra control from source packet stream and compare
    //     packetstream.truncate(packetstream.len() - 4);
    //     assert_eq!(&packet.bytes[..], &packetstream[..]);
    // }
    //
    // #[test]
    // fn read_packet_publish_qos0_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let mut packetstream = vec![
    //         0b0011_0000,
    //         7, // packet type, flags and remaining len
    //         0x00,
    //         0x03,
    //         b'a',
    //         b'/',
    //         b'b', // variable header. topic name = 'a/b'
    //         0x01,
    //         0x02, // payload
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let packet = match packet {
    //         Packet::Publish(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    //
    //     assert_eq!(packet.bytes.len(), packetstream.len() - 4);
    //     // remove extra control from source packet stream and compare
    //     packetstream.truncate(packetstream.len() - 4);
    //     assert_eq!(&packet.bytes[..], &packetstream[..]);
    // }
    //
    // #[test]
    // fn read_packet_puback_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let packetstream = vec![
    //         0b0100_0000,
    //         0x02, // packet type, flags and remaining len
    //         0x00,
    //         0x0A, // fixed header. packet identifier = 10
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let _packet = match packet {
    //         Packet::PubAck(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    // }
    //
    // #[test]
    // fn read_packet_subscribe_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let packetstream = vec![
    //         0b1000_0010,
    //         20, // packet type, flags and remaining len
    //         0x01,
    //         0x04, // variable header. pkid = 260
    //         0x00,
    //         0x03,
    //         b'a',
    //         b'/',
    //         b'+', // payload. topic filter = 'a/+'
    //         0x00, // payload. qos = 0
    //         0x00,
    //         0x01,
    //         b'#', // payload. topic filter = '#'
    //         0x01, // payload. qos = 1
    //         0x00,
    //         0x05,
    //         b'a',
    //         b'/',
    //         b'b',
    //         b'/',
    //         b'c', // payload. topic filter = 'a/b/c'
    //         0x02, // payload. qos = 2
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let _packet = match packet {
    //         Packet::Subscribe(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    // }

    // #[test]
    // fn read_packet_suback_works() {
    //     let mut stream = bytes::BytesMut::new();
    //     let packetstream = vec![
    //         0x90, 4, // packet type, flags and remaining len
    //         0x00, 0x0F, // variable header. pkid = 15
    //         0x01, 0x80, // payload. return codes [success qos1, failure]
    //         0xDE, 0xAD, 0xBE, 0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     assert_eq!(&stream[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    //
    //     let _packet = match packet {
    //         Packet::SubAck(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    // }
    //
    // #[test]
    // fn incomplete_qos1_publish_stream_errors_until_there_are_enough_bytes() {
    //     let mut stream = bytes::BytesMut::new();
    //     let packetstream = vec![
    //         0b0011_0010,
    //         11, // packet type, flags and remaining len
    //         0x00,
    //         0x03,
    //         b'a',
    //         b'/',
    //         b'b', // variable header. topic name = 'a/b'
    //         0x00,
    //         0x0a, // variable header. pkid = 10
    //         0xF1,
    //         0xF2,
    //         0xF3,
    //         0xF4, // publish payload
    //         0xDE,
    //         0xAD,
    //         0xBE,
    //         0xEF, // extra control in the stream
    //     ];
    //
    //     stream.extend_from_slice(&packetstream);
    //
    //     // new byte stream which is a part of complete stream above
    //     let mut s = stream.clone();
    //     s.truncate(0);
    //     match mqtt_read(&mut s.split_off(0), 100) {
    //         Ok(_) => panic!("should've panicked as there aren't enough bytes"),
    //         Err(Error::UnexpectedEof) => (),
    //         Err(e) => panic!("Expecting EoF error. Received = {:?}", e),
    //     };
    //
    //     // new byte stream which is a part of complete stream above
    //     let mut s = stream.clone();
    //     s.truncate(2);
    //     match mqtt_read(&mut s.split_off(0), 100) {
    //         Ok(_) => panic!("should've panicked as there aren't enough bytes"),
    //         Err(Error::UnexpectedEof) => (),
    //         Err(e) => panic!("Expecting EoF error. Received = {:?}", e),
    //     };
    //
    //     // new byte stream which is a part of complete stream above
    //     let mut s = stream.clone();
    //     s.truncate(4);
    //     match mqtt_read(&mut s.split_off(0), 100) {
    //         Ok(_) => panic!("should've panicked as there aren't enough bytes"),
    //         Err(Error::UnexpectedEof) => (),
    //         Err(e) => panic!("Expecting EoF error. Received = {:?}", e),
    //     };
    //
    //     let packet = mqtt_read(&mut stream, 100).unwrap();
    //     match packet {
    //         Packet::Publish(packet) => packet,
    //         packet => panic!("Invalid packet = {:?}", packet),
    //     };
    // }

    // TODO Create a publish stream which takes 2, 3, 4 bytes remaining length
}
