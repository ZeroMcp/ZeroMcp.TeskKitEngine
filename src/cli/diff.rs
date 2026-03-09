use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use crate::diff::baseline::diff_tools;
use crate::generator::known_good::Baseline;
use crate::protocol::client::McpClient;
use crate::transport;

#[derive(Args)]
pub struct DiffArgs {
    /// Path to the baseline file (from generate --known-good)
    #[arg(short, long)]
    pub baseline: PathBuf,

    /// MCP server endpoint to compare against
    #[arg(short, long)]
    pub server: String,

    /// Output file for diff results (defaults to stdout for JSON, stderr for human-readable)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub async fn execute(args: DiffArgs) -> Result<()> {
    tracing::info!(baseline = %args.baseline.display(), server = %args.server, "Running baseline diff");

    let baseline_json = std::fs::read_to_string(&args.baseline).context(format!(
        "Failed to read baseline: {}",
        args.baseline.display()
    ))?;
    let baseline: Baseline =
        serde_json::from_str(&baseline_json).context("Failed to parse baseline file")?;

    let baseline_tools: Vec<crate::protocol::mcp::Tool> = baseline
        .entries
        .iter()
        .map(|e| crate::protocol::mcp::Tool {
            name: e.tool_name.clone(),
            description: None,
            input_schema: e.input_schema.clone(),
            annotations: None,
        })
        .collect();

    let transport = transport::create_transport(&args.server)
        .await
        .context(format!("Failed to connect to '{}'", args.server))?;

    let mut client = McpClient::new(transport);

    let init_result = client.initialize().await.context("MCP handshake failed")?;
    eprintln!(
        "Connected to {} v{} (protocol {})",
        init_result.server_info.name, init_result.server_info.version, init_result.protocol_version
    );

    let current_tools = client.tools_list().await.context("Failed to list tools")?;
    client.close().await.ok();

    let mut report = diff_tools(&baseline_tools, &current_tools);
    report.server = args.server.clone();
    report.baseline_server = baseline.server.clone();

    // Human-readable output to stderr
    if report.has_changes {
        eprintln!("\nChanges detected:");
        for name in &report.added_tools {
            eprintln!("  + ADDED:   {}", name);
        }
        for name in &report.removed_tools {
            eprintln!("  - REMOVED: {}", name);
        }
        for change in &report.changed_tools {
            eprintln!("  ~ CHANGED: {}", change.tool_name);
            for detail in &change.changes {
                eprintln!("      {}", detail);
            }
        }
    } else {
        eprintln!("\nNo changes detected — server matches baseline.");
    }

    // JSON output
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &json)
            .context(format!("Failed to write diff to {}", output_path.display()))?;
        eprintln!("Diff written to {}", output_path.display());
    } else {
        println!("{json}");
    }

    if report.has_changes {
        std::process::exit(1);
    }

    Ok(())
}
