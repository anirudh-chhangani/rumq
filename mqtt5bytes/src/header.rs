struct FixedHeader {
    byte1: u8,
    header_len: usize,
    remaining_len: usize,
}
