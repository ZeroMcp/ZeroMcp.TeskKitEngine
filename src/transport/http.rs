use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::protocol::jsonrpc::JsonRpcMessage;

use super::{McpTransport, TransportError};

/// Streamable HTTP transport: sends JSON-RPC requests via HTTP POST and
/// receives responses. Handles both direct JSON and SSE responses.
pub struct HttpTransport {
    client: Client,
    endpoint: String,
    session_id: Option<String>,
    response_rx: mpsc::UnboundedReceiver<JsonRpcMessage>,
    response_tx: mpsc::UnboundedSender<JsonRpcMessage>,
}

impl HttpTransport {
    pub fn new(endpoint: &str) -> Result<Self, TransportError> {
        let client = Client::builder()
            .build()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let (response_tx, response_rx) = mpsc::unbounded_channel();

        tracing::info!(endpoint = %endpoint, "Created HTTP transport");

        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            session_id: None,
            response_rx,
            response_tx,
        })
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        let is_notification = matches!(message, JsonRpcMessage::Notification(_));

        let mut builder = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(ref session_id) = self.session_id {
            builder = builder.header("Mcp-Session", session_id);
        }

        builder = builder.json(message);

        let msg_json = serde_json::to_string(message).unwrap_or_default();
        tracing::debug!(endpoint = %self.endpoint, body = %msg_json, "-> HTTP POST");

        let response = builder
            .send()
            .await
            .map_err(|e| TransportError::SendFailed(format!("{}: {}", self.endpoint, e)))?;

        let status = response.status();
        tracing::debug!(status = %status, "HTTP response status");

        if let Some(session_id) = response.headers().get("mcp-session") {
            if let Ok(id) = session_id.to_str() {
                self.session_id = Some(id.to_string());
                tracing::debug!(session_id = %id, "Captured session ID");
            }
        }

        if is_notification {
            if status.is_success() {
                tracing::debug!("Notification accepted");
            } else {
                let body = response.text().await.unwrap_or_default();
                return Err(TransportError::SendFailed(format!(
                    "Notification rejected with HTTP {}: {}",
                    status, body
                )));
            }
            return Ok(());
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(TransportError::ReceiveFailed(format!(
                "HTTP {} from {}: {}",
                status, self.endpoint, body
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;

        if body.is_empty() {
            return Ok(());
        }

        tracing::debug!(body = %body, content_type = %content_type, "<- HTTP response");

        if content_type.contains("text/event-stream") {
            for msg in parse_sse_events(&body) {
                self.response_tx
                    .send(msg)
                    .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;
            }
        } else {
            let msg: JsonRpcMessage = serde_json::from_str(&body).map_err(|e| {
                TransportError::ReceiveFailed(format!(
                    "Failed to parse response: {} — body: {}",
                    e, body
                ))
            })?;
            self.response_tx
                .send(msg)
                .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;
        }

        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError> {
        self.response_rx.recv().await.ok_or(TransportError::Closed)
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(ref session_id) = self.session_id {
            let _ = self
                .client
                .delete(&self.endpoint)
                .header("Mcp-Session", session_id)
                .send()
                .await;
            tracing::info!(session_id = %session_id, "Closed HTTP session");
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Extract JSON-RPC messages from an SSE response body.
/// SSE format: lines prefixed with "data: " separated by blank lines.
fn parse_sse_events(body: &str) -> Vec<JsonRpcMessage> {
    let mut messages = Vec::new();
    let mut data_buf = String::new();

    for line in body.lines() {
        if let Some(data) = line
            .strip_prefix("data: ")
            .or_else(|| line.strip_prefix("data:"))
        {
            let data = data.trim();
            if !data.is_empty() {
                data_buf.push_str(data);
            }
        } else if line.trim().is_empty() && !data_buf.is_empty() {
            match serde_json::from_str::<JsonRpcMessage>(&data_buf) {
                Ok(msg) => messages.push(msg),
                Err(e) => tracing::warn!(data = %data_buf, error = %e, "Failed to parse SSE event"),
            }
            data_buf.clear();
        }
    }

    // Handle trailing data without a final blank line
    if !data_buf.is_empty() {
        match serde_json::from_str::<JsonRpcMessage>(&data_buf) {
            Ok(msg) => messages.push(msg),
            Err(e) => {
                tracing::warn!(data = %data_buf, error = %e, "Failed to parse trailing SSE event")
            }
        }
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_single_event() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let msgs = parse_sse_events(body);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn parse_sse_multiple_events() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n";
        let msgs = parse_sse_events(body);
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn parse_sse_no_trailing_newline() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}";
        let msgs = parse_sse_events(body);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn parse_sse_empty_body() {
        let msgs = parse_sse_events("");
        assert!(msgs.is_empty());
    }
}
