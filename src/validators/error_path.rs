use crate::engine::result::{ErrorCategory, ValidationError};
use crate::protocol::jsonrpc::JsonRpcResponse;

/// Check whether a JSON-RPC response represents an error at either level:
/// 1. JSON-RPC level: `response.error` is present
/// 2. MCP tool level: `response.result.isError` is true
fn is_error_response(response: &JsonRpcResponse) -> bool {
    if response.error.is_some() {
        return true;
    }
    if let Some(result) = &response.result {
        if let Some(is_error) = result.get("isError").and_then(|v| v.as_bool()) {
            return is_error;
        }
    }
    false
}

/// Validate that an error response has the expected JSON-RPC error code.
/// Only matches JSON-RPC level errors — MCP `isError` responses don't carry codes.
pub fn validate_error_code(
    tool_name: &str,
    response: &JsonRpcResponse,
    expected_code: i64,
) -> Vec<ValidationError> {
    match &response.error {
        Some(error) => {
            if error.code != expected_code {
                vec![ValidationError {
                    category: ErrorCategory::ErrorPath,
                    message: format!(
                        "Tool '{}': expected error code {}, got {}",
                        tool_name, expected_code, error.code
                    ),
                    context: Some(error.message.clone()),
                }]
            } else {
                vec![]
            }
        }
        None => {
            // No JSON-RPC error — but check if MCP isError is set
            if is_error_response(response) {
                // MCP tool-level error doesn't carry a code, so we can't match the specific code.
                // Treat as a pass since the tool did error, just not at the JSON-RPC level.
                vec![]
            } else {
                vec![ValidationError {
                    category: ErrorCategory::ErrorPath,
                    message: format!(
                        "Tool '{}': expected error response with code {}, but got a success response",
                        tool_name, expected_code
                    ),
                    context: None,
                }]
            }
        }
    }
}

/// Validate that a response is an error (any code).
/// Accepts both JSON-RPC errors and MCP `isError: true` responses.
pub fn validate_is_error(tool_name: &str, response: &JsonRpcResponse) -> Vec<ValidationError> {
    if is_error_response(response) {
        vec![]
    } else {
        vec![ValidationError {
            category: ErrorCategory::ErrorPath,
            message: format!(
                "Tool '{}': expected an error response, but got a success response",
                tool_name
            ),
            context: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::{JsonRpcError, RequestId};

    fn jsonrpc_error_response(code: i64) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: "test error".to_string(),
                data: None,
            }),
        }
    }

    fn success_response() -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: Some(serde_json::json!({
                "content": [{"type": "text", "text": "ok"}],
                "isError": false
            })),
            error: None,
        }
    }

    fn mcp_is_error_response() -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: Some(serde_json::json!({
                "content": [{"type": "text", "text": "Unauthorized"}],
                "isError": true
            })),
            error: None,
        }
    }

    #[test]
    fn correct_error_code_passes() {
        let resp = jsonrpc_error_response(-32601);
        assert!(validate_error_code("test", &resp, -32601).is_empty());
    }

    #[test]
    fn wrong_error_code_fails() {
        let resp = jsonrpc_error_response(-32600);
        let errors = validate_error_code("test", &resp, -32601);
        assert!(!errors.is_empty());
    }

    #[test]
    fn success_when_error_expected_fails() {
        let resp = success_response();
        let errors = validate_error_code("test", &resp, -32601);
        assert!(!errors.is_empty());
    }

    #[test]
    fn is_error_on_jsonrpc_error() {
        let resp = jsonrpc_error_response(-32601);
        assert!(validate_is_error("test", &resp).is_empty());
    }

    #[test]
    fn is_error_on_success_response() {
        let resp = success_response();
        assert!(!validate_is_error("test", &resp).is_empty());
    }

    #[test]
    fn is_error_on_mcp_is_error_true() {
        let resp = mcp_is_error_response();
        assert!(validate_is_error("test", &resp).is_empty(),
            "isError: true should be treated as an error response");
    }

    #[test]
    fn expect_error_code_passes_on_mcp_is_error() {
        let resp = mcp_is_error_response();
        let errors = validate_error_code("test", &resp, -32601);
        assert!(errors.is_empty(),
            "MCP isError: true should satisfy expect_error_code since the tool did error");
    }

    #[test]
    fn is_error_false_is_not_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: Some(serde_json::json!({
                "content": [{"type": "text", "text": "all good"}],
                "isError": false
            })),
            error: None,
        };
        assert!(!validate_is_error("test", &resp).is_empty());
    }
}
