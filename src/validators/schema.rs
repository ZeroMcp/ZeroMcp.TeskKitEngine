use jsonschema::Validator;
use serde_json::Value;

use crate::engine::result::{ErrorCategory, ValidationError};

/// Validate that the provided value conforms to the tool's declared JSON Schema.
/// Typically used to validate input params against `inputSchema`.
pub fn validate_tool_output(
    tool_name: &str,
    schema: &Value,
    value: &Value,
) -> Vec<ValidationError> {
    let validator = match Validator::new(schema) {
        Ok(v) => v,
        Err(e) => {
            return vec![ValidationError {
                category: ErrorCategory::Schema,
                message: format!("Tool '{}' has an invalid inputSchema: {}", tool_name, e),
                context: None,
            }];
        }
    };

    validator
        .iter_errors(value)
        .map(|e| {
            let ctx = e.instance_path().to_string();
            ValidationError {
                category: ErrorCategory::Schema,
                message: format!("Tool '{}': {}", tool_name, e),
                context: if ctx.is_empty() { None } else { Some(ctx) },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_output_passes() {
        let schema = json!({
            "type": "object",
            "properties": {
                "result": { "type": "string" }
            },
            "required": ["result"]
        });
        let response = json!({ "result": "hello" });
        let errors = validate_tool_output("test", &schema, &response);
        assert!(errors.is_empty());
    }

    #[test]
    fn invalid_output_fails() {
        let schema = json!({
            "type": "object",
            "properties": {
                "result": { "type": "string" }
            },
            "required": ["result"]
        });
        let response = json!({ "result": 42 });
        let errors = validate_tool_output("test", &schema, &response);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].category, ErrorCategory::Schema);
    }

    #[test]
    fn missing_required_field() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let response = json!({});
        let errors = validate_tool_output("test", &schema, &response);
        assert!(!errors.is_empty());
    }
}
