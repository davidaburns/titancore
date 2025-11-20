use std::{collections::HashMap, net::SocketAddr};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, Receiver},
};
use tracing::info;

use crate::server::{
    ClientHandle,
    client::Client,
    messages::{ClientMessage, ServerMessage},
};

pub struct Server {
    pub clients: HashMap<SocketAddr, ClientHandle>,
    pub rx: Receiver<ServerMessage>,
}

impl Server {
    pub fn new(rx: Receiver<ServerMessage>) -> Self {
        Self {
            clients: HashMap::new(),
            rx,
        }
    }

    async fn handle_messages(&mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.rx.recv() => {
                    match msg {
                        ServerMessage::AddClient((addr, tx)) => {
                            info!("Adding client: {addr}");
                            self.clients.insert(addr, tx);
                        }
                        ServerMessage::ClientDisconnected(addr) => {
                            self.client_disconnect(addr);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn shutdown(&mut self) {
        todo!();
    }

    async fn cleanup_clients(&mut self) {
        todo!();
    }

    async fn client_send(&self, data: &Vec<u8>) {
        todo!();
    }

    fn client_disconnect(&mut self, addr: SocketAddr) {
        if let Some(client) = self.clients.get(&addr) {
            if !client.handle.is_finished() {
                client.handle.abort();
            }

            self.clients.remove(&addr);
        }
    }

    async fn broadcast(&self, addr: SocketAddr, data: &Vec<u8>) {
        todo!();
    }
}

pub async fn run_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{host}:{port}");
    let (server_tx, server_rx) = mpsc::channel::<ServerMessage>(100);
    let mut server = Server::new(server_rx);

    tokio::spawn(async move {
        server.handle_messages().await;
    });

    let listener = TcpListener::bind(addr.clone()).await?;
    info!("Listening on: {addr}");
    loop {
        let (stream, addr) = listener.accept().await?;
        let (reader, writer) = tokio::io::split(stream);
        let (client_tx, client_rx) = mpsc::channel::<ClientMessage>(32);

        let server_tx_clone = server_tx.clone();
        let mut client = Client {
            id: addr,
            reader,
            writer,
            tx: server_tx_clone,
            rx: client_rx,
        };

        let handle = tokio::spawn(async move {
            client.handle_messages().await;
        });

        let client_handle = ClientHandle {
            tx: client_tx,
            handle,
        };

        let server_tx_clone = server_tx.clone();
        server_tx_clone
            .send(ServerMessage::AddClient((addr, client_handle)))
            .await?;
    }
}
