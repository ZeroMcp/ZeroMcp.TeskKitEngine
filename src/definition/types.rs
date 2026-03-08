use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level test definition document — the public contract consumed by all language DSLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDefinition {
    /// Optional JSON Schema reference for editor support.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema_url: Option<String>,

    /// Format version for forward compatibility (e.g. "1").
    pub version: String,

    /// MCP server endpoint: http(s) URL, ws(s) URL, or stdio command.
    pub server: String,

    /// Array of test cases to execute.
    pub tests: Vec<TestCase>,

    /// Optional global configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<TestConfig>,
}

/// A single test case targeting one tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// The tool name to invoke via `tools/call`.
    pub tool: String,

    /// Parameters to pass to the tool (must match the tool's inputSchema).
    #[serde(default)]
    pub params: Value,

    /// Expected outcomes for this test case.
    #[serde(default)]
    pub expect: Expectation,

    /// Marker indicating this was auto-generated and needs review.
    #[serde(rename = "_generated", default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<bool>,
}

/// Expectations for a single test case.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Expectation {
    /// Validate that tool output conforms to its declared JSON Schema.
    #[serde(default)]
    pub schema_valid: bool,

    /// Run the call multiple times and assert identical output.
    #[serde(default)]
    pub deterministic: bool,

    /// JSONPath expressions for fields to ignore in determinism comparison
    /// (timestamps, IDs, cursors, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_paths: Vec<String>,

    /// If set, assert at least this many streaming chunks are received.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_min_chunks: Option<u32>,

    /// Expected JSON-RPC error code (for error path testing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_error_code: Option<i64>,

    /// If true, expect the tool call to be rejected (error response).
    #[serde(default)]
    pub expect_error: bool,

    /// Custom timeout override for this specific test (milliseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Global configuration for the test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Default timeout per tool call in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Number of repeated calls for determinism validation.
    #[serde(default = "default_determinism_runs")]
    pub determinism_runs: u32,

    /// Number of retry attempts on transient failures.
    #[serde(default)]
    pub retries: u32,

    /// Validate MCP protocol correctness (handshake, JSON-RPC frames).
    #[serde(default)]
    pub validate_protocol: bool,

    /// Validate tool metadata (name, description, inputSchema presence).
    #[serde(default)]
    pub validate_metadata: bool,

    /// Auto-generate error-path tests (unknown tool, malformed params).
    #[serde(default)]
    pub auto_error_tests: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout(),
            determinism_runs: default_determinism_runs(),
            retries: 0,
            validate_protocol: false,
            validate_metadata: false,
            auto_error_tests: false,
        }
    }
}

fn default_timeout() -> u64 {
    30_000
}

fn default_determinism_runs() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_test_definition() {
        let json = r#"{
            "$schema": "https://zeromcp.dev/schemas/testkit.v1.json",
            "version": "1",
            "server": "http://localhost:8000/mcp",
            "tests": [
                {
                    "tool": "search",
                    "params": { "query": "hello" },
                    "expect": {
                        "schema_valid": true,
                        "deterministic": true,
                        "ignore_paths": ["$.result.timestamp", "$.result.id"],
                        "stream_min_chunks": 0
                    }
                }
            ]
        }"#;

        let def: TestDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.version, "1");
        assert_eq!(def.server, "http://localhost:8000/mcp");
        assert_eq!(def.tests.len(), 1);
        assert_eq!(def.tests[0].tool, "search");
        assert!(def.tests[0].expect.schema_valid);
        assert!(def.tests[0].expect.deterministic);
        assert_eq!(def.tests[0].expect.ignore_paths.len(), 2);

        let serialized = serde_json::to_string_pretty(&def).unwrap();
        let roundtrip: TestDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(roundtrip.tests.len(), 1);
    }

    #[test]
    fn deserialize_minimal_definition() {
        let json = r#"{
            "version": "1",
            "server": "stdio:python server.py",
            "tests": [
                { "tool": "echo", "params": { "text": "hi" } }
            ]
        }"#;

        let def: TestDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.tests[0].tool, "echo");
        assert!(!def.tests[0].expect.schema_valid);
        assert!(!def.tests[0].expect.deterministic);
    }

    #[test]
    fn deserialize_generated_stub() {
        let json = r#"{
            "version": "1",
            "server": "http://localhost:8000/mcp",
            "tests": [
                {
                    "tool": "search",
                    "_generated": true,
                    "params": { "query": "__FILL_ME__" },
                    "expect": { "schema_valid": true }
                }
            ]
        }"#;

        let def: TestDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.tests[0].generated, Some(true));
    }
}
