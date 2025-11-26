mod handler;

use crate::handler::{AuthServer, ServerState};
use anyhow::Result;
use tc_core::{platform::SignalWaiter, server::Server};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let waiter = SignalWaiter::new();
    waiter
        .wait(async {
            let server = Server::new(AuthServer, ServerState::new());
            if let Err(e) = server.run("127.0.0.1:8080".parse().unwrap()).await {
                tracing::error!("Error while running server: {e}");
            }
        })
        .await;

    tracing::info!("Cleaning up");
    Ok(())
}
