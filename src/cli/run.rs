use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Args;

use crate::engine::executor::TestExecutor;
use crate::protocol::client::McpClient;
use crate::recording::recorder::RecordedSession;
use crate::recording::recording_transport::RecordingTransport;
use crate::recording::replay::ReplayTransport;
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

    /// Enable protocol validation (handshake + JSON-RPC frame checks)
    #[arg(long)]
    pub validate_protocol: bool,

    /// Enable tool metadata validation
    #[arg(long)]
    pub validate_metadata: bool,

    /// Auto-generate error-path tests (unknown tool, malformed params)
    #[arg(long)]
    pub auto_error_tests: bool,
}

pub async fn execute(args: RunArgs) -> Result<()> {
    tracing::info!(file = %args.file.display(), "Loading test definition");

    let mut definition = crate::definition::parser::load_from_file(&args.file)?;
    tracing::info!(
        tests = definition.tests.len(),
        "Loaded test definition v{}",
        definition.version
    );

    let config = definition.config.get_or_insert_with(Default::default);
    if let Some(timeout) = args.timeout_ms {
        config.timeout_ms = timeout;
    }
    if args.validate_protocol {
        config.validate_protocol = true;
    }
    if args.validate_metadata {
        config.validate_metadata = true;
    }
    if args.auto_error_tests {
        config.auto_error_tests = true;
    }

    // --- Replay mode ---
    if let Some(ref replay_path) = args.replay {
        tracing::info!(path = %replay_path.display(), "Replaying recorded session");
        let session = RecordedSession::load_from_file(replay_path).context(format!(
            "Failed to load recording from {}",
            replay_path.display()
        ))?;
        eprintln!("Replaying session recorded from {}", session.server);

        let replay = ReplayTransport::from_session(session);
        let mut client = McpClient::new(Box::new(replay));

        let init_result = client
            .initialize()
            .await
            .context("MCP handshake failed during replay")?;

        eprintln!(
            "Replayed handshake: {} v{} (protocol {})",
            init_result.server_info.name,
            init_result.server_info.version,
            init_result.protocol_version
        );

        let executor = TestExecutor::new(definition);
        let result = executor
            .run(&mut client, Some(&init_result))
            .await
            .context("Test execution failed during replay")?;

        client.close().await.ok();

        return output_results(&result, &args.output);
    }

    // --- Live server mode ---
    let server_url = args.server.as_deref().unwrap_or(&definition.server);
    tracing::info!(server = %server_url, "Connecting to MCP server");

    let inner_transport = transport::create_transport(server_url)
        .await
        .context(format!("Failed to create transport for '{}'", server_url))?;

    let recording = args.record.is_some();
    let transport: Box<dyn transport::McpTransport> = if recording {
        Box::new(RecordingTransport::wrap(inner_transport, server_url))
    } else {
        inner_transport
    };

    let mut client = McpClient::new(transport);

    let init_result = client.initialize().await.context("MCP handshake failed")?;

    eprintln!(
        "Connected to {} v{} (protocol {})",
        init_result.server_info.name, init_result.server_info.version, init_result.protocol_version
    );

    let executor = TestExecutor::new(definition);
    let result = executor
        .run(&mut client, Some(&init_result))
        .await
        .context("Test execution failed")?;

    // Save recording before closing
    if let Some(ref record_path) = args.record {
        save_recording(&mut client, record_path)?;
    }

    client.close().await.ok();

    output_results(&result, &args.output)
}

fn output_results(
    result: &crate::engine::result::TestRunResult,
    output_path: &Option<PathBuf>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;

    if let Some(path) = output_path {
        std::fs::write(path, &json)
            .context(format!("Failed to write results to {}", path.display()))?;
        eprintln!("Results written to {}", path.display());
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

fn save_recording(client: &mut McpClient, record_path: &std::path::Path) -> Result<()> {
    let transport_any = client.transport_as_any();
    if let Some(recording) = transport_any.downcast_ref::<RecordingTransport>() {
        let session = recording.to_session();
        session.save_to_file(record_path).context(format!(
            "Failed to save recording to {}",
            record_path.display()
        ))?;
        eprintln!("Session recorded to {}", record_path.display());
    }
    Ok(())
}
