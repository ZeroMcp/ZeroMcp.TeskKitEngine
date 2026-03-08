# ZeroMCP TestKit Engine — Progress

## Current Status: Milestones 1–4 Complete + M6 Recording/Replay

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

### Milestone 4: Advanced Validators (COMPLETE)

- [x] **Determinism ignore_paths**: implemented `jsonpath_to_pointers` using `serde_json_path::query_located()` + `NormalizedPath::to_json_pointer()` — full JSONPath support
- [x] **Fallback path converter**: `simple_jsonpath_to_pointer` for `$.foo.bar` / `$.foo[0].bar` syntax
- [x] **Robust `remove_at_pointer`**: walks JSON tree via segments, handles objects and arrays
- [x] **Protocol validation integration**: `validate_initialize_response` runs as pre-flight check when `validate_protocol: true` in config
- [x] **JSON-RPC frame validation**: `validate_jsonrpc_frame` runs on error-path responses when protocol validation is enabled
- [x] **Tool metadata validation**: new `validators::metadata` module checks name, description, inputSchema validity
- [x] **Auto error-path tests**: `generate_auto_error_tests()` creates test cases for unknown tool + malformed params for each known tool
- [x] **Config flags**: `validate_protocol`, `validate_metadata`, `auto_error_tests` added to `TestConfig`
- [x] **CLI flags**: `--validate-protocol`, `--validate-metadata`, `--auto-error-tests` on `mcptest run`
- [x] **ErrorCategory::Metadata**: new error category for metadata validation failures
- [x] 8 new metadata validator tests
- [x] 4 new ignore_paths tests (timestamp, nested, multiple, still-fails)
- [x] 4 new remove_at_pointer tests
- [x] 5 new executor integration tests (protocol validation, metadata validation, auto error tests, frame validation)

### Milestone 5: Test Generation (COMPLETE — merged into M3)

- [x] `mcptest generate --scaffold` connected to live server
- [x] `mcptest generate --known-good` with param parsing and baseline capture

### Milestone 6: Recording, Replay, Diff (COMPLETE)

- [x] **`RecordingTransport`**: middleware that wraps any `McpTransport`, records all sent/received messages with timestamps
- [x] **`ReplayTransport`**: replays a `RecordedSession` without a live server
- [x] **`--record` flag**: `mcptest run --record session.json` saves the full session recording
- [x] **`--replay` flag**: `mcptest run --replay session.json` replays a recording for offline testing
- [x] `mcptest diff` connected to live server comparison
- [x] Human-readable diff output on stderr
- [x] 3 new recording transport tests (records messages, doesn't alter, close propagation)

---

## Test Summary

| Module | Tests | What's covered |
|--------|-------|----------------|
| `definition/` | 9 | Type round-trips, parser validation, schema |
| `protocol/jsonrpc.rs` | 7 | Serialization, deserialization, request IDs |
| `protocol/mcp.rs` | 5 | MCP message types round-trips |
| `protocol/session.rs` | 6 | State machine transitions, error cases |
| `protocol/client.rs` | 8 | Handshake, tools_list (+ pagination), tools_call, error propagation, raw_request, close |
| `transport/mock.rs` | 3 | Send/receive, closed state, empty queue |
| `transport/mod.rs` + `http.rs` | 9 | URL parsing, SSE parsing |
| `engine/executor.rs` | 15 | Simple pass, schema, determinism pass/fail, error-path (3), timeout, multi-case, isError, **protocol validation, metadata validation, auto error tests, frame validation, auto test generation** |
| `engine/result.rs` | 2 | Exit codes, serialization |
| `validators/determinism.rs` | 11 | Identical, different, too-few, **ignore_paths (timestamp, nested, multiple, still-fails), remove_at_pointer (key, nested, array), simple_jsonpath_to_pointer** |
| `validators/error_path.rs` | 5 | Error code correct/wrong, success-when-error, is_error |
| `validators/metadata.rs` | 8 | Valid, empty name, spaces, missing desc, null schema, non-object schema, zero tools, multi-tool |
| `validators/protocol_val.rs` | 4 | Valid init, empty protocol version, valid frame, missing result+error |
| `generator/` | 3 | Scaffold, known-good baseline |
| `recording/recorder.rs` | 1 | Record + serialize |
| `recording/recording_transport.rs` | 3 | Records messages, message passthrough, close propagation |
| `diff/baseline.rs` | 4 | Added/removed/changed/no-change |
| **Total** | **107** | |

---

## Build Notes

- **Windows**: `dlltool.exe` requires MinGW on PATH:
  ```powershell
  $env:PATH += ";C:\mingw\mingw64\bin"
  $env:PATH += ";C:\mingw\mingw64"
  ```
- Edition: Rust 2024 (requires Rust 1.85+)
- 107 unit tests pass as of Milestone 4 + M6 completion

## File Structure

```
src/
  main.rs                     CLI entry point
  lib.rs                      Public API
  cli/
    mod.rs                    CLI definition (clap)
    run.rs                    mcptest run (with --record, --replay, --validate-protocol, --validate-metadata, --auto-error-tests)
    generate.rs               mcptest generate (--scaffold / --known-good)
    diff.rs                   mcptest diff
  definition/
    types.rs                  TestDefinition, TestCase, Expectation, TestConfig (with validate_protocol, validate_metadata, auto_error_tests)
    parser.rs                 Load + validate JSON test definitions
    schema.rs                 Embedded JSON Schema v1
  protocol/
    jsonrpc.rs                JSON-RPC 2.0 types
    mcp.rs                    MCP message types (initialize, tools/list, tools/call)
    session.rs                Session state machine
    client.rs                 McpClient — high-level MCP operations + transport_as_any
  transport/
    mod.rs                    McpTransport trait + factory
    http.rs                   Streamable HTTP + SSE transport
    stdio.rs                  Stdio subprocess transport
    mock.rs                   Scriptable mock transport for testing
  engine/
    executor.rs               Test execution orchestrator (protocol validation, metadata validation, auto error tests)
    result.rs                 TestRunResult, ToolTestResult, ValidationError, ErrorCategory (+Metadata)
  validators/
    schema.rs                 JSON Schema validation
    determinism.rs            Multi-run comparison with full JSONPath ignore_paths support
    protocol_val.rs           Protocol correctness checks (handshake + frame)
    error_path.rs             Error response validation
    metadata.rs               Tool metadata validation (name, description, inputSchema)
  generator/
    scaffold.rs               Generate stub test definitions
    known_good.rs             Capture known-good baselines
  recording/
    recorder.rs               Session recording types
    recording_transport.rs    Recording middleware transport (wraps any McpTransport)
    replay.rs                 Replay transport (offline testing)
  diff/
    baseline.rs               Baseline drift detection
```
