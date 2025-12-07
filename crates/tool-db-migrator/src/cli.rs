use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(
        short('c'),
        long("conn"),
        env("TC_DATABASE_CONNECTION"),
        help("Connection string to the database migrations are ran against"),
        required = true
    )]
    pub conn: String,

    #[arg(
        short('d'),
        long("dir"),
        env("TC_MIGRATION_DIR"),
        help("Directory migrations are stored at"),
        default_value = "./migrations"
    )]
    pub dir: String,

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
