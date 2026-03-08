use jsonschema::Validator;
use serde_json::Value;

use crate::engine::result::{ErrorCategory, ValidationError};
use crate::protocol::mcp::Tool;

/// Validate that all tools returned by `tools/list` have well-formed metadata:
/// - Non-empty `name`
/// - Non-empty `description` (warning-level, still reported)
/// - `inputSchema` is a valid JSON Schema object
pub fn validate_tool_metadata(tools: &[Tool]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for tool in tools {
        if tool.name.is_empty() {
            errors.push(ValidationError {
                category: ErrorCategory::Metadata,
                message: "Tool has an empty name".to_string(),
                context: None,
            });
        }

        if tool.name.contains(' ') {
            errors.push(ValidationError {
                category: ErrorCategory::Metadata,
                message: format!(
                    "Tool '{}': name contains spaces (may cause invocation issues)",
                    tool.name
                ),
                context: None,
            });
        }

        if tool.description.as_ref().is_none_or(|d| d.is_empty()) {
            errors.push(ValidationError {
                category: ErrorCategory::Metadata,
                message: format!(
                    "Tool '{}': missing or empty description",
                    tool.name
                ),
                context: None,
            });
        }

        validate_input_schema(&tool.name, &tool.input_schema, &mut errors);
    }

    if tools.is_empty() {
        errors.push(ValidationError {
            category: ErrorCategory::Metadata,
            message: "Server reported zero tools via tools/list".to_string(),
            context: None,
        });
    }

    errors
}

fn validate_input_schema(tool_name: &str, schema: &Value, errors: &mut Vec<ValidationError>) {
    if schema.is_null() {
        errors.push(ValidationError {
            category: ErrorCategory::Metadata,
            message: format!("Tool '{}': inputSchema is null", tool_name),
            context: None,
        });
        return;
    }

    if !schema.is_object() {
        errors.push(ValidationError {
            category: ErrorCategory::Metadata,
            message: format!(
                "Tool '{}': inputSchema is not an object (got {})",
                tool_name,
                schema_type_name(schema)
            ),
            context: None,
        });
        return;
    }

    if let Err(e) = Validator::new(schema) {
        errors.push(ValidationError {
            category: ErrorCategory::Metadata,
            message: format!(
                "Tool '{}': inputSchema is not valid JSON Schema: {}",
                tool_name, e
            ),
            context: None,
        });
    }
}

fn schema_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_tool(name: &str, desc: Option<&str>, schema: Value) -> Tool {
        Tool {
            name: name.to_string(),
            description: desc.map(|s| s.to_string()),
            input_schema: schema,
            annotations: None,
        }
    }

    #[test]
    fn valid_tool_metadata_passes() {
        let tools = vec![make_tool(
            "search",
            Some("Search for things"),
            json!({"type": "object", "properties": {"q": {"type": "string"}}}),
        )];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.is_empty(), "Expected no errors: {:?}", errors);
    }

    #[test]
    fn empty_name_fails() {
        let tools = vec![make_tool(
            "",
            Some("desc"),
            json!({"type": "object"}),
        )];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.iter().any(|e| e.message.contains("empty name")));
    }

    #[test]
    fn name_with_spaces_warns() {
        let tools = vec![make_tool(
            "my tool",
            Some("desc"),
            json!({"type": "object"}),
        )];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.iter().any(|e| e.message.contains("spaces")));
    }

    #[test]
    fn missing_description_warns() {
        let tools = vec![make_tool(
            "search",
            None,
            json!({"type": "object"}),
        )];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.iter().any(|e| e.message.contains("description")));
    }

    #[test]
    fn null_input_schema_fails() {
        let tools = vec![make_tool("search", Some("desc"), Value::Null)];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.iter().any(|e| e.message.contains("null")));
    }

    #[test]
    fn non_object_input_schema_fails() {
        let tools = vec![make_tool("search", Some("desc"), json!("string"))];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.iter().any(|e| e.message.contains("not an object")));
    }

    #[test]
    fn zero_tools_warns() {
        let errors = validate_tool_metadata(&[]);
        assert!(errors.iter().any(|e| e.message.contains("zero tools")));
    }

    #[test]
    fn multiple_tools_validates_each() {
        let tools = vec![
            make_tool("good", Some("desc"), json!({"type": "object"})),
            make_tool("", None, Value::Null),
        ];
        let errors = validate_tool_metadata(&tools);
        assert!(errors.len() >= 3, "Expected at least 3 errors for bad tool: {:?}", errors);
    }
}
