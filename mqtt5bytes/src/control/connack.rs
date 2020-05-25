use crate::{ConnectReturnCode, Error, FixedHeader};
use bytes::{Buf, Bytes};

#[derive(Debug, Clone, PartialEq)]
pub struct ConnAck {
    pub session_present: bool,
    pub code: ConnectReturnCode,
}

impl ConnAck {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        let get_return_code = |num: u8| -> Result<ConnectReturnCode, Error> {
            match num {
                0 => Ok(ConnectReturnCode::Accepted),
                1 => Ok(ConnectReturnCode::BadUsernamePassword),
                2 => Ok(ConnectReturnCode::NotAuthorized),
                3 => Ok(ConnectReturnCode::RefusedIdentifierRejected),
                4 => Ok(ConnectReturnCode::RefusedProtocolVersion),
                5 => Ok(ConnectReturnCode::ServerUnavailable),
                num => Err(Error::InvalidConnectReturnCode(num)),
            }
        };

        let variable_header_index = fixed_header.header_len;
        bytes.advance(variable_header_index);

        if fixed_header.remaining_len != 2 {
            return Err(Error::PayloadSizeIncorrect);
        }

        let flags = bytes.get_u8();
        let return_code = bytes.get_u8();

        let session_present = (flags & 0x01) == 1;
        let code = get_return_code(return_code)?;
        let connack = ConnAck { session_present, code };

        Ok(connack)
    }
}

impl ConnAck {
    pub fn new(code: ConnectReturnCode, session_present: bool) -> ConnAck {
        ConnAck { code, session_present }
    }
}

#[cfg(test)]
mod test_connack {
    use crate::*;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;

    #[test]
    fn connack_stitching_works_correctly() {
        let mut stream = bytes::BytesMut::new();
        let packetstream = &[
            0b0010_0000,
            0x02, // packet type, flags and remaining len
            0x01,
            0x00, // variable header. connack flags, connect return code
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra control in the stream
        ];

        stream.extend_from_slice(&packetstream[..]);
        let packet = mqtt_read(&mut stream, 100).unwrap();
        let packet = match packet {
            Packet::ConnAck(packet) => packet,
            packet => panic!("Invalid packet = {:?}", packet),
        };

        assert_eq!(
            packet,
            ConnAck {
                session_present: true,
                code: ConnectReturnCode::Accepted,
            }
        );
    }
}
