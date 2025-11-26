use anyhow::Result;
use async_trait::async_trait;
use tc_core::server::{Context, Packet, PacketHandler};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogonPacketError {
    #[error("Invalid opcode")]
    InvalidOpcode,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LogonOpcode {
    CmdAuthLogonChallenge = 0x00,
    CmdAuthLogonProof = 0x01,
    CmdAuthReconnectChallenge = 0x02,
    CmdAuthReconnectProof = 0x03,
    CmdSurveyResult = 0x04,
    CmdRealmList = 0x10,
    CmdXferInitiate = 0x30,
    CmdXferData = 0x31,
    CmdXferAccept = 0x32,
    CmdXferResume = 0x33,
    CmdXferCancel = 0x34,
}

impl TryFrom<u8> for LogonOpcode {
    type Error = LogonPacketError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x00 => Ok(LogonOpcode::CmdAuthLogonChallenge),
            0x01 => Ok(LogonOpcode::CmdAuthLogonProof),
            0x02 => Ok(LogonOpcode::CmdAuthReconnectChallenge),
            0x03 => Ok(LogonOpcode::CmdAuthReconnectProof),
            0x04 => Ok(LogonOpcode::CmdSurveyResult),
            0x10 => Ok(LogonOpcode::CmdRealmList),
            0x31 => Ok(LogonOpcode::CmdXferInitiate),
            0x32 => Ok(LogonOpcode::CmdXferData),
            0x33 => Ok(LogonOpcode::CmdXferResume),
            0x34 => Ok(LogonOpcode::CmdXferCancel),
            _ => Err(LogonPacketError::InvalidOpcode),
        }
    }
}

pub struct LogonPacket {
    opcode: LogonOpcode,
    payload: Vec<u8>,
}

impl Packet for LogonPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode(payload: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let op = payload[0];
        let payload = payload[1..].to_vec();

        Ok(Self {
            opcode: LogonOpcode::try_from(op)?,
            payload,
        })
    }
}

pub struct ServerState;
impl ServerState {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct ServerPacketHandler;

#[async_trait]
impl PacketHandler for ServerPacketHandler {
    type Packet = LogonPacket;
    type State = ServerState;

    async fn handle(
        &self,
        packet: Self::Packet,
        state: &Self::State,
        ctx: &mut Context,
    ) -> Result<Option<Self::Packet>> {
        match packet.opcode {
            _ => tracing::info!("Unknown opcode: {:?}", packet.opcode),
        }
        Ok(None)
    }
}
