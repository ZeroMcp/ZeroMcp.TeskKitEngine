use serde::{Deserialize, Serialize};

/// Overall test run result — the canonical engine output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    /// "passed", "failed", or "error"
    pub status: RunStatus,

    /// Individual results per test case, in order.
    pub results: Vec<ToolTestResult>,

    /// Total execution time in milliseconds.
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Passed,
    Failed,
    Error,
}

/// Result of a single tool test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTestResult {
    /// The tool name that was tested.
    pub tool: String,

    /// Overall pass/fail for this test case.
    pub passed: bool,

    /// Schema validation result (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_valid: Option<bool>,

    /// Determinism check result (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deterministic: Option<bool>,

    /// Number of streaming chunks received (if streaming was tested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_chunks: Option<u32>,

    /// Errors encountered during this test case.
    pub errors: Vec<ValidationError>,

    /// Execution time in milliseconds.
    pub elapsed_ms: u64,
}

/// A structured validation error with category and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub category: ErrorCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Schema,
    Determinism,
    Protocol,
    ErrorPath,
    Timeout,
    Transport,
    Internal,
}

impl TestRunResult {
    /// CI exit code: 0 = all passed, 1 = any failed, 2 = engine error.
    pub fn exit_code(&self) -> i32 {
        match self.status {
            RunStatus::Passed => 0,
            RunStatus::Failed => 1,
            RunStatus::Error => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes() {
        let passed = TestRunResult {
            status: RunStatus::Passed,
            results: vec![],
            elapsed_ms: 0,
        };
        assert_eq!(passed.exit_code(), 0);

        let failed = TestRunResult {
            status: RunStatus::Failed,
            results: vec![],
            elapsed_ms: 0,
        };
        assert_eq!(failed.exit_code(), 1);

        let error = TestRunResult {
            status: RunStatus::Error,
            results: vec![],
            elapsed_ms: 0,
        };
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn result_serialization() {
        let result = TestRunResult {
            status: RunStatus::Passed,
            results: vec![ToolTestResult {
                tool: "search".to_string(),
                passed: true,
                schema_valid: Some(true),
                deterministic: Some(true),
                stream_chunks: Some(1),
                errors: vec![],
                elapsed_ms: 42,
            }],
            elapsed_ms: 100,
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        let parsed: TestRunResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, RunStatus::Passed);
        assert_eq!(parsed.results[0].tool, "search");
    }
}
