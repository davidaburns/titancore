// use std::{collections::HashMap, net::SocketAddr};
// use tokio::{
//     net::TcpListener,
//     sync::mpsc::{self, Receiver},
// };
// use tracing::{error, info};

// use crate::server::{
//     ClientHandle,
//     client::Client,
//     messages::{ClientMessage, ServerMessage},
// };

// pub struct Server {
//     pub clients: HashMap<SocketAddr, ClientHandle>,
//     pub rx: Receiver<ServerMessage>,
//     pub running: bool,
// }

// impl Server {
//     pub fn new(rx: Receiver<ServerMessage>) -> Self {
//         Self {
//             clients: HashMap::new(),
//             rx,
//             running: false,
//         }
//     }

//     pub async fn handle_messages(&mut self) {
//         loop {
//             tokio::select! {
//                 Some(msg) = self.rx.recv() => {
//                     match msg {
//                         ServerMessage::ServerAddClient((addr, tx)) => {
//                             self.clients.insert(addr, tx);
//                         }
//                         ServerMessage::ClientDisconnected(addr) => {
//                             self.client_disconnect(addr);
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     pub async fn shutdown(&mut self) {
//         info!("Server shutting down");
//         for (_, client) in self.clients.iter() {
//             if !client.handle.is_finished() {
//                 client.handle.abort();
//             }
//         }

//         self.clients.clear();
//     }

//     pub async fn cleanup_clients(&mut self) {
//         let before = self.clients.len();
//         self.clients.retain(|_, c| !c.handle.is_finished());

//         let after = before - self.clients.len();

//         info!("Cleaned up {after} clients");
//     }

//     pub async fn client_send(&self, addr: SocketAddr, data: Vec<u8>) {
//         if let Some(client) = self.clients.get(&addr) {
//             if let Err(e) = client.tx.send(ClientMessage::Send(data)).await {
//                 error!("Error while communicating over server -> client channel: {e}");
//             }
//         }
//     }

//     pub fn client_disconnect(&mut self, addr: SocketAddr) {
//         if let Some(client) = self.clients.get(&addr) {
//             if !client.handle.is_finished() {
//                 client.handle.abort();
//             }

//             self.clients.remove(&addr);
//         }
//     }

//     pub async fn broadcast(&self, data: Vec<u8>) {
//         for (_, client) in self.clients.iter() {
//             if let Err(e) = client.tx.send(ClientMessage::Send(data.clone())).await {
//                 error!("Error while broadcasting to clients: {e}");
//             }
//         }
//     }
// }

// pub async fn run_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
//     let addr = format!("{host}:{port}");
//     let (server_tx, server_rx) = mpsc::channel::<ServerMessage>(100);
//     let mut server = Server::new(server_rx);

//     tokio::spawn(async move {
//         server.handle_messages().await;
//     });

//     let listener = TcpListener::bind(addr.clone()).await?;
//     info!("Listening on: {addr}");
//     loop {
//         let (stream, addr) = listener.accept().await?;
//         let (reader, writer) = tokio::io::split(stream);
//         let (client_tx, client_rx) = mpsc::channel::<ClientMessage>(32);

//         let server_tx_clone = server_tx.clone();
//         let mut client = Client {
//             id: addr,
//             reader,
//             writer,
//             tx: server_tx_clone,
//             rx: client_rx,
//         };

//         let handle = tokio::spawn(async move {
//             client.handle_messages().await;
//         });

//         let client_handle = ClientHandle {
//             tx: client_tx,
//             handle,
//         };

//         let server_tx_clone = server_tx.clone();
//         server_tx_clone
//             .send(ServerMessage::ServerAddClient((addr, client_handle)))
//             .await?;
//     }
// }
//

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

            if let Some(response) = handler.handle(packet, &state, &mut ctx).await? {
                ctx.send_packet(response).await?;
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
