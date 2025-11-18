use crate::client::{Client, spawn_read_task, spawn_write_task};
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::info;

pub struct Server {
    clients: Arc<Mutex<Vec<Client>>>,
    next_id: Arc<Mutex<usize>>,
    dc_rx: UnboundedReceiver<usize>,
    dc_tx: UnboundedSender<usize>,
}

impl Server {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<usize>();
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(0)),
            dc_rx: rx,
            dc_tx: tx,
        }
    }

    pub async fn listen(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let listen_addr = format!("{host}:{port}");
        let listener = TcpListener::bind(listen_addr.clone()).await?;
        let clients = self.clients.clone();

        info!("Listening on: {listen_addr}");
        loop {
            tokio::select! {
                Ok((stream, addr)) = listener.accept() => {
                    let client = self.client_from_stream(stream).await;

                    let mut clients = clients.lock().await;
                    clients.push(client);

                    info!("Connection from: {addr}");
                },
                Some(client_id) = self.dc_rx.recv() => {
                    info!("Client {} disconnected", client_id);
                    self.cleanup_clients().await;
                }
            }
        }
    }

    pub async fn broadcast(&self, data: Vec<u8>) {
        let clients = self.clients.clone();
        let clients = clients.lock().await;

        for client in &*clients {
            let _ = client.send(data.clone());
        }
    }

    pub async fn send_to_client(&self, client_id: usize, data: Vec<u8>) -> Result<(), String> {
        let clients = self.clients.clone();
        let clients = clients.lock().await;

        clients
            .iter()
            .find(|c| c.id == client_id)
            .ok_or_else(|| format!("Client {} not found", client_id))?
            .send(data)
            .map_err(|e| e.to_string())
    }

    pub async fn disconnect_client(&mut self, client_id: usize) -> Result<(), String> {
        let clients = self.clients.clone();
        let clients = clients.lock().await;

        let client = clients
            .iter()
            .find(|c| c.id == client_id)
            .ok_or_else(|| format!("Client {} not found", client_id))?;

        client.disconnect();
        self.cleanup_clients().await;

        Ok(())
    }

    pub async fn cleanup_clients(&mut self) {
        let clients = self.clients.clone();
        let mut clients = clients.lock().await;

        let before = clients.len();
        clients.retain(|c| c.is_alive());
        let removed = before - clients.len();

        if removed > 0 {
            info!("Cleaned up {} disconnected clients", removed)
        }
    }

    pub async fn shutdown(&mut self) {
        let clients = self.clients.clone();
        let mut clients = clients.lock().await;

        info!("Shutting server down");
        for client in &*clients {
            client.disconnect();
        }

        clients.clear();
    }

    async fn client_from_stream(&mut self, stream: TcpStream) -> Client {
        let mut client_id_guard = self.next_id.lock().await;
        let client_id = *client_id_guard;

        *client_id_guard += 1;
        drop(client_id_guard);

        let (reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();

        let read_task = spawn_read_task(reader, tx.clone(), self.dc_tx.clone(), client_id);
        let write_task = spawn_write_task(writer, rx, self.dc_tx.clone(), client_id);

        Client {
            id: client_id,
            tx,
            read_task,
            write_task,
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
