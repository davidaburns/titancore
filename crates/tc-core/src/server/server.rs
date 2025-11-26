use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpListener, TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::mpsc,
};

use crate::server::{ConnectionId, ConnectionRegistry, Context, Packet, PacketHandler};

pub struct Server<H: PacketHandler> {
    handler: Arc<H>,
    state: Arc<H::State>,
    registry: Arc<ConnectionRegistry>,
}

impl<H: PacketHandler> Server<H> {
    pub fn new(handler: H, state: H::State) -> Self {
        Self {
            handler: Arc::new(handler),
            state: Arc::new(state),
            registry: Arc::new(ConnectionRegistry::new()),
        }
    }

    pub async fn run(self, addr: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        tracing::info!("Listening on: {addr}");

        loop {
            let (stream, peer_addr) = listener.accept().await?;

            let handler = Arc::clone(&self.handler);
            let state = Arc::clone(&self.state);
            let registry = Arc::clone(&self.registry);

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(stream, peer_addr, handler, state, registry).await
                {
                    tracing::error!("Connection error: {e}");
                }
            });
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        handler: Arc<H>,
        state: Arc<H::State>,
        registry: Arc<ConnectionRegistry>,
    ) -> Result<()> {
        let id = ConnectionId::new();
        let (reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::channel(32);

        registry.register(id, tx.clone(), addr).await;
        tokio::spawn(Self::write_loop(writer, rx));

        let result =
            Self::read_loop(reader, addr, id, handler, state, tx, Arc::clone(&registry)).await;

        registry.unregister(id).await;
        result
    }

    async fn read_loop(
        mut reader: OwnedReadHalf,
        addr: SocketAddr,
        id: ConnectionId,
        handler: Arc<H>,
        state: Arc<H::State>,
        tx: mpsc::Sender<Vec<u8>>,
        registry: Arc<ConnectionRegistry>,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 1500];
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            let packet = H::Packet::decode(&buffer[..n])?;
            let mut ctx = Context::new(id, addr, tx.clone(), Arc::clone(&registry));

            if let Err(e) = handler.handle(packet, &state, &mut ctx).await {
                tracing::warn!("Error while handling packet: {e}");
            }
        }

        Ok(())
    }

    async fn write_loop(mut writer: OwnedWriteHalf, mut rx: mpsc::Receiver<Vec<u8>>) -> Result<()> {
        while let Some(bytes) = rx.recv().await {
            writer.write_all(&bytes).await?;
        }

        Ok(())
    }
}
