pub mod http;
pub mod mock;
pub mod stdio;

use async_trait::async_trait;
use thiserror::Error;

use crate::protocol::jsonrpc::JsonRpcMessage;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Transport closed")]
    Closed,

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Transport-agnostic interface for communicating with an MCP server.
///
/// Implementations handle the wire format (newline-delimited JSON for stdio,
/// HTTP POST/SSE for Streamable HTTP) while the protocol layer above works
/// purely with `JsonRpcMessage` values.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a JSON-RPC message to the server.
    async fn send(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError>;

    /// Receive the next JSON-RPC message from the server.
    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError>;

    /// Close the transport connection.
    async fn close(&mut self) -> Result<(), TransportError>;

    /// Downcast support for testing — allows inspecting the concrete transport type.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Determine the transport type from a server URL string.
pub fn parse_server_url(server: &str) -> TransportKind {
    if server.starts_with("http://") || server.starts_with("https://") {
        TransportKind::Http(server.to_string())
    } else if server.starts_with("ws://") || server.starts_with("wss://") {
        TransportKind::WebSocket(server.to_string())
    } else if let Some(cmd) = server.strip_prefix("stdio:") {
        TransportKind::Stdio(cmd.to_string())
    } else {
        TransportKind::Stdio(server.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum TransportKind {
    Http(String),
    WebSocket(String),
    Stdio(String),
}

/// Create a transport from a server URL string.
pub async fn create_transport(server: &str) -> Result<Box<dyn McpTransport>, TransportError> {
    match parse_server_url(server) {
        TransportKind::Http(url) => {
            let transport = http::HttpTransport::new(&url)?;
            Ok(Box::new(transport))
        }
        TransportKind::Stdio(cmd) => {
            let transport = stdio::StdioTransport::spawn(&cmd).await?;
            Ok(Box::new(transport))
        }
        TransportKind::WebSocket(url) => Err(TransportError::ConnectionFailed(format!(
            "WebSocket transport not yet implemented for {}",
            url
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_url() {
        assert!(matches!(
            parse_server_url("http://localhost:8000/mcp"),
            TransportKind::Http(_)
        ));
    }

    #[test]
    fn parse_https_url() {
        assert!(matches!(
            parse_server_url("https://example.com/mcp"),
            TransportKind::Http(_)
        ));
    }

    #[test]
    fn parse_stdio_explicit() {
        match parse_server_url("stdio:python server.py") {
            TransportKind::Stdio(cmd) => assert_eq!(cmd, "python server.py"),
            other => panic!("Expected Stdio, got {other:?}"),
        }
    }

    #[test]
    fn parse_stdio_implicit() {
        assert!(matches!(
            parse_server_url("python server.py"),
            TransportKind::Stdio(_)
        ));
    }

    #[test]
    fn parse_websocket() {
        assert!(matches!(
            parse_server_url("ws://localhost:8000/mcp"),
            TransportKind::WebSocket(_)
        ));
    }
}
