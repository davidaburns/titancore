use crate::db;
use clap::{Parser, Subcommand};
use tc_core::database::{
    DatabaseHandle, MigrationMigrator, MigrationRegistry, MigrationReport, PoolConfig,
};
use tokio::fs::DirBuilder;

#[derive(Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(
        short('c'),
        long("conn"),
        env("TC_DATABASE_CONNECTION"),
        help("Connection string to the database migrations are ran against")
    )]
    pub conn: Option<String>,

    #[arg(
        short('d'),
        long("dir"),
        env("TC_MIGRATION_DIR"),
        help("Directory migrations are stored at"),
        default_value = "./migrations"
    )]
    pub dir: Option<String>,

    #[arg(long("create"), help("Create the database if it does not exist"))]
    pub create_db: bool,

    #[command(subcommand)]
    pub cmd: CliSubCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CliSubCommand {
    #[command(about("Check the status of migrations against the database"))]
    Status,

    #[command(about("Migrate all pending migrations that have not been applied"))]
    Up,

    #[command(about("Migrate the database to a speicif version (up or down)"))]
    To { version: i64 },

    #[command(about("Create new migration file with the specified name"))]
    New { name: String },
}

pub async fn run_migration_cmd(
    cmd: CliSubCommand,
    conn: String,
    dir: String,
    create_db: bool,
) -> anyhow::Result<()> {
    if create_db {
        let db_name = db::database_from_connection_string(&conn)?;
        if !db::database_exists(&conn, &db_name).await? {
            db::create_database(&conn, &db_name).await?;
        }
    }

    let config = PoolConfig {
        connection_string: conn.clone(),
        ..Default::default()
    };

    let db = DatabaseHandle::connect(config).await?;
    let registry = MigrationRegistry::from_dir(dir.clone()).await?;
    let migrator = MigrationMigrator::new(&db, &registry);

    if !migrator.initialized().await? {
        migrator.init().await?;
    }

    match cmd {
        CliSubCommand::Status => {
            let current = migrator.current_version().await?.unwrap_or(0);
            let pending = migrator.pending().await?;

            tracing::info!("Current version: {}", current);
            tracing::info!("Pending migrations: {}", pending.len());
            for m in &pending {
                tracing::info!("  - {}: {}", m.version, m.name);
            }
        }
        CliSubCommand::Up => {
            let pending = migrator.pending().await?;
            tracing::info!("Pending migrations: {}", pending.len());
            for m in &pending {
                tracing::info!("  - {}: {}", m.version, m.name);
            }

            let report = migrator.migrate_pending().await?;
            print_migration_report(report);
        }
        CliSubCommand::To { version } => {
            tracing::info!("Migrating database to version: {}", version);
            let report = migrator.migrate_to(version).await?;
            print_migration_report(report);
        }
        _ => {}
    }

    Ok(())
}

pub async fn run_new_cmd(name: String, dir: String) -> anyhow::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(dir.clone()).await?;

    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let up_filename = format!("{}/{}_{}.up.sql", dir, timestamp, name);
    let down_filename = format!("{}/{}_{}.down.sql", dir, timestamp, name);

    tokio::fs::write(&up_filename, "-- Add up migration SQL here\n").await?;
    tokio::fs::write(&down_filename, "-- Add down migration SQL here\n").await?;

    tracing::info!("Created new migration file: {}", up_filename);
    tracing::info!("Created new migration file: {}", down_filename);

    Ok(())
}

fn print_migration_report(report: MigrationReport) {
    tracing::info!("Initial Version: {}", report.initial_version);
    tracing::info!("Final Version: {}", report.final_version);
    tracing::info!("Applied: {}", report.applied.len());
    tracing::info!("Reverted: {}", report.reverted.len());
}
