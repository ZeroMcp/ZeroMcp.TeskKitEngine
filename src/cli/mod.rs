pub mod diff;
pub mod generate;
pub mod run;

use clap::Parser;

#[derive(Parser)]
#[command(name = "mcptest")]
#[command(about = "Universal testing toolkit for MCP servers")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Run tests from a test definition file against an MCP server
    Run(run::RunArgs),
    /// Generate test definitions from a live MCP server
    Generate(generate::GenerateArgs),
    /// Compare a baseline against the current server state
    Diff(diff::DiffArgs),
}
