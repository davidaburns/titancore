use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, error::SendError},
    task::JoinHandle,
};

use tracing::{error, info};

pub struct Client {
    pub id: usize,
    pub tx: UnboundedSender<Vec<u8>>,
    pub read_task: JoinHandle<()>,
    pub write_task: JoinHandle<()>,
}

impl Client {
    pub fn send(&self, data: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.tx.send(data)
    }

    pub fn is_alive(&self) -> bool {
        !self.read_task.is_finished() && !self.write_task.is_finished()
    }

    pub fn disconnect(&self) {
        self.read_task.abort();
        self.write_task.abort();
    }
}

pub fn spawn_read_task(
    mut reader: OwnedReadHalf,
    tx: UnboundedSender<Vec<u8>>,
    dc_tx: UnboundedSender<usize>,
    client_id: usize,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1500];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    let bytes = buffer[..n].to_vec();
                    info!("From Client: {:?}", bytes);

                    if let Err(e) = tx.send(bytes) {
                        error!("Error sending data to be written to client: {e}");
                    }
                }
                Err(e) => {
                    error!("Error reading from client stream: {e}");
                }
            }
        }

        let _ = dc_tx.send(client_id);
    })
}

pub fn spawn_write_task(
    mut writer: OwnedWriteHalf,
    mut rx: UnboundedReceiver<Vec<u8>>,
    dc_tx: UnboundedSender<usize>,
    client_id: usize,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            if let Err(e) = writer.write_all(&packet).await {
                error!("Client {} write error: {}", client_id, e);
            }
        }

        let _ = dc_tx.send(client_id);
    })
}
