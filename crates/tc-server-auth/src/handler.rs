use crate::packets::LogonPacket;
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
            _ => {
                let mut output = String::from(format!("Opcode: {:?} Payload: ", packet.opcode));
                for byte in packet.payload {
                    output += format!("0x{:02X} ", byte).as_str();
                }

                tracing::info!("{}", output);
                if let Err(e) = ctx.send_bytes(output.as_bytes().to_vec()).await {
                    tracing::error!("Error sending to client: {e}");
                }
            }
        }

        Ok(())
    }
}
