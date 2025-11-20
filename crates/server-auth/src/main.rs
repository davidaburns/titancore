use core::platform::SignalWaiter;
use core::server::run_server;
use tracing::{error, info};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let waiter = SignalWaiter::new();

    waiter
        .wait(async {
            if let Err(e) = run_server("0.0.0.0", 8080).await {
                error!("Error while running the server: {e}");
            }
        })
        .await;

    info!("Cleaning up");
}
