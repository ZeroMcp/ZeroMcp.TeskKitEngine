use async_trait::async_trait;
use std::collections::VecDeque;

use crate::protocol::jsonrpc::JsonRpcMessage;

use super::{McpTransport, TransportError};

/// A scriptable mock transport for unit testing.
///
/// Queue up responses with `push_response` before calling code that
/// uses the transport. Sent messages are captured for assertions.
pub struct MockTransport {
    responses: VecDeque<Result<JsonRpcMessage, TransportError>>,
    pub sent_messages: Vec<JsonRpcMessage>,
    closed: bool,
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            responses: VecDeque::new(),
            sent_messages: Vec::new(),
            closed: false,
        }
    }

    /// Queue a successful response to be returned by the next `receive()` call.
    pub fn push_response(&mut self, message: JsonRpcMessage) {
        self.responses.push_back(Ok(message));
    }

    /// Queue an error to be returned by the next `receive()` call.
    pub fn push_error(&mut self, error: TransportError) {
        self.responses.push_back(Err(error));
    }

    /// Return the number of queued responses not yet consumed.
    pub fn pending_responses(&self) -> usize {
        self.responses.len()
    }

    /// Get the last sent message (most recent `send()` call).
    pub fn last_sent(&self) -> Option<&JsonRpcMessage> {
        self.sent_messages.last()
    }
}

#[async_trait]
impl McpTransport for MockTransport {
    async fn send(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        self.sent_messages.push(message.clone());
        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        self.responses
            .pop_front()
            .unwrap_or(Err(TransportError::Closed))
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Helper: build a JSON-RPC success response for a given request id and result value.
pub fn success_response(id: i64, result: serde_json::Value) -> JsonRpcMessage {
    JsonRpcMessage::Response(crate::protocol::jsonrpc::JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: crate::protocol::jsonrpc::RequestId::Number(id),
        result: Some(result),
        error: None,
    })
}

/// Helper: build a JSON-RPC error response.
pub fn error_response(id: i64, code: i64, message: &str) -> JsonRpcMessage {
    JsonRpcMessage::Response(crate::protocol::jsonrpc::JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: crate::protocol::jsonrpc::RequestId::Number(id),
        result: None,
        error: Some(crate::protocol::jsonrpc::JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    })
}

/// Helper: build a standard MCP initialize result response.
pub fn init_response(id: i64) -> JsonRpcMessage {
    success_response(
        id,
        serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "mock-server",
                "version": "1.0.0"
            }
        }),
    )
}

/// Helper: build a tools/list response.
pub fn tools_list_response(id: i64, tools: serde_json::Value) -> JsonRpcMessage {
    success_response(id, serde_json::json!({ "tools": tools }))
}

/// Helper: build a tools/call success response with text content.
pub fn tool_call_response(id: i64, text: &str) -> JsonRpcMessage {
    success_response(
        id,
        serde_json::json!({
            "content": [{ "type": "text", "text": text }],
            "isError": false
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_send_and_receive() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));

        let msg = JsonRpcMessage::Request(crate::protocol::jsonrpc::JsonRpcRequest::new(
            1i64,
            "initialize",
            None,
        ));
        mock.send(&msg).await.unwrap();

        assert_eq!(mock.sent_messages.len(), 1);

        let resp = mock.receive().await.unwrap();
        assert!(matches!(resp, JsonRpcMessage::Response(_)));
    }

    #[tokio::test]
    async fn mock_closed_rejects() {
        let mut mock = MockTransport::new();
        mock.close().await.unwrap();

        let msg = JsonRpcMessage::Request(crate::protocol::jsonrpc::JsonRpcRequest::new(
            1i64, "test", None,
        ));
        let err = mock.send(&msg).await.unwrap_err();
        assert!(matches!(err, TransportError::Closed));
    }

    #[tokio::test]
    async fn mock_empty_receive_returns_closed() {
        let mut mock = MockTransport::new();
        let err = mock.receive().await.unwrap_err();
        assert!(matches!(err, TransportError::Closed));
    }
}
