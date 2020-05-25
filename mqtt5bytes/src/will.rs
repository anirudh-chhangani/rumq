use crate::QoS;
use alloc::string::String;

#[derive(Debug, Clone, PartialEq)]
pub struct LastWill {
    pub topic: String,
    pub message: String,
    pub qos: QoS,
    pub retain: bool,
}
