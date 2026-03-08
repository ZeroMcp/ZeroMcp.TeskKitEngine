use crate::engine::result::{ErrorCategory, ValidationError};
use crate::protocol::jsonrpc::JsonRpcResponse;

/// Validate that an error response has the expected JSON-RPC error code.
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

/// Validate that a response is an error (any code).
pub fn validate_is_error(tool_name: &str, response: &JsonRpcResponse) -> Vec<ValidationError> {
    if response.error.is_some() {
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

    fn error_response(code: i64) -> JsonRpcResponse {
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
            result: Some(serde_json::json!({})),
            error: None,
        }
    }

    #[test]
    fn correct_error_code_passes() {
        let resp = error_response(-32601);
        assert!(validate_error_code("test", &resp, -32601).is_empty());
    }

    #[test]
    fn wrong_error_code_fails() {
        let resp = error_response(-32600);
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
    fn is_error_on_error_response() {
        let resp = error_response(-32601);
        assert!(validate_is_error("test", &resp).is_empty());
    }

    #[test]
    fn is_error_on_success_response() {
        let resp = success_response();
        assert!(!validate_is_error("test", &resp).is_empty());
    }
}
