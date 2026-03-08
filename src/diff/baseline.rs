use serde::{Deserialize, Serialize};

use crate::protocol::mcp::Tool;

/// Result of comparing a baseline against the current server state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub server: String,
    pub baseline_server: String,
    pub added_tools: Vec<String>,
    pub removed_tools: Vec<String>,
    pub changed_tools: Vec<ToolDiff>,
    pub has_changes: bool,
}

/// Describes how a tool changed between baseline and current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiff {
    pub tool_name: String,
    pub changes: Vec<String>,
}

/// Compare a set of baseline tools against the current server's tools.
pub fn diff_tools(baseline_tools: &[Tool], current_tools: &[Tool]) -> DiffReport {
    let baseline_names: std::collections::HashSet<&str> =
        baseline_tools.iter().map(|t| t.name.as_str()).collect();
    let current_names: std::collections::HashSet<&str> =
        current_tools.iter().map(|t| t.name.as_str()).collect();

    let added: Vec<String> = current_names
        .difference(&baseline_names)
        .map(|s| s.to_string())
        .collect();

    let removed: Vec<String> = baseline_names
        .difference(&current_names)
        .map(|s| s.to_string())
        .collect();

    let mut changed = Vec::new();
    for baseline_tool in baseline_tools {
        if let Some(current_tool) = current_tools.iter().find(|t| t.name == baseline_tool.name) {
            let changes = diff_single_tool(baseline_tool, current_tool);
            if !changes.is_empty() {
                changed.push(ToolDiff {
                    tool_name: baseline_tool.name.clone(),
                    changes,
                });
            }
        }
    }

    let has_changes = !added.is_empty() || !removed.is_empty() || !changed.is_empty();

    DiffReport {
        server: String::new(),
        baseline_server: String::new(),
        added_tools: added,
        removed_tools: removed,
        changed_tools: changed,
        has_changes,
    }
}

fn diff_single_tool(baseline: &Tool, current: &Tool) -> Vec<String> {
    let mut changes = Vec::new();

    if baseline.description != current.description {
        changes.push(format!(
            "description changed: {:?} -> {:?}",
            baseline.description, current.description
        ));
    }

    if baseline.input_schema != current.input_schema {
        changes.push("inputSchema changed".to_string());

        if let (Some(b_props), Some(c_props)) = (
            baseline.input_schema.get("properties"),
            current.input_schema.get("properties"),
        ) {
            let b_keys: std::collections::HashSet<&str> = b_props
                .as_object()
                .map(|o| o.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            let c_keys: std::collections::HashSet<&str> = c_props
                .as_object()
                .map(|o| o.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();

            for added in c_keys.difference(&b_keys) {
                changes.push(format!("  + added parameter '{}'", added));
            }
            for removed in b_keys.difference(&c_keys) {
                changes.push(format!("  - removed parameter '{}'", removed));
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_tool(name: &str, props: serde_json::Value) -> Tool {
        Tool {
            name: name.to_string(),
            description: Some(format!("{name} tool")),
            input_schema: json!({ "type": "object", "properties": props }),
            annotations: None,
        }
    }

    #[test]
    fn detects_added_tools() {
        let baseline = vec![make_tool("search", json!({}))];
        let current = vec![
            make_tool("search", json!({})),
            make_tool("fetch", json!({})),
        ];
        let report = diff_tools(&baseline, &current);
        assert!(report.has_changes);
        assert!(report.added_tools.contains(&"fetch".to_string()));
    }

    #[test]
    fn detects_removed_tools() {
        let baseline = vec![
            make_tool("search", json!({})),
            make_tool("fetch", json!({})),
        ];
        let current = vec![make_tool("search", json!({}))];
        let report = diff_tools(&baseline, &current);
        assert!(report.has_changes);
        assert!(report.removed_tools.contains(&"fetch".to_string()));
    }

    #[test]
    fn detects_schema_changes() {
        let baseline = vec![make_tool(
            "search",
            json!({ "query": { "type": "string" } }),
        )];
        let current = vec![make_tool(
            "search",
            json!({ "query": { "type": "string" }, "limit": { "type": "integer" } }),
        )];
        let report = diff_tools(&baseline, &current);
        assert!(report.has_changes);
        assert_eq!(report.changed_tools.len(), 1);
    }

    #[test]
    fn no_changes_when_identical() {
        let tools = vec![make_tool(
            "search",
            json!({ "query": { "type": "string" } }),
        )];
        let report = diff_tools(&tools, &tools);
        assert!(!report.has_changes);
    }
}
