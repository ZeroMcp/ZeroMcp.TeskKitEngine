use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;

use crate::generator::{known_good::Baseline, scaffold};
use crate::protocol::client::McpClient;
use crate::transport;

#[derive(Args)]
pub struct GenerateArgs {
    /// MCP server endpoint
    #[arg(short, long)]
    pub server: String,

    /// Generate scaffold stubs (one per tool with placeholder params)
    #[arg(long)]
    pub scaffold: bool,

    /// Generate known-good baseline from live responses
    #[arg(long)]
    pub known_good: bool,

    /// Tool parameters as tool_name:'{"key":"value"}' pairs
    #[arg(long, value_name = "TOOL:JSON")]
    pub params: Vec<String>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    pub out: Option<PathBuf>,
}

pub async fn execute(args: GenerateArgs) -> Result<()> {
    if !args.scaffold && !args.known_good {
        anyhow::bail!("Specify either --scaffold or --known-good");
    }

    tracing::info!(server = %args.server, "Connecting to MCP server");

    let transport = transport::create_transport(&args.server)
        .await
        .context(format!("Failed to connect to '{}'", args.server))?;

    let mut client = McpClient::new(transport);

    let init_result = client.initialize().await.context("MCP handshake failed")?;

    eprintln!(
        "Connected to {} v{} (protocol {})",
        init_result.server_info.name, init_result.server_info.version, init_result.protocol_version
    );

    let tools = client.tools_list().await.context("Failed to list tools")?;

    eprintln!("Discovered {} tool(s)", tools.len());
    for tool in &tools {
        eprintln!(
            "  - {} {}",
            tool.name,
            tool.description.as_deref().unwrap_or("")
        );
    }

    let json_output = if args.scaffold {
        let definition = scaffold::generate_scaffold(&args.server, &tools);
        serde_json::to_string_pretty(&definition)?
    } else {
        let param_map = parse_params(&args.params)?;
        let mut baseline = Baseline::new(&args.server);

        for tool in &tools {
            let tool_params = param_map
                .get(tool.name.as_str())
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            eprintln!("Calling tool '{}' ...", tool.name);

            match client.tools_call(&tool.name, tool_params.clone()).await {
                Ok(result) => {
                    let response_value = serde_json::to_value(&result)?;
                    baseline.add_entry(tool, tool_params, response_value);
                    if result.is_error {
                        eprintln!("  ⚠ tool returned isError: true");
                    } else {
                        eprintln!("  ✓ captured response");
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ failed: {:#}", e);
                    baseline.add_entry(
                        tool,
                        tool_params,
                        serde_json::json!({ "__error": format!("{:#}", e) }),
                    );
                }
            }
        }

        serde_json::to_string_pretty(&baseline)?
    };

    client.close().await.ok();

    if let Some(ref out_path) = args.out {
        std::fs::write(out_path, &json_output)
            .context(format!("Failed to write output to {}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        println!("{json_output}");
    }

    Ok(())
}

/// Parse --params values in the format `tool_name:{"key":"value"}`.
fn parse_params(raw: &[String]) -> Result<std::collections::HashMap<&str, Value>> {
    let mut map = std::collections::HashMap::new();

    for entry in raw {
        let (name, json_str) = entry.split_once(':').ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid --params format '{}'. Expected tool_name:{{\"key\":\"value\"}}",
                entry
            )
        })?;

        let value: Value = serde_json::from_str(json_str).context(format!(
            "Invalid JSON in --params for tool '{}': {}",
            name, json_str
        ))?;

        map.insert(name, value);
    }

    Ok(map)
}
