use crate::server::Server;

#[derive(Debug, Clone)]
pub struct Packet {
    pub opcode: u8,
    pub payload: Vec<u8>,
}

pub trait PacketParser {
    fn from_bytes(bytes: &Vec<u8>) -> Self;
}

pub trait PacketProcessor {
    fn process(&self, server: &mut Server);
}
