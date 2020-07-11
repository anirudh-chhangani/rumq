use bytes::{Bytes, Buf, BytesMut};
use alloc::string::String;
use crate::{Error, ByteLengths, decode_variable_byte, decode_utf_string, Utf8Pair, decode_utf_string_pair};
use alloc::vec::Vec;

pub(crate) struct PropertyIdentifiers;

/// property identifiers as defined in MQTT v5.0 spec
impl PropertyIdentifiers {
    pub const PAYLOAD_FORMAT_INDICATOR: u8 = 0;
    pub const MESSAGE_EXPIRY_INTERVAL: u8 = 2;
    pub const CONTENT_TYPE: u8 = 3;
    pub const RESPONSE_TOPIC: u8 = 8;
    pub const CORRELATION_DATA: u8 = 9;
    pub const SUBSCRIPTION_IDENTIFIER: u8 = 11;
    pub const SESSION_EXPIRY_INTERVAL: u8 = 17;
    pub const ASSIGNED_CLIENT_IDENTIFIER: u8 = 18;
    pub const SERVER_KEEP_ALIVE: u8 = 19;
    pub const AUTHENTICATION_METHOD: u8 = 21;
    pub const AUTHENTICATION_DATA: u8 = 22;
    pub const REQUEST_PROBLEM_INFORMATION: u8 = 23;
    pub const WILL_DELAY_INTERVAL: u8 = 24;
    pub const REQUEST_RESPONSE_INFORMATION: u8 = 25;
    pub const RESPONSE_INFO: u8 = 26;
    pub const SERVER_INFO: u8 = 28;
    pub const REASON_STRING: u8 = 31;
    pub const RECEIVE_MAXIMUM: u8 = 33;
    pub const TOPIC_ALIAS_MAXIMUM: u8 = 34;
    pub const TOPIC_ALIAS: u8 = 35;
    pub const MAXIMUM_QOS: u8 = 36;
    pub const RETAIN_AVAILABLE: u8 = 37;
    pub const USER_PROPERTY: u8 = 38;
    pub const MAXIMUM_PACKET_SIZE: u8 = 39;
    pub const WILDCARD_SUBSCRIPTION_AVAILABLE: u8 = 40;
    pub const SUBSCRIPTION_IDENTIFIER_AVAILABLE: u8 = 41;
    pub const SHARED_SUBSCRIPTION_AVAILABLE: u8 = 42;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Properties {
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<String>,
    pub subscription_identifier: Option<u32>,
    pub session_expiry_interval: Option<u32>,
    pub assigned_client_identifier: Option<String>,
    pub server_keep_alive: Option<u16>,
    pub authentication_method: Option<String>,
    pub authentication_data: Option<String>,
    pub request_problem_info: Option<u8>,
    pub will_delay_interval: Option<u32>,
    pub request_response_info: Option<u8>,
    pub response_info: Option<String>,
    pub server_info: Option<String>,
    pub reason_string: Option<String>,
    pub receive_maximum: Option<u16>,
    pub topic_alias_maximum: Option<u16>,
    pub topic_alias: Option<u16>,
    pub maximum_qos: Option<u8>,
    pub retain_available: Option<u8>,
    pub user_property: Option<Utf8Pair>,
    pub maximum_packet_size: Option<u32>,
    pub wildcard_subscription_available: Option<u8>,
    pub subscription_identifier_available: Option<u8>,
    pub shared_subscription_available: Option<u8>,
}

pub fn extract_properties(stream: &mut Bytes) -> Result<Option<Properties>, Error> {
    let (_prop_length, _read) = decode_variable_byte(stream);
    let mut prop_length = _prop_length?;
    if prop_length > 0 {
        let mut payload_format_indicator: Option<u8> = None;
        let mut message_expiry_interval: Option<u32> = None;
        let mut content_type: Option<String> = None;
        let mut response_topic: Option<String> = None;
        let mut correlation_data: Option<String> = None; // binary data
        let mut subscription_identifier: Option<u32> = None;
        let mut session_expiry_interval: Option<u32> = None;
        let mut assigned_client_identifier: Option<String> = None;
        let mut server_keep_alive: Option<u16> = None;
        let mut authentication_method: Option<String> = None;
        let mut authentication_data: Option<String> = None; // binary data
        let mut request_problem_info: Option<u8> = None;
        let mut will_delay_interval: Option<u32> = None;
        let mut request_response_info: Option<u8> = None;
        let mut response_info: Option<String> = None;
        let mut server_info: Option<String> = None;
        let mut reason_string: Option<String> = None;
        let mut receive_maximum: Option<u16> = None;
        let mut topic_alias_maximum: Option<u16> = None;
        let mut topic_alias: Option<u16> = None;
        let mut maximum_qos: Option<u8> = None;
        let mut retain_available: Option<u8> = None;
        let mut user_property: Option<Utf8Pair> = None;
        let mut maximum_packet_size: Option<u32> = None;
        let mut wildcard_subscription_available: Option<u8> = None;
        let mut subscription_identifier_available: Option<u8> = None;
        let mut shared_subscription_available: Option<u8> = None;

        {
            while prop_length > 0 {
                // initial 1 byte identifier for the property
                let ident = stream.get_u8();
                prop_length -= ByteLengths::BYTE_INT;

                // match identifier to extract properties
                match ident {
                    PropertyIdentifiers::PAYLOAD_FORMAT_INDICATOR => {
                        payload_format_indicator = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::MESSAGE_EXPIRY_INTERVAL => {
                        message_expiry_interval = Some(stream.get_u32());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::CONTENT_TYPE => {
                        let (data, len) = decode_utf_string(stream);
                        content_type = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::RESPONSE_TOPIC => {
                        let (data, len) = decode_utf_string(stream);
                        response_topic = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::CORRELATION_DATA => {
                        let (data, len) = decode_utf_string(stream);
                        correlation_data = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::SUBSCRIPTION_IDENTIFIER => {
                        subscription_identifier = Some(stream.get_u32());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::SESSION_EXPIRY_INTERVAL => {
                        session_expiry_interval = Some(stream.get_u32());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::ASSIGNED_CLIENT_IDENTIFIER => {
                        let (data, len) = decode_utf_string(stream);
                        assigned_client_identifier = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::SERVER_KEEP_ALIVE => {
                        server_keep_alive = Some(stream.get_u16());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::AUTHENTICATION_METHOD => {
                        let (data, len) = decode_utf_string(stream);
                        authentication_method = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::AUTHENTICATION_DATA => {
                        let (data, len) = decode_utf_string(stream);
                        authentication_data = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::REQUEST_PROBLEM_INFORMATION => {
                        request_problem_info = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::WILL_DELAY_INTERVAL => {
                        will_delay_interval = Some(stream.get_u32());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::REQUEST_RESPONSE_INFORMATION => {
                        request_response_info = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::RESPONSE_INFO => {
                        let (data, len) = decode_utf_string(stream);
                        response_info = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::SERVER_INFO => {
                        let (data, len) = decode_utf_string(stream);
                        server_info = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::REASON_STRING => {
                        let (data, len) = decode_utf_string(stream);
                        reason_string = Some(data?);
                        prop_length -= len;
                    }
                    PropertyIdentifiers::RECEIVE_MAXIMUM => {
                        receive_maximum = Some(stream.get_u16());
                        prop_length -= ByteLengths::TWO_BYTE_INT;
                    }
                    PropertyIdentifiers::TOPIC_ALIAS_MAXIMUM => {
                        topic_alias_maximum = Some(stream.get_u16());
                        prop_length -= ByteLengths::TWO_BYTE_INT;
                    }
                    PropertyIdentifiers::TOPIC_ALIAS => {
                        topic_alias = Some(stream.get_u16());
                        prop_length -= ByteLengths::TWO_BYTE_INT;
                    }
                    PropertyIdentifiers::MAXIMUM_QOS => {
                        maximum_qos = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::RETAIN_AVAILABLE => {
                        retain_available = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::USER_PROPERTY => {
                        let (data, len) = decode_utf_string_pair(stream);
                        user_property = Some(data?);
                        prop_length -= len
                    }
                    PropertyIdentifiers::MAXIMUM_PACKET_SIZE => {
                        maximum_packet_size = Some(stream.get_u32());
                        prop_length -= ByteLengths::FOUR_BYTE_INT;
                    }
                    PropertyIdentifiers::WILDCARD_SUBSCRIPTION_AVAILABLE => {
                        wildcard_subscription_available = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::SUBSCRIPTION_IDENTIFIER_AVAILABLE => {
                        subscription_identifier_available = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    PropertyIdentifiers::SHARED_SUBSCRIPTION_AVAILABLE => {
                        shared_subscription_available = Some(stream.get_u8());
                        prop_length -= ByteLengths::BYTE_INT;
                    }
                    _ => {
                        return Err(Error::InvalidProperty);
                    }
                }
            }
        }

        let props = Properties {
            payload_format_indicator,
            message_expiry_interval,
            content_type,
            response_topic,
            correlation_data,
            subscription_identifier,
            session_expiry_interval,
            assigned_client_identifier,
            server_keep_alive,
            authentication_method,
            authentication_data,
            request_problem_info,
            will_delay_interval,
            request_response_info,
            response_info,
            server_info,
            reason_string,
            receive_maximum,
            topic_alias_maximum,
            topic_alias,
            maximum_qos,
            retain_available,
            user_property,
            maximum_packet_size,
            wildcard_subscription_available,
            subscription_identifier_available,
            shared_subscription_available,
        };

        return Ok(Some(props));
    }
    return Ok(None);
}