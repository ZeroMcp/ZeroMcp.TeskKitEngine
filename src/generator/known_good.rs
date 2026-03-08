use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::mcp::Tool;

/// A known-good baseline entry capturing a tool's schema and actual response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub tool_name: String,
    pub input_schema: Value,
    pub params_used: Value,
    pub response: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_paths: Vec<String>,
}

/// Full baseline document written by `generate --known-good`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: String,
    pub server: String,
    pub captured_at: String,
    pub entries: Vec<BaselineEntry>,
}

impl Baseline {
    pub fn new(server: &str) -> Self {
        Self {
            version: "1".to_string(),
            server: server.to_string(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, tool: &Tool, params: Value, response: Value) {
        self.entries.push(BaselineEntry {
            tool_name: tool.name.clone(),
            input_schema: tool.input_schema.clone(),
            params_used: params,
            response,
            ignore_paths: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn baseline_round_trip() {
        let mut baseline = Baseline::new("http://localhost:8000/mcp");
        let tool = Tool {
            name: "search".to_string(),
            description: Some("Search".to_string()),
            input_schema: json!({"type": "object"}),
            annotations: None,
        };
        baseline.add_entry(&tool, json!({"query": "hello"}), json!({"result": "world"}));

        let json = serde_json::to_string_pretty(&baseline).unwrap();
        let parsed: Baseline = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].tool_name, "search");
    }
}
