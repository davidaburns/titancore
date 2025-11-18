use crate::client::{Client, spaw_write_task, spawn_read_task};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::info;

pub struct Server {
    clients: Vec<Client>,
    next_id: usize,
}

impl Server {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
            next_id: 0,
        }
    }

    pub async fn listen(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let listen_addr = format!("{host}:{port}");
        let listener = TcpListener::bind(listen_addr.clone()).await?;

        info!("Listening on: {listen_addr}");
        loop {
            tokio::select! {
                Ok((stream, addr)) = listener.accept() => {
                    let client = self.client_from_stream(stream);

                    self.clients.push(client);
                    info!("Connection from: {addr}");
                }
            }
        }
    }

    pub fn send_to_client(&self, client_id: usize, data: Vec<u8>) -> Result<(), String> {
        self.clients
            .iter()
            .find(|c| c.id == client_id)
            .ok_or_else(|| format!("Client {} not found", client_id))?
            .send(data)
            .map_err(|e| e.to_string())
    }

    pub fn broadcast(&self, data: Vec<u8>) {
        for client in &self.clients {
            let _ = client.send(data.clone());
        }
    }

    pub fn disconnect_client(&mut self, client_id: usize) -> Result<(), String> {
        let client = self
            .clients
            .iter()
            .find(|c| c.id == client_id)
            .ok_or_else(|| format!("Client {} not found", client_id))?;

        client.disconnect();
        self.cleanup_clients();

        Ok(())
    }

    pub fn cleanup_clients(&mut self) {
        let before = self.clients.len();
        self.clients.retain(|c| c.is_alive());
        let removed = before - self.clients.len();

        if removed > 0 {
            info!("Cleaned up {} disconnected clients", removed)
        }
    }

    pub fn shutdown(&mut self) {
        info!("Shutting server down");
        for client in &self.clients {
            client.disconnect();
        }

        self.clients.clear();
    }

    fn client_from_stream(&mut self, stream: TcpStream) -> Client {
        let client_id = self.next_id;
        self.next_id += 1;

        let (reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();

        let read_task = spawn_read_task(reader, tx.clone());
        let write_task = spaw_write_task(writer, rx, client_id);

        Client {
            id: client_id,
            tx: tx,
            read_task,
            write_task,
        }
    }
}
