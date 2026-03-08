use serde_json::Value;
use serde_json_path::JsonPath;

use crate::engine::result::{ErrorCategory, ValidationError};

/// Compare multiple tool call responses for determinism.
///
/// Responses are compared pairwise after removing fields matched by
/// `ignore_paths` (JSONPath expressions for timestamps, IDs, etc.).
pub fn validate_determinism(
    tool_name: &str,
    responses: &[Value],
    ignore_paths: &[String],
) -> Vec<ValidationError> {
    if responses.len() < 2 {
        return vec![ValidationError {
            category: ErrorCategory::Determinism,
            message: format!(
                "Tool '{}': need at least 2 responses for determinism check, got {}",
                tool_name,
                responses.len()
            ),
            context: None,
        }];
    }

    let cleaned: Vec<Value> = responses
        .iter()
        .map(|r| remove_ignored_paths(r, ignore_paths))
        .collect();

    let baseline = &cleaned[0];
    let mut errors = Vec::new();

    for (i, response) in cleaned.iter().enumerate().skip(1) {
        if baseline != response {
            let diff_desc = describe_diff(baseline, response);
            errors.push(ValidationError {
                category: ErrorCategory::Determinism,
                message: format!(
                    "Tool '{}': response #{} differs from baseline: {}",
                    tool_name,
                    i + 1,
                    diff_desc
                ),
                context: None,
            });
        }
    }

    errors
}

fn remove_ignored_paths(value: &Value, ignore_paths: &[String]) -> Value {
    let mut cleaned = value.clone();

    for path_str in ignore_paths {
        if let Ok(path) = JsonPath::parse(path_str) {
            let pointers: Vec<String> = {
                let nodes = path.query(&cleaned);
                nodes
                    .all()
                    .iter()
                    .filter_map(|node| value_to_pointer(&cleaned, node))
                    .collect()
            };
            for ptr in &pointers {
                remove_at_pointer(&mut cleaned, ptr);
            }
        }
    }

    cleaned
}

fn value_to_pointer(_root: &Value, _node: &Value) -> Option<String> {
    // TODO(m4): Implement proper JSONPath-to-pointer mapping
    None
}

fn remove_at_pointer(value: &mut Value, pointer: &str) {
    if let Some((parent_ptr, key)) = pointer.rsplit_once('/') {
        let parent_ptr = if parent_ptr.is_empty() {
            "/"
        } else {
            parent_ptr
        };
        if let Some(parent) = value.pointer_mut(parent_ptr) {
            if let Some(obj) = parent.as_object_mut() {
                obj.remove(key);
            }
        }
    }
}

fn describe_diff(a: &Value, b: &Value) -> String {
    let a_str = serde_json::to_string_pretty(a).unwrap_or_default();
    let b_str = serde_json::to_string_pretty(b).unwrap_or_default();

    let diff = similar::TextDiff::from_lines(&a_str, &b_str);
    let mut changes = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => changes.push(format!("-{}", change)),
            similar::ChangeTag::Insert => changes.push(format!("+{}", change)),
            _ => {}
        }
    }

    if changes.is_empty() {
        "structural difference detected".to_string()
    } else {
        changes.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identical_responses_pass() {
        let responses = vec![
            json!({"result": "hello"}),
            json!({"result": "hello"}),
            json!({"result": "hello"}),
        ];
        let errors = validate_determinism("test", &responses, &[]);
        assert!(errors.is_empty());
    }

    #[test]
    fn different_responses_fail() {
        let responses = vec![
            json!({"result": "hello"}),
            json!({"result": "world"}),
        ];
        let errors = validate_determinism("test", &responses, &[]);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].category, ErrorCategory::Determinism);
    }

    #[test]
    fn too_few_responses() {
        let responses = vec![json!({"result": "hello"})];
        let errors = validate_determinism("test", &responses, &[]);
        assert!(!errors.is_empty());
    }
}
