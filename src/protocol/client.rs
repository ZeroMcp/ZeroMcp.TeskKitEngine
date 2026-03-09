use anyhow::{Context, Result};
use serde_json::Value;

use crate::protocol::jsonrpc::{
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use crate::protocol::mcp::{
    InitializeParams, InitializeResult, METHOD_INITIALIZE, METHOD_INITIALIZED, METHOD_TOOLS_CALL,
    METHOD_TOOLS_LIST, Tool, ToolCallParams, ToolCallResult, ToolsListResult,
};
use crate::protocol::session::Session;
use crate::transport::McpTransport;

/// High-level MCP client that wraps a transport and manages the session lifecycle.
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    session: Session,
}

impl McpClient {
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self {
            transport,
            session: Session::new(),
        }
    }

    /// Perform the full MCP initialize handshake:
    /// 1. Send `initialize` request
    /// 2. Receive and validate the response
    /// 3. Send `notifications/initialized` notification
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        self.session
            .transition_to_initializing()
            .context("Failed to start initialization")?;

        let params = InitializeParams::for_mcptest();
        let id = self.session.next_request_id();
        let request =
            JsonRpcRequest::new(id, METHOD_INITIALIZE, Some(serde_json::to_value(&params)?));

        tracing::info!("Sending initialize request");
        self.transport
            .send(&JsonRpcMessage::Request(request))
            .await
            .context("Failed to send initialize request")?;

        let response = self
            .transport
            .receive()
            .await
            .context("Failed to receive initialize response")?;

        let init_result = self.extract_result::<InitializeResult>(response, "initialize")?;

        tracing::info!(
            server = %init_result.server_info.name,
            version = %init_result.server_info.version,
            protocol = %init_result.protocol_version,
            "Server identified"
        );

        let notif = JsonRpcNotification::new(METHOD_INITIALIZED, None);
        self.transport
            .send(&JsonRpcMessage::Notification(notif))
            .await
            .context("Failed to send initialized notification")?;

        self.session
            .transition_to_ready(init_result.clone())
            .context("Failed to transition to ready")?;

        tracing::info!("MCP session ready");
        Ok(init_result)
    }

    /// Call `tools/list` and return all tools, handling pagination.
    pub async fn tools_list(&mut self) -> Result<Vec<Tool>> {
        self.session
            .ensure_ready("tools/list")
            .context("Session not ready for tools/list")?;

        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut params = serde_json::Map::new();
            if let Some(ref c) = cursor {
                params.insert("cursor".to_string(), Value::String(c.clone()));
            }

            let id = self.session.next_request_id();
            let request = JsonRpcRequest::new(id, METHOD_TOOLS_LIST, Some(Value::Object(params)));

            tracing::debug!(cursor = ?cursor, "Requesting tools/list");
            self.transport
                .send(&JsonRpcMessage::Request(request))
                .await
                .context("Failed to send tools/list request")?;

            let response = self
                .transport
                .receive()
                .await
                .context("Failed to receive tools/list response")?;

            let result = self.extract_result::<ToolsListResult>(response, "tools/list")?;
            let page_count = result.tools.len();
            all_tools.extend(result.tools);

            tracing::debug!(
                tools_in_page = page_count,
                total = all_tools.len(),
                "Received tools page"
            );

            match result.next_cursor {
                Some(next) if !next.is_empty() => cursor = Some(next),
                _ => break,
            }
        }

        tracing::info!(count = all_tools.len(), "Discovered tools");
        Ok(all_tools)
    }

    /// Call a single tool via `tools/call`.
    pub async fn tools_call(&mut self, name: &str, arguments: Value) -> Result<ToolCallResult> {
        self.session
            .ensure_ready("tools/call")
            .context("Session not ready for tools/call")?;

        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };

        let id = self.session.next_request_id();
        let request =
            JsonRpcRequest::new(id, METHOD_TOOLS_CALL, Some(serde_json::to_value(&params)?));

        tracing::debug!(tool = %name, "Calling tool");
        self.transport
            .send(&JsonRpcMessage::Request(request))
            .await
            .context(format!("Failed to send tools/call for '{}'", name))?;

        let response = self.transport.receive().await.context(format!(
            "Failed to receive tools/call response for '{}'",
            name
        ))?;

        let result =
            self.extract_result::<ToolCallResult>(response, &format!("tools/call({})", name))?;
        tracing::debug!(
            tool = %name,
            is_error = result.is_error,
            content_items = result.content.len(),
            "Tool call complete"
        );

        Ok(result)
    }

    /// Send a raw JSON-RPC request and return the raw response.
    /// Used for error-path testing where we intentionally send malformed requests.
    pub async fn raw_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse> {
        let id = self.session.next_request_id();
        let request = JsonRpcRequest::new(id, method, params);

        self.transport
            .send(&JsonRpcMessage::Request(request))
            .await
            .context(format!("Failed to send raw request '{}'", method))?;

        let response = self
            .transport
            .receive()
            .await
            .context(format!("Failed to receive response for '{}'", method))?;

        match response {
            JsonRpcMessage::Response(resp) => Ok(resp),
            other => anyhow::bail!("Expected response for '{}', got: {:?}", method, other),
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        let _ = self.session.transition_to_closed();
        self.transport
            .close()
            .await
            .context("Failed to close transport")?;
        Ok(())
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Access the underlying transport for type-specific operations (e.g. extracting recordings).
    pub fn transport_as_any(&self) -> &dyn std::any::Any {
        self.transport.as_any()
    }

    fn extract_result<T: serde::de::DeserializeOwned>(
        &self,
        message: JsonRpcMessage,
        method: &str,
    ) -> Result<T> {
        match message {
            JsonRpcMessage::Response(resp) => {
                if let Some(error) = resp.error {
                    anyhow::bail!(
                        "Server returned error for '{}': [{}] {}{}",
                        method,
                        error.code,
                        error.message,
                        error.data.map(|d| format!(" — {}", d)).unwrap_or_default()
                    );
                }
                let result_value = resp
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response for '{}'", method))?;

                serde_json::from_value(result_value)
                    .context(format!("Failed to deserialize {} response", method))
            }
            other => anyhow::bail!("Expected response for '{}', got: {:?}", method, other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::*;

    fn make_client(mock: MockTransport) -> McpClient {
        McpClient::new(Box::new(mock))
    }

    #[tokio::test]
    async fn initialize_handshake_succeeds() {
        let mut mock = MockTransport::new();
        // Response to initialize request (id=1)
        mock.push_response(init_response(1));

        let mut client = make_client(mock);
        let result = client.initialize().await.unwrap();

        assert_eq!(result.server_info.name, "mock-server");
        assert_eq!(result.protocol_version, "2025-11-25");
    }

    #[tokio::test]
    async fn initialize_server_error_propagates() {
        let mut mock = MockTransport::new();
        mock.push_response(error_response(1, -32600, "Bad request"));

        let mut client = make_client(mock);
        let err = client.initialize().await.unwrap_err();
        assert!(err.to_string().contains("Bad request"));
    }

    #[tokio::test]
    async fn tools_list_returns_tools() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([
                {
                    "name": "search",
                    "description": "Search stuff",
                    "inputSchema": { "type": "object", "properties": { "q": { "type": "string" } } }
                },
                {
                    "name": "echo",
                    "inputSchema": { "type": "object" }
                }
            ]),
        ));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();

        let tools = client.tools_list().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "search");
        assert_eq!(tools[1].name, "echo");
    }

    #[tokio::test]
    async fn tools_list_handles_pagination() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));

        // Page 1 with cursor
        mock.push_response(success_response(
            2,
            serde_json::json!({
                "tools": [{ "name": "tool_a", "inputSchema": {} }],
                "nextCursor": "page2"
            }),
        ));
        // Page 2 without cursor
        mock.push_response(tools_list_response(
            3,
            serde_json::json!([{ "name": "tool_b", "inputSchema": {} }]),
        ));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();

        let tools = client.tools_list().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "tool_a");
        assert_eq!(tools[1].name, "tool_b");
    }

    #[tokio::test]
    async fn tools_list_before_init_fails() {
        let mock = MockTransport::new();
        let mut client = make_client(mock);

        let err = client.tools_list().await.unwrap_err();
        assert!(err.to_string().contains("not ready"));
    }

    #[tokio::test]
    async fn tools_call_returns_result() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tool_call_response(2, "hello world"));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();

        let result = client
            .tools_call("echo", serde_json::json!({"text": "hello world"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn tools_call_server_error() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(error_response(2, -32601, "Method not found"));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();

        let err = client
            .tools_call("nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Method not found"));
    }

    #[tokio::test]
    async fn raw_request_returns_response() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(error_response(2, -32601, "Unknown tool"));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();

        let resp = client
            .raw_request("tools/call", Some(serde_json::json!({"name": "nope"})))
            .await
            .unwrap();

        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn close_transitions_session() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));

        let mut client = make_client(mock);
        client.initialize().await.unwrap();
        client.close().await.unwrap();

        let err = client.tools_list().await.unwrap_err();
        assert!(err.to_string().contains("not ready") || err.to_string().contains("Closed"));
    }
}
