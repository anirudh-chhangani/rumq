use crate::control::*;

#[derive(Debug, Clone)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq,
    PingResp,
    Disconnect(Disconnect),
    Auth(Auth),
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ConnectReturnCode {
    Accepted = 0,
    RefusedProtocolVersion,
    RefusedIdentifierRejected,
    ServerUnavailable,
    BadUsernamePassword,
    NotAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscribeReturnCodes {
    Success(QoS),
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    MQTT(u8),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Connect = 1,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
    Auth,
}
