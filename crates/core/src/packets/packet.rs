#[derive(Debug, Clone)]
pub struct Packet {
    pub opcode: u8,
    pub payload: Vec<u8>,
}
