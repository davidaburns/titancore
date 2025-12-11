mod cli;
mod error;
mod global_handlers;
mod routes;
mod sql;

use crate::{cli::CliArgs, global_handlers::handle_404};
use axum::{
    Router,
    routing::{get, post},
};
use clap::Parser;
use std::sync::Arc;
use tc_core::{
    database::{DatabaseHandle, PoolConfig},
    platform::SignalWaiter,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("TitanCore Api v{}", env!("CARGO_PKG_VERSION"));

    let args = CliArgs::parse();
    let db_config = PoolConfig {
        connection_string: args.db_connection_str.clone(),
        ..Default::default()
    };

    tracing::info!("Connecting to database...");
    let db = Arc::new(DatabaseHandle::connect(db_config).await?);

    let waiter = SignalWaiter::new();
    waiter
        .wait(async move {
            let app = Router::new()
                .route("/", get(routes::index::get_index))
                .route("/account", post(routes::account::create_account))
                .fallback(handle_404)
                .with_state(db);

            let listener = TcpListener::bind(format!("{}:{}", args.host, args.port))
                .await
                .unwrap();

            tracing::info!("Listening on: {}", listener.local_addr().unwrap());
            axum::serve(listener, app).await.unwrap();
        })
        .await;

    tracing::info!("Cleaning up");
    Ok(())
}
