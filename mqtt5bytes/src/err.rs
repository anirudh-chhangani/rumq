use cfg_if::cfg_if;

// NOTE: Std is an exclusive or features. Won't work with all features enabled
cfg_if! {
    if #[cfg(feature = "std")] {
        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error("Invalid connect return code `{0}`")]
            InvalidConnectReturnCode(u8),
            #[error("Invalid protocol. Expecting 'MQTT' in payload")]
            InvalidProtocol,
            #[error("Invalid protocol level `{0}`")]
            InvalidProtocolLevel(u8),
            #[error("Incorrect packet format")]
            IncorrectPacketFormat,
            #[error("Unsupported Packet type `{0}`")]
            InvalidPacketType(u8),
            #[error("Unsupported QoS `{0}`")]
            InvalidQoS(u8),
            #[error("Invalid packet identifier = 0")]
            PacketIdZero,
            #[error("Payload size incorrect")]
            PayloadSizeIncorrect,
            #[error("Payload too long")]
            PayloadTooLong,
            #[error("Payload size limit exceeded")]
            PayloadSizeLimitExceeded,
            #[error("Payload required")]
            PayloadRequired,
            #[error("Topic name must only contain valid UTF-8")]
            TopicNotUtf8,
            #[error("Malformed remaining length")]
            MalformedRemainingLength,
            #[error("Trying to access wrong boundary")]
            BoundaryCrossed,
            #[error("EOF. Not enough data in buffer")]
            UnexpectedEof,
            #[error("I/O")]
            Io(#[from] std::io::Error),
        }
    } else {
        pub enum Error {
            InvalidConnectReturnCode(u8),
            InvalidProtocol,
            InvalidProtocolLevel(u8),
            IncorrectPacketFormat,
            InvalidPacketType(u8),
            InvalidQoS(u8),
            PacketIdZero,
            PayloadSizeIncorrect,
            PayloadTooLong,
            PayloadSizeLimitExceeded,
            PayloadRequired,
            TopicNotUtf8,
            BoundaryCrossed,
            MalformedRemainingLength,
            UnexpectedEof,
        }
    }
}
