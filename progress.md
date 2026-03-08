# ZeroMCP TestKit Engine — Progress

## Current Status: Milestones 1–3 Complete

### Milestone 1: Foundation (COMPLETE)

- [x] Cargo project initialized with all dependencies
- [x] CLI skeleton with `run`, `generate`, `diff` subcommands (clap derive)
- [x] Test definition types: `TestDefinition`, `TestCase`, `Expectation`, `TestConfig`
- [x] JSON Schema v1 for test definition format (embedded)
- [x] Test definition parser with schema validation and `__FILL_ME__` detection
- [x] JSON-RPC 2.0 types: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`, `JsonRpcError`, `RequestId`, `JsonRpcMessage`
- [x] MCP protocol types: `InitializeParams`, `InitializeResult`, `Tool`, `ToolsListResult`, `ToolCallParams`, `ToolCallResult`, `ToolContent`
- [x] MCP session state machine: `Disconnected -> Initializing -> Ready -> Closed`
- [x] Transport trait (`McpTransport`) with stdio and HTTP implementations (skeleton)
- [x] Engine result types: `TestRunResult`, `ToolTestResult`, `ValidationError`, `ErrorCategory`
- [x] Test executor skeleton
- [x] Validators: schema, determinism, protocol, error path (all with unit tests)
- [x] Test generator: scaffold (generates stubs from `tools/list`) and known-good baseline types
- [x] Recording/replay: session recorder and replay transport
- [x] Diff engine: tool comparison with added/removed/changed detection
- [x] README.md created
- [x] progress.md created

### Milestone 2: Transport Layer + MCP Session (COMPLETE)

- [x] HTTP transport: handles direct JSON responses, SSE event parsing, notification 202 handling, HTTP error reporting
- [x] SSE parser: extracts JSON-RPC messages from `text/event-stream` response bodies
- [x] `McpClient`: high-level client wrapping transport + session state machine
- [x] MCP handshake: sends `initialize` request, receives result, sends `notifications/initialized`
- [x] `tools/list` with pagination support (follows `nextCursor`)
- [x] `tools/call` invocation with parameter passing
- [x] `raw_request` for error-path testing (sends arbitrary JSON-RPC, returns raw response)
- [x] Transport factory: `create_transport(url)` creates HTTP or stdio transport from server URL
- [x] Session ID tracking via `Mcp-Session` header
- [x] `MockTransport` for unit testing (scriptable send/receive with helpers)
- [x] 8 client tests: handshake, tools_list (with pagination), tools_call, error propagation, raw_request, close
- [x] 10 executor tests: simple pass, schema validation, determinism pass/fail, error-path (expect_error, expect_code, wrong code), timeout, multi-case aggregation, isError flag

### Milestone 3: Test Execution Engine (COMPLETE)

- [x] `TestExecutor`: iterates test cases, calls tools via `McpClient`, runs validators, collects results
- [x] Timeout handling per test case using `tokio::time::timeout`
- [x] Schema validation of live tool responses against `tools/list` inputSchema
- [x] Determinism validation: multi-run with configurable `determinism_runs`
- [x] Error path testing: `expect_error` and `expect_error_code` expectations
- [x] `mcptest run`: load definition, connect, handshake, execute, output JSON, CI exit codes (0/1/2)
- [x] `mcptest generate --scaffold`: connect to live server, discover tools, emit stub test definitions with `__FILL_ME__` placeholders
- [x] `mcptest generate --known-good`: connect, call each tool with provided params, capture full baseline
- [x] `mcptest diff`: compare baseline against live server, report added/removed/changed tools
- [x] `--params` parsing for known-good: `tool_name:{"key":"value"}` format
- [x] Output to file (`--out`) or stdout
- [x] 83 unit tests — all passing (up from 61)

### Milestone 4: Advanced Validators (PENDING)

- [ ] Determinism: implement `value_to_pointer` for JSONPath `ignore_paths` to actually strip fields
- [ ] Protocol validation: integrate `validate_initialize_response` and `validate_jsonrpc_frame` into executor
- [ ] Error path: test unknown tool name, malformed params, timeout behaviour against live servers

### Milestone 5: Test Generation (COMPLETE — merged into M3)

- [x] `mcptest generate --scaffold` connected to live server
- [x] `mcptest generate --known-good` with param parsing and baseline capture

### Milestone 6: Recording, Replay, Diff (PARTIAL — diff complete, recording pending)

- [ ] Recording transport middleware wrapping real transport
- [ ] `mcptest run --record` and `--replay` flags wired up
- [x] `mcptest diff` connected to live server comparison
- [x] Human-readable diff output on stderr

---

## Build Notes

- **Windows**: `dlltool.exe` requires MinGW on PATH:
  ```powershell
  $env:PATH += ";C:\mingw\mingw64\bin"
  $env:PATH += ";C:\mingw\mingw64"
  ```
- Edition: Rust 2024 (requires Rust 1.85+)
- 83 unit tests pass as of Milestone 3 completion (including client + executor tests)

## File Structure

```
src/
  main.rs                     CLI entry point
  lib.rs                      Public API
  cli/
    mod.rs                    CLI definition (clap)
    run.rs                    mcptest run
    generate.rs               mcptest generate (--scaffold / --known-good)
    diff.rs                   mcptest diff
  definition/
    types.rs                  TestDefinition, TestCase, Expectation, TestConfig
    parser.rs                 Load + validate JSON test definitions
    schema.rs                 Embedded JSON Schema v1
  protocol/
    jsonrpc.rs                JSON-RPC 2.0 types
    mcp.rs                    MCP message types (initialize, tools/list, tools/call)
    session.rs                Session state machine
    client.rs                 McpClient — high-level MCP operations
  transport/
    mod.rs                    McpTransport trait + factory
    http.rs                   Streamable HTTP + SSE transport
    stdio.rs                  Stdio subprocess transport
  engine/
    executor.rs               Test execution orchestrator
    result.rs                 TestRunResult, ToolTestResult, ValidationError
  validators/
    schema.rs                 JSON Schema validation
    determinism.rs            Multi-run comparison
    protocol_val.rs           Protocol correctness checks
    error_path.rs             Error response validation
  generator/
    scaffold.rs               Generate stub test definitions
    known_good.rs             Capture known-good baselines
  recording/
    recorder.rs               Session recording
    replay.rs                 Replay transport
  diff/
    baseline.rs               Baseline drift detection
```
