use serde_json::Value;

/// Embedded JSON Schema for TestKit test definition format v1.
/// Used to validate user-provided test definition files before execution.
pub fn test_definition_schema_v1() -> Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zeromcp.dev/schemas/testkit.v1.json",
        "title": "ZeroMCP TestKit Test Definition",
        "description": "A versioned JSON document describing MCP server test cases",
        "type": "object",
        "required": ["version", "server", "tests"],
        "properties": {
            "$schema": {
                "type": "string",
                "description": "JSON Schema reference for editor support"
            },
            "version": {
                "type": "string",
                "const": "1",
                "description": "Format version"
            },
            "server": {
                "type": "string",
                "minLength": 1,
                "description": "MCP server endpoint (http/https URL, ws/wss URL, or stdio:command)"
            },
            "tests": {
                "type": "array",
                "minItems": 1,
                "items": { "$ref": "#/$defs/testCase" },
                "description": "Array of test cases"
            },
            "config": { "$ref": "#/$defs/testConfig" }
        },
        "additionalProperties": false,
        "$defs": {
            "testCase": {
                "type": "object",
                "required": ["tool"],
                "properties": {
                    "tool": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Tool name to invoke via tools/call"
                    },
                    "params": {
                        "description": "Parameters to pass to the tool"
                    },
                    "expect": { "$ref": "#/$defs/expectation" },
                    "_generated": {
                        "type": "boolean",
                        "description": "Marker for auto-generated stubs"
                    }
                },
                "additionalProperties": false
            },
            "expectation": {
                "type": "object",
                "properties": {
                    "schema_valid": { "type": "boolean", "default": false },
                    "deterministic": { "type": "boolean", "default": false },
                    "ignore_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "JSONPath expressions for non-deterministic fields"
                    },
                    "stream_min_chunks": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "expect_error_code": {
                        "type": "integer",
                        "description": "Expected JSON-RPC error code"
                    },
                    "expect_error": {
                        "type": "boolean",
                        "default": false
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Per-test timeout override in milliseconds"
                    }
                },
                "additionalProperties": false
            },
            "testConfig": {
                "type": "object",
                "properties": {
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 1,
                        "default": 30000
                    },
                    "determinism_runs": {
                        "type": "integer",
                        "minimum": 1,
                        "default": 3
                    },
                    "retries": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0
                    }
                },
                "additionalProperties": false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_valid_json() {
        let schema = test_definition_schema_v1();
        assert!(schema.is_object());
        assert_eq!(schema["$defs"]["testCase"]["type"], "object");
    }
}
