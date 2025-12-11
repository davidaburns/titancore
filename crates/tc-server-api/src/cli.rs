use clap::Parser;

#[derive(Parser)]
pub struct CliArgs {
    #[arg(
        long("db"),
        env("TC_DATABASE_CONNECTION"),
        help("Connection string to the database the api will run against"),
        required = true
    )]
    pub db_connection_str: String,

    #[arg(
        long("host"),
        env("TC_API_HOST"),
        help("The host ip that the api will listen on"),
        default_value = "0.0.0.0"
    )]
    pub host: String,

    #[arg(
        long("port"),
        env("TC_API_PORT"),
        help("The port that the api will open on"),
        default_value = "3000"
    )]
    pub port: u16,
}
