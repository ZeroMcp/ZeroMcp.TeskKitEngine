use crate::engine::result::{ErrorCategory, ValidationError};
use crate::protocol::jsonrpc::{JsonRpcResponse, JSONRPC_VERSION};
use crate::protocol::mcp::InitializeResult;

/// Validate that the server's initialize response is protocol-correct.
pub fn validate_initialize_response(response: &InitializeResult) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if response.protocol_version.is_empty() {
        errors.push(ValidationError {
            category: ErrorCategory::Protocol,
            message: "Server returned empty protocolVersion".to_string(),
            context: None,
        });
    }

    if response.server_info.name.is_empty() {
        errors.push(ValidationError {
            category: ErrorCategory::Protocol,
            message: "Server returned empty serverInfo.name".to_string(),
            context: None,
        });
    }

    errors
}

/// Validate JSON-RPC frame structure.
pub fn validate_jsonrpc_frame(response: &JsonRpcResponse) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if response.jsonrpc != JSONRPC_VERSION {
        errors.push(ValidationError {
            category: ErrorCategory::Protocol,
            message: format!(
                "Expected jsonrpc version '{}', got '{}'",
                JSONRPC_VERSION, response.jsonrpc
            ),
            context: None,
        });
    }

    if response.result.is_none() && response.error.is_none() {
        errors.push(ValidationError {
            category: ErrorCategory::Protocol,
            message: "JSON-RPC response has neither result nor error".to_string(),
            context: None,
        });
    }

    if response.result.is_some() && response.error.is_some() {
        errors.push(ValidationError {
            category: ErrorCategory::Protocol,
            message: "JSON-RPC response has both result and error".to_string(),
            context: None,
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::RequestId;
    use crate::protocol::mcp::{Implementation, ServerCapabilities};
    use serde_json::json;

    #[test]
    fn valid_init_response() {
        let result = InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "test".to_string(),
                version: "1.0".to_string(),
            },
            instructions: None,
        };
        assert!(validate_initialize_response(&result).is_empty());
    }

    #[test]
    fn empty_protocol_version() {
        let result = InitializeResult {
            protocol_version: String::new(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "test".to_string(),
                version: "1.0".to_string(),
            },
            instructions: None,
        };
        let errors = validate_initialize_response(&result);
        assert!(!errors.is_empty());
    }

    #[test]
    fn valid_jsonrpc_frame() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: Some(json!({})),
            error: None,
        };
        assert!(validate_jsonrpc_frame(&resp).is_empty());
    }

    #[test]
    fn missing_result_and_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: None,
            error: None,
        };
        let errors = validate_jsonrpc_frame(&resp);
        assert!(!errors.is_empty());
    }
}
