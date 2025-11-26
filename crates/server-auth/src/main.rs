mod handler;

use std::sync::Arc;

use tc_core::{platform::SignalWaiter, server::Server};
use tracing::{error, info};

use crate::handler::{ServerPacketHandler, ServerState};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let waiter = SignalWaiter::new();

    waiter
        .wait(async {
            let server = Server::new(ServerPacketHandler, ServerState::new());
            server.run("127.0.0.1:8080".parse().unwrap()).await;
        })
        .await;

    info!("Cleaning up");
    Ok(())
}
