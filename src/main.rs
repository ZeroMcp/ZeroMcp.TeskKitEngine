use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

mod cli;
mod definition;
mod diff;
mod engine;
mod generator;
mod protocol;
mod recording;
mod transport;
mod validators;

use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => cli::run::execute(args).await,
        Command::Generate(args) => cli::generate::execute(args).await,
        Command::Diff(args) => cli::diff::execute(args).await,
    }
}
