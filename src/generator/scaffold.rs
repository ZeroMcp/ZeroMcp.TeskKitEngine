use serde_json::json;

use crate::definition::types::{Expectation, TestCase, TestConfig, TestDefinition};
use crate::protocol::mcp::Tool;

/// Generate a scaffold test definition from a list of discovered tools.
///
/// Each tool gets a stub test case with `_generated: true` and placeholder
/// params (`__FILL_ME__`). Users must replace placeholders before the tests
/// become meaningful.
pub fn generate_scaffold(server: &str, tools: &[Tool]) -> TestDefinition {
    let tests = tools
        .iter()
        .map(|tool| {
            let params = generate_placeholder_params(&tool.input_schema);
            TestCase {
                tool: tool.name.clone(),
                params,
                expect: Expectation {
                    schema_valid: true,
                    deterministic: false,
                    ..Default::default()
                },
                generated: Some(true),
            }
        })
        .collect();

    TestDefinition {
        schema_url: Some("https://zeromcp.dev/schemas/testkit.v1.json".to_string()),
        version: "1".to_string(),
        server: server.to_string(),
        tests,
        config: Some(TestConfig::default()),
    }
}

/// Walk the JSON Schema `properties` and generate `__FILL_ME__` placeholders.
fn generate_placeholder_params(schema: &serde_json::Value) -> serde_json::Value {
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        let mut params = serde_json::Map::new();
        for (key, prop_schema) in properties {
            let placeholder = match prop_schema.get("type").and_then(|t| t.as_str()) {
                Some("string") => json!("__FILL_ME__"),
                Some("number") | Some("integer") => json!(0),
                Some("boolean") => json!(false),
                Some("array") => json!([]),
                Some("object") => json!({}),
                _ => json!("__FILL_ME__"),
            };
            params.insert(key.clone(), placeholder);
        }
        serde_json::Value::Object(params)
    } else {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_stubs_for_each_tool() {
        let tools = vec![
            Tool {
                name: "search".to_string(),
                description: Some("Search things".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer" }
                    },
                    "required": ["query"]
                }),
                annotations: None,
            },
            Tool {
                name: "echo".to_string(),
                description: None,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" }
                    }
                }),
                annotations: None,
            },
        ];

        let def = generate_scaffold("http://localhost:8000/mcp", &tools);
        assert_eq!(def.version, "1");
        assert_eq!(def.tests.len(), 2);
        assert_eq!(def.tests[0].tool, "search");
        assert_eq!(def.tests[0].generated, Some(true));
        assert_eq!(def.tests[0].params["query"], "__FILL_ME__");
        assert_eq!(def.tests[0].params["limit"], 0);
        assert_eq!(def.tests[1].tool, "echo");
    }

    #[test]
    fn empty_tools_produces_empty_tests() {
        let def = generate_scaffold("http://localhost:8000/mcp", &[]);
        assert!(def.tests.is_empty());
    }
}
