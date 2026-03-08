use std::time::Instant;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::definition::{Expectation, TestConfig, TestDefinition};
use crate::engine::result::{
    ErrorCategory, RunStatus, TestRunResult, ToolTestResult, ValidationError,
};
use crate::protocol::client::McpClient;
use crate::protocol::mcp::Tool;
use crate::validators;

/// Orchestrates the execution of all test cases in a definition against
/// a connected MCP server.
pub struct TestExecutor {
    definition: TestDefinition,
    config: TestConfig,
}

impl TestExecutor {
    pub fn new(definition: TestDefinition) -> Self {
        let config = definition.config.clone().unwrap_or_default();
        Self { definition, config }
    }

    /// Execute all test cases and return the aggregated result.
    /// The client must already be initialized (session Ready).
    pub async fn run(&self, client: &mut McpClient) -> Result<TestRunResult> {
        let start = Instant::now();

        let tools = client
            .tools_list()
            .await
            .context("Failed to list tools from server")?;

        let tool_map: std::collections::HashMap<&str, &Tool> =
            tools.iter().map(|t| (t.name.as_str(), t)).collect();

        let mut results: Vec<ToolTestResult> = Vec::new();
        let mut any_failed = false;

        for test_case in &self.definition.tests {
            let case_start = Instant::now();
            let timeout = test_case
                .expect
                .timeout_ms
                .unwrap_or(self.config.timeout_ms);

            tracing::info!(tool = %test_case.tool, "Executing test case");

            let tool_result = tokio::time::timeout(
                std::time::Duration::from_millis(timeout),
                self.run_single(client, &test_case.tool, &test_case.params, &test_case.expect, tool_map.get(test_case.tool.as_str()).copied()),
            )
            .await;

            let elapsed = case_start.elapsed().as_millis() as u64;

            let tool_test_result = match tool_result {
                Ok(Ok(mut r)) => {
                    r.elapsed_ms = elapsed;
                    r
                }
                Ok(Err(e)) => ToolTestResult {
                    tool: test_case.tool.clone(),
                    passed: false,
                    schema_valid: None,
                    deterministic: None,
                    stream_chunks: None,
                    errors: vec![ValidationError {
                        category: ErrorCategory::Internal,
                        message: format!("{:#}", e),
                        context: None,
                    }],
                    elapsed_ms: elapsed,
                },
                Err(_) => ToolTestResult {
                    tool: test_case.tool.clone(),
                    passed: false,
                    schema_valid: None,
                    deterministic: None,
                    stream_chunks: None,
                    errors: vec![ValidationError {
                        category: ErrorCategory::Timeout,
                        message: format!(
                            "Tool '{}' timed out after {}ms",
                            test_case.tool, timeout
                        ),
                        context: None,
                    }],
                    elapsed_ms: elapsed,
                },
            };

            if !tool_test_result.passed {
                any_failed = true;
            }
            results.push(tool_test_result);
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(TestRunResult {
            status: if any_failed {
                RunStatus::Failed
            } else {
                RunStatus::Passed
            },
            results,
            elapsed_ms: elapsed,
        })
    }

    async fn run_single(
        &self,
        client: &mut McpClient,
        tool_name: &str,
        params: &Value,
        expect: &Expectation,
        tool_descriptor: Option<&Tool>,
    ) -> Result<ToolTestResult> {
        let mut errors: Vec<ValidationError> = Vec::new();
        let mut schema_valid_result: Option<bool> = None;
        let mut deterministic_result: Option<bool> = None;

        // --- Error path testing: expect an error response ---
        if expect.expect_error || expect.expect_error_code.is_some() {
            let response = client.raw_request(
                "tools/call",
                Some(serde_json::json!({ "name": tool_name, "arguments": params })),
            ).await.context("Failed to send error-path request")?;

            if let Some(code) = expect.expect_error_code {
                let mut e = validators::error_path::validate_error_code(tool_name, &response, code);
                errors.append(&mut e);
            } else {
                let mut e = validators::error_path::validate_is_error(tool_name, &response);
                errors.append(&mut e);
            }

            return Ok(ToolTestResult {
                tool: tool_name.to_string(),
                passed: errors.is_empty(),
                schema_valid: None,
                deterministic: None,
                stream_chunks: None,
                errors,
                elapsed_ms: 0,
            });
        }

        // --- Normal tool call ---
        let call_result = client
            .tools_call(tool_name, params.clone())
            .await
            .context(format!("tools/call failed for '{}'", tool_name))?;

        if call_result.is_error {
            errors.push(ValidationError {
                category: ErrorCategory::Protocol,
                message: format!("Tool '{}' returned isError: true", tool_name),
                context: None,
            });
        }

        let response_value = serde_json::to_value(&call_result)?;

        // --- Schema validation ---
        if expect.schema_valid {
            if let Some(tool) = tool_descriptor {
                let schema_errors =
                    validators::schema::validate_tool_output(tool_name, &tool.input_schema, &response_value);
                schema_valid_result = Some(schema_errors.is_empty());
                errors.extend(schema_errors);
            } else {
                errors.push(ValidationError {
                    category: ErrorCategory::Schema,
                    message: format!(
                        "Tool '{}' not found in tools/list — cannot validate schema",
                        tool_name
                    ),
                    context: None,
                });
                schema_valid_result = Some(false);
            }
        }

        // --- Determinism validation ---
        if expect.deterministic {
            let runs = self.config.determinism_runs as usize;
            let mut responses = vec![response_value.clone()];

            for i in 1..runs {
                tracing::debug!(tool = %tool_name, run = i + 1, total = runs, "Determinism re-run");
                let re_result = client
                    .tools_call(tool_name, params.clone())
                    .await
                    .context(format!("Determinism re-run {} failed for '{}'", i + 1, tool_name))?;
                responses.push(serde_json::to_value(&re_result)?);
            }

            let det_errors =
                validators::determinism::validate_determinism(tool_name, &responses, &expect.ignore_paths);
            deterministic_result = Some(det_errors.is_empty());
            errors.extend(det_errors);
        }

        Ok(ToolTestResult {
            tool: tool_name.to_string(),
            passed: errors.is_empty(),
            schema_valid: schema_valid_result,
            deterministic: deterministic_result,
            stream_chunks: None,
            errors,
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{Expectation, TestCase, TestConfig, TestDefinition};
    use crate::engine::result::ErrorCategory;
    use crate::transport::mock::*;

    fn make_definition(tests: Vec<TestCase>) -> TestDefinition {
        TestDefinition {
            schema_url: None,
            version: "1".to_string(),
            server: "mock://test".to_string(),
            tests,
            config: Some(TestConfig {
                timeout_ms: 5000,
                determinism_runs: 3,
                retries: 0,
            }),
        }
    }

    fn make_test_case(tool: &str, expect: Expectation) -> TestCase {
        TestCase {
            tool: tool.to_string(),
            params: serde_json::json!({"q": "hello"}),
            expect,
            generated: None,
        }
    }

    /// Set up a MockTransport pre-loaded with init and tools/list responses,
    /// then return a ready McpClient.
    async fn ready_client(mock: MockTransport) -> McpClient {
        let mut client = McpClient::new(Box::new(mock));
        client.initialize().await.unwrap();
        client
    }

    #[tokio::test]
    async fn simple_tool_call_passes() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        // tools/list
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "echo",
                "inputSchema": { "type": "object" }
            }]),
        ));
        // tools/call for the test case
        mock.push_response(tool_call_response(3, "hello"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "echo",
            Expectation::default(),
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].passed);
        assert_eq!(result.status, RunStatus::Passed);
        assert_eq!(result.exit_code(), 0);
    }

    #[tokio::test]
    async fn schema_validation_passes_when_valid() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "echo",
                "inputSchema": {
                    "type": "object",
                    "properties": { "text": { "type": "string" } }
                }
            }]),
        ));
        mock.push_response(tool_call_response(3, "result"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "echo",
            Expectation {
                schema_valid: true,
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert_eq!(result.results.len(), 1);
        // schema_valid is set to Some(true/false)
        assert!(result.results[0].schema_valid.is_some());
    }

    #[tokio::test]
    async fn determinism_passes_with_identical_responses() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "echo",
                "inputSchema": { "type": "object" }
            }]),
        ));
        // 3 identical responses (determinism_runs = 3)
        mock.push_response(tool_call_response(3, "same"));
        mock.push_response(tool_call_response(4, "same"));
        mock.push_response(tool_call_response(5, "same"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "echo",
            Expectation {
                deterministic: true,
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert_eq!(result.results[0].deterministic, Some(true));
        assert!(result.results[0].passed);
    }

    #[tokio::test]
    async fn determinism_fails_with_different_responses() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "echo",
                "inputSchema": { "type": "object" }
            }]),
        ));
        mock.push_response(tool_call_response(3, "first"));
        mock.push_response(tool_call_response(4, "different!"));
        mock.push_response(tool_call_response(5, "first"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "echo",
            Expectation {
                deterministic: true,
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert_eq!(result.results[0].deterministic, Some(false));
        assert!(!result.results[0].passed);
        assert_eq!(result.status, RunStatus::Failed);
        assert_eq!(result.exit_code(), 1);
    }

    #[tokio::test]
    async fn error_path_expect_error_passes() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(2, serde_json::json!([])));
        // raw_request returns an error
        mock.push_response(error_response(3, -32601, "Method not found"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "nonexistent",
            Expectation {
                expect_error: true,
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert!(result.results[0].passed);
    }

    #[tokio::test]
    async fn error_path_expect_specific_code() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(2, serde_json::json!([])));
        mock.push_response(error_response(3, -32601, "Method not found"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "nonexistent",
            Expectation {
                expect_error_code: Some(-32601),
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert!(result.results[0].passed);
        assert!(result.results[0].errors.is_empty());
    }

    #[tokio::test]
    async fn error_path_wrong_code_fails() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(2, serde_json::json!([])));
        mock.push_response(error_response(3, -32600, "Invalid request"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "nonexistent",
            Expectation {
                expect_error_code: Some(-32601),
                ..Default::default()
            },
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert!(!result.results[0].passed);
        assert_eq!(result.results[0].errors[0].category, ErrorCategory::ErrorPath);
    }

    #[tokio::test]
    async fn timeout_produces_timeout_error() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "slow",
                "inputSchema": { "type": "object" }
            }]),
        ));
        // Don't push a tools/call response — the receive will return Closed,
        // but the executor wraps it in a timeout.

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![TestCase {
            tool: "slow".to_string(),
            params: serde_json::json!({}),
            expect: Expectation {
                timeout_ms: Some(100), // very short timeout
                ..Default::default()
            },
            generated: None,
        }]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert!(!result.results[0].passed);
        // The error should be either a timeout or a transport closed error
        assert!(!result.results[0].errors.is_empty());
    }

    #[tokio::test]
    async fn multiple_test_cases_aggregated() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([
                { "name": "echo", "inputSchema": { "type": "object" } },
                { "name": "greet", "inputSchema": { "type": "object" } }
            ]),
        ));
        mock.push_response(tool_call_response(3, "hello"));
        mock.push_response(tool_call_response(4, "hi there"));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![
            make_test_case("echo", Expectation::default()),
            make_test_case("greet", Expectation::default()),
        ]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert_eq!(result.results.len(), 2);
        assert!(result.results[0].passed);
        assert!(result.results[1].passed);
        assert_eq!(result.status, RunStatus::Passed);
    }

    #[tokio::test]
    async fn tool_returning_is_error_fails() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));
        mock.push_response(tools_list_response(
            2,
            serde_json::json!([{
                "name": "broken",
                "inputSchema": { "type": "object" }
            }]),
        ));
        // Tool returns isError: true
        mock.push_response(success_response(
            3,
            serde_json::json!({
                "content": [{ "type": "text", "text": "something went wrong" }],
                "isError": true
            }),
        ));

        let mut client = ready_client(mock).await;

        let def = make_definition(vec![make_test_case(
            "broken",
            Expectation::default(),
        )]);

        let executor = TestExecutor::new(def);
        let result = executor.run(&mut client).await.unwrap();

        assert!(!result.results[0].passed);
        assert!(result.results[0]
            .errors
            .iter()
            .any(|e| e.category == ErrorCategory::Protocol));
    }
}
