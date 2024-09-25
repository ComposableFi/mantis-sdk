use clap::{Parser};
use dotenv::dotenv;
use mantis_sdk::cmd::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let cli = Cli::parse();
    cli.run().await
}
