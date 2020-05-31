use crate::Error;
use crate::FixedHeader;

use bytes::Bytes;

#[derive(Debug, Clone, PartialEq)]
pub struct Auth {}

impl Auth {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {
        Ok(Auth{})
    }
}
