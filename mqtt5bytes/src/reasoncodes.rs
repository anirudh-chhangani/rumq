pub struct ReasonCode;

/// The CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, DISCONNECT and AUTH Control Packets have a single Reason Code as part of the Variable Header.
/// The SUBACK and UNSUBACK packets contain a list of one or more Reason Codes in the Payload.
impl ReasonCode {
    pub const SUCCESS: u8 = 0;
    pub const NORMAL_DISCONNECT: u8 = 0;
    pub const GRANTED_QOS_0: u8 = 0;
    pub const GRANTED_QOS_1: u8 = 1;
    pub const GRANTED_QOS_2: u8 = 2;
    pub const DISCONNECT_WITH_WILL: u8 = 4;
    pub const NO_MATCHING_SUBSCRIBERS: u8 = 16;
    pub const NO_SUBSCRIPTION_EXISTED: u8 = 17;
    pub const CONTINUE_AUTHENTICATION: u8 = 24;
    pub const RE_AUTHENTICATE: u8 = 25;
    pub const UNSPECIFIED_ERROR: u8 = 128;
    pub const MALFORMED_PACKET: u8 = 129;
    pub const PROTO_ERROR: u8 = 130;
    pub const IMPLEMENTATION_ERROR: u8 = 131;
    pub const UNSUPPORTED_PROTO_VERSION: u8 = 132;
    pub const CLIENT_ID_NOT_VALID: u8 = 133;
    pub const BAD_USER_NAME_OR_PASSWORD: u8 = 134;
    pub const NOT_AUTHORIZED: u8 = 135;
    pub const SERVER_UNAVAIL: u8 = 136;
    pub const SERVER_BUSY: u8 = 137;
    pub const BANNED: u8 = 138;
    pub const SERVER_SHUTTING_DOWN: u8 = 139;
    pub const BAD_AUTH_METHOD: u8 = 140;
    pub const KEEP_ALIVE_TIMEOUT: u8 = 141;
    pub const SESSION_TAKE_OVER: u8 = 142;
    pub const TOPIC_FILTER_INVALID: u8 = 143;
    pub const TOPIC_NAME_INVALID: u8 = 144;
    pub const PACKET_IDENTIFIER_IN_USE: u8 = 145;
    pub const PACKET_IDENTIFIER_NOT_FOUND: u8 = 146;
    pub const RECEIVE_MAXIMUM_EXCEEDED: u8 = 147;
    pub const TOPIC_ALIAS_INVALID: u8 = 148;
    pub const PACKET_TOO_LARGE: u8 = 149;
    pub const MESSAGE_RATE_TOO_HIGH: u8 = 150;
    pub const QUOTA_EXCEEDED: u8 = 151;
    pub const ADMINISTRATIVE_ACTION: u8 = 152;
    pub const PAYLOAD_FORMAT_INVALID: u8 = 153;
    pub const RETAIN_NOT_SUPPORTED: u8 = 154;
    pub const QOS_NOT_SUPPORTED: u8 = 155;
    pub const USE_ANOTHER_SERVER: u8 = 156;
    pub const SERVER_MOVED: u8 = 157;
    pub const SHARED_SUB_NOT_SUPPORTED: u8 = 158;
    pub const CONNECTION_RATE_EXCEEDED: u8 = 159;
    pub const MAXIMUM_CONNECT_TIME: u8 = 160;
    pub const SUBSCRIPTION_ID_UNSUPPORTED: u8 = 161;
    pub const WILDCARD_SUBSCRIPTION_NOT_SUPPORTED: u8 = 162;
}

#[cfg(test)]
mod test_reason_code {
    use crate::reasoncodes::ReasonCode;

    #[test]
    fn reason_code_assertion() {
        let reason_code: ReasonCode;
        let value1: u8 = 159;
        assert_eq!(value1, ReasonCode::CONNECTION_RATE_EXCEEDED);
        assert_ne!(value1, ReasonCode::WILDCARD_SUBSCRIPTION_NOT_SUPPORTED);
    }
}
