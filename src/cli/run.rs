use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Args;

use crate::engine::executor::TestExecutor;
use crate::protocol::client::McpClient;
use crate::transport;

#[derive(Args)]
pub struct RunArgs {
    /// Path to the test definition JSON file
    #[arg(short, long)]
    pub file: PathBuf,

    /// MCP server endpoint (http://..., ws://..., or a command for stdio).
    /// Overrides the server field in the test definition.
    #[arg(short, long)]
    pub server: Option<String>,

    /// Path to write the session recording
    #[arg(long)]
    pub record: Option<PathBuf>,

    /// Path to a recorded session to replay instead of connecting to a live server
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// Output file for results (defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Timeout per test case in milliseconds (overrides definition config)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

pub async fn execute(args: RunArgs) -> Result<()> {
    tracing::info!(file = %args.file.display(), "Loading test definition");

    let mut definition = crate::definition::parser::load_from_file(&args.file)?;
    tracing::info!(
        tests = definition.tests.len(),
        "Loaded test definition v{}",
        definition.version
    );

    if let Some(timeout) = args.timeout_ms {
        let config = definition.config.get_or_insert_with(Default::default);
        config.timeout_ms = timeout;
    }

    let server_url = args
        .server
        .as_deref()
        .unwrap_or(&definition.server);

    tracing::info!(server = %server_url, "Connecting to MCP server");

    let transport = transport::create_transport(server_url)
        .await
        .context(format!("Failed to create transport for '{}'", server_url))?;

    let mut client = McpClient::new(transport);

    let init_result = client
        .initialize()
        .await
        .context("MCP handshake failed")?;

    eprintln!(
        "Connected to {} v{} (protocol {})",
        init_result.server_info.name,
        init_result.server_info.version,
        init_result.protocol_version
    );

    let executor = TestExecutor::new(definition);
    let result = executor
        .run(&mut client)
        .await
        .context("Test execution failed")?;

    client.close().await.ok();

    let json = serde_json::to_string_pretty(&result)?;

    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &json)
            .context(format!("Failed to write results to {}", output_path.display()))?;
        eprintln!("Results written to {}", output_path.display());
    } else {
        println!("{json}");
    }

    let passed = result.results.iter().filter(|r| r.passed).count();
    let failed = result.results.iter().filter(|r| !r.passed).count();
    eprintln!(
        "\n{} passed, {} failed ({} total) in {}ms",
        passed,
        failed,
        result.results.len(),
        result.elapsed_ms
    );

    let exit_code = result.exit_code();
    if exit_code != 0 {
        process::exit(exit_code);
    }

    Ok(())
}
