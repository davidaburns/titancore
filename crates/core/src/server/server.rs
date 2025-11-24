use std::{collections::HashMap, net::SocketAddr};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, Receiver},
};
use tracing::{error, info};

use crate::server::{
    ClientHandle,
    client::Client,
    messages::{ClientMessage, ServerMessage},
};

pub struct Server {
    pub clients: HashMap<SocketAddr, ClientHandle>,
    pub rx: Receiver<ServerMessage>,
    pub running: bool,
}

impl Server {
    pub fn new(rx: Receiver<ServerMessage>) -> Self {
        Self {
            clients: HashMap::new(),
            rx,
            running: false,
        }
    }

    pub async fn handle_messages(&mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.rx.recv() => {
                    match msg {
                        ServerMessage::ServerAddClient((addr, tx)) => {
                            self.clients.insert(addr, tx);
                        }
                        ServerMessage::ClientDisconnected(addr) => {
                            self.client_disconnect(addr);
                        }
                    }
                }
            }
        }
    }

    pub async fn shutdown(&mut self) {
        info!("Server shutting down");
        for (_, client) in self.clients.iter() {
            if !client.handle.is_finished() {
                client.handle.abort();
            }
        }

        self.clients.clear();
    }

    pub async fn cleanup_clients(&mut self) {
        let before = self.clients.len();
        self.clients.retain(|_, c| !c.handle.is_finished());

        let after = before - self.clients.len();

        info!("Cleaned up {after} clients");
    }

    pub async fn client_send(&self, addr: SocketAddr, data: Vec<u8>) {
        if let Some(client) = self.clients.get(&addr) {
            if let Err(e) = client.tx.send(ClientMessage::Send(data)).await {
                error!("Error while communicating over server -> client channel: {e}");
            }
        }
    }

    pub fn client_disconnect(&mut self, addr: SocketAddr) {
        if let Some(client) = self.clients.get(&addr) {
            if !client.handle.is_finished() {
                client.handle.abort();
            }

            self.clients.remove(&addr);
        }
    }

    pub async fn broadcast(&self, data: Vec<u8>) {
        for (_, client) in self.clients.iter() {
            if let Err(e) = client.tx.send(ClientMessage::Send(data.clone())).await {
                error!("Error while broadcasting to clients: {e}");
            }
        }
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
            .send(ServerMessage::ServerAddClient((addr, client_handle)))
            .await?;
    }
}
