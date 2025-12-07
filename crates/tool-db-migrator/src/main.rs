mod cli;
mod db;

use crate::cli::{CliArgs, CliSubCommand};
use clap::Parser;
use tc_core::database::{DatabaseHandle, MigrationMigrator, MigrationRegistry, PoolConfig};
use tokio::fs::DirBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = CliArgs::parse();
    if args.create_db {
        let db_name = db::database_from_connection_string(&args.conn)?;
        if !db::database_exists(&args.conn, &db_name).await? {
            db::create_database(&args.conn, &db_name).await?;
        }
    }

    let config = PoolConfig {
        connection_string: args.conn.clone(),
        ..Default::default()
    };

    let db = DatabaseHandle::connect(config).await?;
    let registry = MigrationRegistry::from_dir(args.dir.clone()).await?;
    let migrator = MigrationMigrator::new(&db, &registry);

    match args.cmd {
        CliSubCommand::Status => {
            if !migrator.initialized().await? {
                migrator.init().await?;
            }

            let current = migrator.current_version().await?.unwrap_or(0);
            let pending = migrator.pending().await?;

            tracing::info!("Current version: {}", current);
            tracing::info!("Pending migrations: {}", pending.len());
            for m in &pending {
                tracing::info!("  - {}: {}", m.version, m.name);
            }
        }
        CliSubCommand::Up => {
            if !migrator.initialized().await? {
                migrator.init().await?;
            }

            let pending = migrator.pending().await?;
            tracing::info!("Pending migrations: {}", pending.len());
            for m in &pending {
                tracing::info!("  - {}: {}", m.version, m.name);
            }

            let report = migrator.migrate_pending().await?;
            tracing::info!("Initial Version: {}", report.initial_version);
            tracing::info!("Final Version: {}", report.final_version);
            tracing::info!("Applied: {}", report.applied.len());
            tracing::info!("Reverted: {}", report.reverted.len());
        }
        CliSubCommand::To { version } => {
            if !migrator.initialized().await? {
                migrator.init().await?;
            }

            tracing::info!("Migrating database to version: {}", version);

            let report = migrator.migrate_to(version).await?;
            tracing::info!("Initial Version: {}", report.initial_version);
            tracing::info!("Final Version: {}", report.final_version);
            tracing::info!("Applied: {}", report.applied.len());
            tracing::info!("Reverted: {}", report.reverted.len());
        }
        CliSubCommand::New { name } => {
            let mut builder = DirBuilder::new();
            builder.recursive(true);
            builder.create(args.dir.clone()).await?;

            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let up_filename = format!("{}/{}_{}.up.sql", args.dir, timestamp, name);
            let down_filename = format!("{}/{}_{}.down.sql", args.dir, timestamp, name);

            tokio::fs::write(&up_filename, "-- Add up migration SQL here\n").await?;
            tokio::fs::write(&down_filename, "-- Add down migration SQL here\n").await?;

            tracing::info!("Created new migration file: {}", up_filename);
            tracing::info!("Created new migration file: {}", down_filename);
        }
    }

    Ok(())
}
