use serde_json::Value;

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

/// Remove all fields matching the given JSONPath expressions from a value.
fn remove_ignored_paths(value: &Value, ignore_paths: &[String]) -> Value {
    let mut cleaned = value.clone();

    for path_str in ignore_paths {
        let pointers = jsonpath_to_pointers(path_str, &cleaned);
        // Remove in reverse order so earlier indices stay valid
        for ptr in pointers.into_iter().rev() {
            remove_at_pointer(&mut cleaned, &ptr);
        }
    }

    cleaned
}

/// Convert a JSONPath expression to a list of JSON Pointer paths that match
/// in the given value. Uses `query_located` to get both the matched nodes
/// and their normalized paths, then converts to JSON Pointers.
fn jsonpath_to_pointers(jsonpath: &str, value: &Value) -> Vec<String> {
    if let Ok(path) = serde_json_path::JsonPath::parse(jsonpath) {
        let located = path.query_located(value);
        let pointers: Vec<String> = located
            .all()
            .iter()
            .map(|node| node.location().to_json_pointer())
            .collect();

        if !pointers.is_empty() {
            return pointers;
        }
    }

    // Fallback: convert simple dot-notation JSONPath to JSON Pointer.
    // Handles: $.foo.bar -> /foo/bar, $.foo[0].bar -> /foo/0/bar
    if let Some(pointer) = simple_jsonpath_to_pointer(jsonpath) {
        if value.pointer(&pointer).is_some() {
            return vec![pointer];
        }
    }

    vec![]
}

/// Convert simple JSONPath expressions like `$.foo.bar` or `$.foo[0].bar`
/// to JSON Pointer format `/foo/bar` or `/foo/0/bar`.
fn simple_jsonpath_to_pointer(jsonpath: &str) -> Option<String> {
    let path = jsonpath.strip_prefix('$')?;
    if path.is_empty() {
        return Some(String::new());
    }

    let mut pointer = String::new();

    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }

        if let Some(bracket_pos) = segment.find('[') {
            let key = &segment[..bracket_pos];
            if !key.is_empty() {
                pointer.push('/');
                pointer.push_str(key);
            }

            let rest = &segment[bracket_pos..];
            for part in rest.split('[') {
                if part.is_empty() {
                    continue;
                }
                if let Some(idx_str) = part.strip_suffix(']') {
                    pointer.push('/');
                    pointer.push_str(idx_str);
                }
            }
        } else {
            pointer.push('/');
            pointer.push_str(segment);
        }
    }

    Some(pointer)
}

/// Remove a value at the given JSON Pointer path from the document.
fn remove_at_pointer(value: &mut Value, pointer: &str) {
    if pointer.is_empty() {
        return;
    }

    let segments: Vec<&str> = pointer
        .strip_prefix('/')
        .unwrap_or(pointer)
        .split('/')
        .collect();

    if segments.is_empty() {
        return;
    }

    let parent_segments = &segments[..segments.len() - 1];
    let last_key = segments[segments.len() - 1];

    let mut current = value as &mut Value;
    for seg in parent_segments {
        current = match current {
            Value::Object(map) => match map.get_mut(*seg) {
                Some(v) => v,
                None => return,
            },
            Value::Array(arr) => match seg.parse::<usize>() {
                Ok(idx) if idx < arr.len() => &mut arr[idx],
                _ => return,
            },
            _ => return,
        };
    }

    match current {
        Value::Object(map) => {
            map.remove(last_key);
        }
        Value::Array(arr) => {
            if let Ok(idx) = last_key.parse::<usize>() {
                if idx < arr.len() {
                    arr.remove(idx);
                }
            }
        }
        _ => {}
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

    #[test]
    fn ignore_paths_strips_timestamp() {
        let responses = vec![
            json!({"result": "hello", "timestamp": "2025-01-01T00:00:00Z"}),
            json!({"result": "hello", "timestamp": "2025-01-01T00:00:01Z"}),
        ];
        let errors = validate_determinism(
            "test",
            &responses,
            &["$.timestamp".to_string()],
        );
        assert!(errors.is_empty(), "Should pass after ignoring timestamp: {:?}", errors);
    }

    #[test]
    fn ignore_paths_strips_nested_field() {
        let responses = vec![
            json!({"data": {"value": "same", "id": "abc-123"}}),
            json!({"data": {"value": "same", "id": "def-456"}}),
        ];
        let errors = validate_determinism(
            "test",
            &responses,
            &["$.data.id".to_string()],
        );
        assert!(errors.is_empty(), "Should pass after ignoring data.id: {:?}", errors);
    }

    #[test]
    fn ignore_paths_multiple_fields() {
        let responses = vec![
            json!({"value": "same", "id": "abc", "ts": 1}),
            json!({"value": "same", "id": "def", "ts": 2}),
        ];
        let errors = validate_determinism(
            "test",
            &responses,
            &["$.id".to_string(), "$.ts".to_string()],
        );
        assert!(errors.is_empty(), "Should pass after ignoring id and ts: {:?}", errors);
    }

    #[test]
    fn ignore_paths_still_fails_on_real_diff() {
        let responses = vec![
            json!({"value": "hello", "id": "abc"}),
            json!({"value": "world", "id": "def"}),
        ];
        let errors = validate_determinism(
            "test",
            &responses,
            &["$.id".to_string()],
        );
        assert!(!errors.is_empty(), "Should still fail because value differs");
    }

    #[test]
    fn simple_jsonpath_to_pointer_basic() {
        assert_eq!(simple_jsonpath_to_pointer("$.foo"), Some("/foo".to_string()));
        assert_eq!(simple_jsonpath_to_pointer("$.foo.bar"), Some("/foo/bar".to_string()));
        assert_eq!(simple_jsonpath_to_pointer("$.foo[0].bar"), Some("/foo/0/bar".to_string()));
        assert_eq!(simple_jsonpath_to_pointer("$"), Some(String::new()));
    }

    #[test]
    fn remove_at_pointer_removes_key() {
        let mut val = json!({"a": 1, "b": 2});
        remove_at_pointer(&mut val, "/a");
        assert_eq!(val, json!({"b": 2}));
    }

    #[test]
    fn remove_at_pointer_nested() {
        let mut val = json!({"data": {"id": "abc", "value": "hello"}});
        remove_at_pointer(&mut val, "/data/id");
        assert_eq!(val, json!({"data": {"value": "hello"}}));
    }

    #[test]
    fn remove_at_pointer_array_element() {
        let mut val = json!({"items": [1, 2, 3]});
        remove_at_pointer(&mut val, "/items/1");
        assert_eq!(val, json!({"items": [1, 3]}));
    }
}
