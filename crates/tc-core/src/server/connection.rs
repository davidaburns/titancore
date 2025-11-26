use anyhow::Result;
use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone)]
pub struct ConnectionHandle {
    _id: ConnectionId,
    sender: mpsc::Sender<Vec<u8>>,
    addr: SocketAddr,
}

pub struct ConnectionRegistry {
    connections: RwLock<HashMap<ConnectionId, ConnectionHandle>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(
        &self,
        id: ConnectionId,
        sender: mpsc::Sender<Vec<u8>>,
        addr: SocketAddr,
    ) {
        self.connections.write().await.insert(
            id,
            ConnectionHandle {
                _id: id,
                sender,
                addr,
            },
        );
    }

    pub async fn unregister(&self, id: ConnectionId) {
        self.connections.write().await.remove(&id);
    }

    pub async fn count(&self) -> usize {
        self.connections.read().await.len()
    }

    pub async fn send_to(&self, id: ConnectionId, bytes: Vec<u8>) -> Result<()> {
        let connections = self.connections.read().await;
        if let Some(handle) = connections.get(&id) {
            handle.sender.send(bytes).await?
        }

        Ok(())
    }

    pub async fn broadcast_all(&self, bytes: Vec<u8>) -> Result<()> {
        let connections = self.connections.read().await;
        for handle in connections.values() {
            handle.sender.send(bytes.clone()).await?;
        }

        Ok(())
    }

    pub async fn broadcast_except(&self, sender_id: ConnectionId, bytes: Vec<u8>) -> Result<()> {
        let connections = self.connections.read().await;
        for (id, handle) in connections.iter() {
            if *id == sender_id {
                continue;
            }

            handle.sender.send(bytes.clone()).await?;
        }

        Ok(())
    }

    pub async fn broadcast_filter<F>(&self, bytes: Vec<u8>, filter: F) -> Result<()>
    where
        F: Fn(&ConnectionHandle) -> bool,
    {
        let connections = self.connections.read().await;
        for handle in connections.values() {
            if filter(handle) {
                handle.sender.send(bytes.clone()).await?;
            }
        }

        Ok(())
    }

    pub async fn ids(&self) -> Vec<ConnectionId> {
        self.connections.read().await.keys().copied().collect()
    }

    pub async fn get_addr(&self, id: ConnectionId) -> Option<SocketAddr> {
        self.connections.read().await.get(&id).map(|h| h.addr)
    }
}
