use crate::opcode::LogonOpcode;
use anyhow::{Result, anyhow};
use tc_core::server::Packet;

pub struct LogonPacket {
    pub opcode: LogonOpcode,
    pub payload: Vec<u8>,
}

impl Packet for LogonPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode(payload: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        if payload.len() == 0 {
            return Err(anyhow!("packet payload is empty"));
        }

        let op = payload[0];
        let payload = if payload.len() >= 2 {
            payload[1..]
                .to_vec()
                .iter()
                .copied()
                .filter(|&b| b != 0x0D && b != 0x0A)
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            opcode: LogonOpcode::from(op),
            payload,
        })
    }
}
