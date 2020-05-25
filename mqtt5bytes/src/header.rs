pub(crate) struct FixedHeader {
    pub(crate) byte1: u8,
    pub(crate) header_len: usize,
    pub(crate) remaining_len: usize,
}
