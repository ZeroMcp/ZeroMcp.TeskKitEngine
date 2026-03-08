use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::protocol::jsonrpc::JsonRpcMessage;

use super::{McpTransport, TransportError};

/// Stdio transport: launches the MCP server as a subprocess and communicates
/// via newline-delimited JSON-RPC over stdin/stdout.
pub struct StdioTransport {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl StdioTransport {
    /// Spawn a subprocess from a shell command string and set up stdio pipes.
    pub async fn spawn(command: &str) -> Result<Self, TransportError> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(TransportError::ConnectionFailed(
                "Empty command string".to_string(),
            ));
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to spawn '{}': {}", command, e))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to capture child stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to capture child stdout".to_string())
        })?;

        let reader = BufReader::new(stdout);

        tracing::info!(command = %command, "Spawned MCP server subprocess");

        Ok(Self {
            child,
            stdin,
            reader,
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        let json = serde_json::to_string(message)?;
        tracing::debug!(msg = %json, "-> stdio");
        self.stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self
                .reader
                .read_line(&mut line)
                .await
                .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;

            if bytes_read == 0 {
                return Err(TransportError::Closed);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            tracing::debug!(msg = %trimmed, "<- stdio");
            let message: JsonRpcMessage = serde_json::from_str(trimmed)?;
            return Ok(message);
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        let _ = self.child.kill().await;
        tracing::info!("Closed stdio transport");
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
