use crate::server::{ClientMessage, ServerMessage};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::error;

pub struct ClientHandle {
    pub tx: Sender<ClientMessage>,
    pub handle: JoinHandle<()>,
}

pub struct Client {
    pub id: SocketAddr,
    pub reader: tokio::io::ReadHalf<TcpStream>,
    pub writer: tokio::io::WriteHalf<TcpStream>,
    pub tx: Sender<ServerMessage>,
    pub rx: Receiver<ClientMessage>,
}

impl Client {
    pub async fn handle_messages(&mut self) {
        let mut buffer = [0u8; 1500];
        loop {
            tokio::select! {
               // Reading bytes over the tcp stream
               result = self.reader.read(&mut buffer) => {
                   match result {
                       Ok(0) => {
                           self.server_send(ServerMessage::ClientDisconnected(self.id)).await;
                       }
                       Ok(n) => {
                           let bytes = buffer[..n].to_vec();
                           if bytes[0] == 48 {
                               self.server_send(ServerMessage::ClientDisconnected(self.id)).await;
                           }
                       }
                       Err(e) => {
                           error!("Error reading from {}: {}", self.id, e);
                       }
                   }
               }

               // Recieving messages over the server channel
               Some(msg) = self.rx.recv() => {
                   match msg {
                       ClientMessage::Send(bytes) => {
                           if let Err(e) = self.writer.write_all(&bytes).await {
                               error!("Error writing to {}: {}", self.id, e);
                               break;
                           }
                       }
                   }
               }
            }
        }
    }

    pub async fn server_send(&self, msg: ServerMessage) {
        if let Err(e) = self.tx.send(msg).await {
            error!("Error while communicating over server channel: {e}");
        }
    }
}
