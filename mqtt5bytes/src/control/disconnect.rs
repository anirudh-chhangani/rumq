use crate::Error;
use crate::FixedHeader;

use bytes::Bytes;

pub struct Disconnect {}

impl Disconnect {
    pub(crate) fn assemble(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Self, Error> {}
}
