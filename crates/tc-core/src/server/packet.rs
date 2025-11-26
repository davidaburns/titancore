use crate::server::Context;
use anyhow::Result;
use async_trait::async_trait;

pub trait Packet: Send + Sync + 'static {
    fn encode(&self) -> Result<Vec<u8>>;

    fn decode(payload: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

#[async_trait]
pub trait PacketHandler: Send + Sync + 'static {
    type Packet: Packet;
    type State: Send + Sync + 'static;

    async fn handle(
        &self,
        packet: Self::Packet,
        state: &Self::State,
        ctx: &mut Context,
    ) -> Result<Option<Self::Packet>>;
}
