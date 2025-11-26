use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tc_core::server::{Context, Packet, PacketHandler};

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
    CmdUknownOpcode = 0xFF,
}

impl From<u8> for LogonOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => LogonOpcode::CmdAuthLogonChallenge,
            0x01 => LogonOpcode::CmdAuthLogonProof,
            0x02 => LogonOpcode::CmdAuthReconnectChallenge,
            0x03 => LogonOpcode::CmdAuthReconnectProof,
            0x04 => LogonOpcode::CmdSurveyResult,
            0x10 => LogonOpcode::CmdRealmList,
            0x30 => LogonOpcode::CmdXferInitiate,
            0x31 => LogonOpcode::CmdXferData,
            0x32 => LogonOpcode::CmdXferAccept,
            0x33 => LogonOpcode::CmdXferResume,
            0x34 => LogonOpcode::CmdXferCancel,
            _ => LogonOpcode::CmdUknownOpcode,
        }
    }
}

pub struct LogonPacket {
    opcode: LogonOpcode,
    _payload: Vec<u8>,
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
            payload[1..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            opcode: LogonOpcode::from(op),
            _payload: payload,
        })
    }
}

pub struct ServerState;
impl ServerState {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct AuthServer;

#[async_trait]
impl PacketHandler for AuthServer {
    type Packet = LogonPacket;
    type State = ServerState;

    async fn handle(
        &self,
        packet: Self::Packet,
        _state: &Self::State,
        _ctx: &mut Context,
    ) -> Result<()> {
        match packet.opcode {
            _ => {
                let mut output = String::from(format!("Opcode: {:?} Payload: ", packet.opcode));
                for byte in packet._payload {
                    output += format!("0x{:02X} ", byte).as_str();
                }

                tracing::info!("{}", output);
                if let Err(e) = _ctx.send_bytes(output.as_bytes().to_vec()).await {
                    tracing::error!("Error sending to client: {e}");
                }
            }
        }

        Ok(())
    }
}
