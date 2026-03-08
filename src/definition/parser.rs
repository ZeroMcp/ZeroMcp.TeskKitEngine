use std::path::Path;

use anyhow::{Context, Result};
use jsonschema::Validator;
use serde_json::Value;

use super::schema::test_definition_schema_v1;
use super::types::TestDefinition;

/// Load a test definition from a JSON file, validating it against the v1 schema.
pub fn load_from_file(path: &Path) -> Result<TestDefinition> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read test definition: {}", path.display()))?;

    load_from_str(&contents)
}

/// Parse a test definition from a JSON string, validating against the v1 schema.
pub fn load_from_str(json: &str) -> Result<TestDefinition> {
    let value: Value = serde_json::from_str(json).context("Invalid JSON in test definition")?;

    validate_against_schema(&value)?;
    check_fill_me_placeholders(&value)?;

    let definition: TestDefinition =
        serde_json::from_value(value).context("Test definition does not match expected types")?;

    if definition.version != "1" {
        anyhow::bail!(
            "Unsupported test definition version '{}' (expected '1')",
            definition.version
        );
    }

    Ok(definition)
}

fn validate_against_schema(value: &Value) -> Result<()> {
    let schema = test_definition_schema_v1();
    let validator =
        Validator::new(&schema).map_err(|e| anyhow::anyhow!("Invalid schema: {}", e))?;

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("  - {}", e))
        .collect();

    if !errors.is_empty() {
        anyhow::bail!(
            "Test definition schema validation failed:\n{}",
            errors.join("\n")
        );
    }

    Ok(())
}

fn check_fill_me_placeholders(value: &Value) -> Result<()> {
    let json_str = serde_json::to_string(value)?;
    if json_str.contains("__FILL_ME__") {
        tracing::warn!(
            "Test definition contains __FILL_ME__ placeholder values. \
             These tests will likely fail until placeholders are replaced with real values."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_definition() {
        let json = r#"{
            "version": "1",
            "server": "http://localhost:8000/mcp",
            "tests": [
                {
                    "tool": "search",
                    "params": { "query": "hello" },
                    "expect": {
                        "schema_valid": true,
                        "deterministic": true,
                        "ignore_paths": ["$.result.timestamp"]
                    }
                }
            ]
        }"#;

        let def = load_from_str(json).unwrap();
        assert_eq!(def.tests.len(), 1);
        assert_eq!(def.tests[0].tool, "search");
    }

    #[test]
    fn reject_missing_version() {
        let json = r#"{
            "server": "http://localhost:8000/mcp",
            "tests": [{ "tool": "search" }]
        }"#;

        let err = load_from_str(json).unwrap_err();
        assert!(err.to_string().contains("schema validation failed"));
    }

    #[test]
    fn reject_empty_tests() {
        let json = r#"{
            "version": "1",
            "server": "http://localhost:8000/mcp",
            "tests": []
        }"#;

        let err = load_from_str(json).unwrap_err();
        assert!(err.to_string().contains("schema validation failed"));
    }

    #[test]
    fn reject_wrong_version() {
        let json = r#"{
            "version": "99",
            "server": "http://localhost:8000/mcp",
            "tests": [{ "tool": "search" }]
        }"#;

        let err = load_from_str(json).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Unsupported") || msg.contains("schema validation"),
            "Unexpected error: {msg}"
        );
    }

    #[test]
    fn warn_on_fill_me_placeholders() {
        let json = r#"{
            "version": "1",
            "server": "http://localhost:8000/mcp",
            "tests": [
                {
                    "tool": "search",
                    "params": { "query": "__FILL_ME__" },
                    "expect": { "schema_valid": true }
                }
            ]
        }"#;

        let def = load_from_str(json).unwrap();
        assert_eq!(def.tests[0].params["query"], "__FILL_ME__");
    }
}
