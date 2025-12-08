mod cli;
mod db;

use crate::cli::{CliArgs, CliSubCommand};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .without_time()
        .with_level(false)
        .with_target(false)
        .with_file(false)
        .init();

    let args = CliArgs::parse();
    match args.cmd {
        CliSubCommand::Status | CliSubCommand::Up | CliSubCommand::To { .. } => {
            let (conn, dir) = match (args.conn, args.dir) {
                (Some(conn), Some(dir)) => (conn, dir),
                _ => {
                    tracing::error!("Error: connection or migration directory not provided");
                    tracing::error!("Usage: --conn <CONN> --dir <DIR> [status | up | to]");
                    std::process::exit(1);
                }
            };

            cli::run_migration_cmd(args.cmd, conn, dir, args.create_db).await?;
            Ok(())
        }
        CliSubCommand::New { name } => {
            let dir = match args.dir {
                Some(dir) => dir,
                None => {
                    tracing::error!("Error: migration directory not provided");
                    tracing::error!("Usage: --dir <DIR> new <NAME>");
                    std::process::exit(1);
                }
            };

            cli::run_new_cmd(name, dir).await?;
            Ok(())
        }
    }
}
