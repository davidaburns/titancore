use core::platform::SignalWaiter;
use core::server::Server;
use tracing::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut server = Server::new();
    let waiter = SignalWaiter::new();

    waiter
        .wait(async {
            server.listen("0.0.0.0", 8080).await.unwrap();
        })
        .await;

    info!("Cleaning up");
    server.shutdown().await;
}
