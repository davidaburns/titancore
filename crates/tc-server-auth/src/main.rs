#[allow(dead_code, unused)]
mod handler;
mod opcode;
mod packets;

use crate::handler::{AuthServer, ServerState};
use anyhow::Result;
use tc_core::{platform::SignalWaiter, server::Server};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let waiter = SignalWaiter::new();
    waiter
        .wait(async {
            tracing_subscriber::fmt::init();
            tracing::info!("TitanCore v{}", env!("CARGO_PKG_VERSION"));

            let server = Server::new(AuthServer, ServerState::new());
            if let Err(e) = server.run("127.0.0.1:3724".parse().unwrap()).await {
                tracing::error!("Error while running server: {e}");
            }
        })
        .await;

    tracing::info!("Cleaning up");
    Ok(())
}
