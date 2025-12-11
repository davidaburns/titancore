use crate::{
    opcode::LogonOpcode,
    packets::{AuthLogonChallengeRequest, LogonPacket},
};
use anyhow::Result;
use async_trait::async_trait;
use tc_core::server::{Context, PacketHandler};

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
        state: &Self::State,
        ctx: &mut Context,
    ) -> Result<()> {
        match packet.opcode {
            LogonOpcode::CmdAuthLogonChallenge => {
                match AuthLogonChallengeRequest::try_from(packet) {
                    Ok(req) => {
                        tracing::info!("{:?}", req);
                        tracing::info!(
                            "Game Name: {}",
                            std::str::from_utf8(&req.game_name).unwrap()
                        );

                        tracing::info!("OS: {}", std::str::from_utf8(&req.os).unwrap());
                        tracing::info!("Platform: {}", std::str::from_utf8(&req.platform).unwrap());
                        tracing::info!("Country: {}", std::str::from_utf8(&req.country).unwrap());
                        tracing::info!(
                            "Account: {}",
                            std::str::from_utf8(&req.account_name).unwrap()
                        );
                    }
                    Err(e) => tracing::error!("Error parsing AuthLogonChallengeRequest: {e}"),
                };
            }
            _ => {
                let mut output = String::from(format!("Opcode: {:?} Payload: ", packet.opcode));
                for byte in packet.payload {
                    output += format!("0x{:02X} ", byte).as_str();
                }

                tracing::info!("Unknown Bytes: {}", output);
            }
        }

        Ok(())
    }
}
