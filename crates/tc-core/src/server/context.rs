use crate::server::{ConnectionHandle, ConnectionId, ConnectionRegistry, Packet};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;

pub struct Context {
    connection_id: ConnectionId,
    addr: SocketAddr,
    sender: mpsc::Sender<Vec<u8>>,
    registry: Arc<ConnectionRegistry>,
}

impl Context {
    pub fn new(
        id: ConnectionId,
        addr: SocketAddr,
        sender: mpsc::Sender<Vec<u8>>,
        registry: Arc<ConnectionRegistry>,
    ) -> Self {
        Self {
            connection_id: id,
            addr,
            sender,
            registry,
        }
    }
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn send_packet(&mut self, packet: impl Packet) -> Result<()> {
        let bytes = packet.encode()?;
        self.sender.send(bytes).await?;

        Ok(())
    }

    pub async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
        self.sender.send(bytes).await?;
        Ok(())
    }

    pub async fn send_to(&self, target: ConnectionId, packet: impl Packet) -> Result<()> {
        let bytes = packet.encode()?;
        self.registry.send_to(target, bytes).await?;

        Ok(())
    }

    pub async fn broadcast_others(&self, packet: impl Packet) -> Result<()> {
        let bytes = packet.encode()?;
        self.registry
            .broadcast_except(self.connection_id, bytes)
            .await?;

        Ok(())
    }

    pub async fn broadcast_all(&self, packet: impl Packet) -> Result<()> {
        let bytes = packet.encode()?;
        self.registry.broadcast_all(bytes).await?;

        Ok(())
    }

    pub async fn broadcast_filter<F>(&self, packet: impl Packet, filter: F) -> Result<()>
    where
        F: Fn(&ConnectionHandle) -> bool,
    {
        let bytes = packet.encode()?;
        self.registry.broadcast_filter(bytes, filter).await?;

        Ok(())
    }

    pub async fn connections(&self) -> Vec<ConnectionId> {
        self.registry.ids().await
    }

    pub async fn connection_count(&self) -> usize {
        self.registry.count().await
    }
}
