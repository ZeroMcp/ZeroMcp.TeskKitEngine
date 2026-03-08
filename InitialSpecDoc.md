# ZeroMCP.TestKit

**Product Specification** — *Version 1.0 • Draft*

---

## 1. Purpose and Vision

ZeroMCP.TestKit is a universal testing toolkit for MCP servers. It is designed to validate any MCP-compliant server regardless of implementation language, runtime, or framework — not to test ZeroMCP-specific behaviour.

The goal is to make MCP server validation:

- Deterministic across repeated runs
- Protocol-correct in handshake, frames, errors, and streaming
- Schema-accurate for tool metadata and results
- Interoperable across languages and runtimes
- Independently usable — with no dependency on any ZeroMCP product

> **Positioning:** ZeroMCP.TestKit is the canonical correctness oracle for the MCP ecosystem. ZeroMCP products use it because it is the best available tool, not because it was built for them.

---

## 2. High-Level Architecture

### 2.1 Core Test Engine (Standalone)

A language-neutral executable responsible for executing MCP protocol tests. It accepts a JSON test definition and returns a structured JSON result. The engine has no knowledge of any specific MCP server implementation.

**Responsibilities**

- Connect to MCP servers over any supported transport
- Validate handshake and session lifecycle
- Execute tool calls with provided parameters
- Validate JSON Schema compliance for tool inputs and outputs
- Validate streaming semantics and chunk ordering
- Validate cancellation behaviour
- Validate concurrency handling
- Validate determinism across repeated runs
- Validate error frames and error codes
- Validate metadata stability
- Validate authentication passthrough behaviour

**Non-Responsibilities**

- Test discovery
- Reporting formats beyond JSON
- Language-specific DSLs
- Business logic or application-level validation
- Any ZeroMCP-specific behaviour

**Implementation Notes**

- Rust preferred for portability, safety, and performance
- Outputs deterministic JSON for CI and tooling integration
- Single static binary with no runtime dependencies
- No dependency on any MCP framework or implementation

---

### 2.2 Language-Specific Fluent DSLs

Thin wrappers that generate test definitions and invoke the core engine. Each DSL is idiomatic to its language and integrates with that language's native test frameworks. They contain no test logic of their own — all correctness decisions are made by the engine.

**Initial Languages**

- .NET (ZeroMCP.TestKit)
- Node.js (ZeroMCP.TestKit.Node)
- Python (ZeroMCP.TestKit.Py)
- Go (ZeroMCP.TestKit.Go)

**Responsibilities**

- Provide fluent, expressive test definitions
- Serialize test definitions to the core JSON format
- Invoke the core engine (process or FFI)
- Parse results and integrate with native test frameworks
- Provide idiomatic assertion styles

**Example — .NET**
```csharp
await McpTest
    .Server("http://localhost:8000/mcp")
    .Tool("search")
        .WithParams(new { query = "hello" })
        .ExpectSchemaMatch()
        .ExpectDeterministic()
    .RunAsync();
```

**Example — Node.js**
```js
await mcpTest()
  .server("http://localhost:8000/mcp")
  .tool("search")
    .params({ query: "hello" })
    .expectSchemaValid()
    .expectDeterministic()
  .run();
```

---

### 2.3 Runners and Integrations

Optional components that wrap the core engine for different environments. These contain no test logic — they are orchestration only.

- CLI runner for CI pipelines
- GitHub Action for MCP server validation
- Visual Studio Test Explorer integration (.NET)
- VSCode extension for local development
- JetBrains plugin for IDE integration

> **Visual Studio Test Explorer:** Test Explorer is not a separate runner — it is a quality bar on the .NET DSL. If the DSL integrates correctly with xUnit, NUnit, and MSTest, Test Explorer picks up results automatically. It should be treated as a first-class deliverable of the .NET DSL, not an optional extra. Developers running MCP tests locally will expect results to appear alongside their other tests without any additional configuration.

---

## 3. Test Definition Format (JSON)

### 3.1 Structure

A test definition is a versioned JSON document containing:

- `version` — format version for forward compatibility
- `server` — MCP endpoint (http, ws, stdio)
- `tests` — array of test cases
- `config` — optional settings (timeouts, retries, determinism runs)

> **Versioning:** The format must carry a version field from day one. The test definition format is a public contract consumed by all language DSLs — breaking it without versioning would be a cross-ecosystem incident.

### 3.2 Example
```json
{
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
}
```

### 3.3 Expected Result Format
```json
{
  "status": "passed",
  "results": [
    {
      "tool": "search",
      "passed": true,
      "schema_valid": true,
      "deterministic": true,
      "stream_chunks": 1,
      "errors": []
    }
  ]
}
```

---

## 4. Test Categories

### 4.1 Protocol Validation

- Handshake correctness
- Session lifecycle
- Frame structure and ordering
- Error frame correctness and error codes

### 4.2 Tool Metadata Validation

- JSON Schema correctness for all tool descriptors
- Parameter presence and type accuracy
- Return type validation
- Optional field behaviour
- Documentation presence (name, description)
- Metadata stability across repeated calls

### 4.3 Determinism Validation

Tests run the same tool call multiple times and compare outputs. A test definition may specify paths to ignore for known non-deterministic fields (timestamps, IDs, cursors).

- Multiple runs with identical inputs
- Metadata stability
- Output stability with configurable `ignore_paths`

> **Design Decision:** Determinism tests must support `ignore_paths` (JSONPath) to accommodate real-world APIs that embed timestamps, generated IDs, or pagination tokens in responses. Without this, determinism tests produce false failures and get disabled.

### 4.4 Authentication Validation

Tests that authentication credentials are correctly forwarded and that the server responds appropriately to different auth contexts. This is generic MCP auth behaviour, not framework-specific.

- Bearer token forwarding — token present in outbound request
- API key forwarding — header or query parameter
- Rejection on missing or invalid credentials — correct error frame returned
- Tool visibility under different auth contexts — `tools/list` reflects auth state
- `tools/call` rejection for unauthorized callers — not just hidden from list

### 4.5 Error Path Validation

First-class testing of error conditions, distinct from protocol validation. MCP error behaviour is a contract that clients depend on.

- Unknown tool name — correct JSON-RPC error code
- Malformed parameters — validation error response
- Server-side error — error frame with appropriate code
- Timeout behaviour — error returned within configured deadline

### 4.6 Streaming Validation

- Chunk ordering
- Minimum chunk count
- Backpressure handling
- Cancellation mid-stream

### 4.7 Concurrency Validation

- Multiple simultaneous tool calls
- Isolation of tool state
- No cross-request contamination

---

## 5. Relay Passthrough Validation

When an MCP server is fronted by a relay or proxy, the relay must be transparent — it must not mutate frames, drop headers, or alter tool metadata. TestKit validates this as a generic MCP concern, applicable to any relay.

Tests run in three configurations:

- Directly against the server
- Through a single relay
- Through multiple chained relays

Assertions verify:

- Metadata passthrough — tool names, descriptions, and schemas unchanged
- Header and token preservation — auth credentials not dropped or mutated
- No frame mutation — JSON-RPC responses identical through any relay depth
- Relay error handling — relay reports upstream failure correctly rather than swallowing it

---

## 6. Test Generation

The CLI provides a `test generate` command that introspects a live MCP server and emits a test definition file. This removes the blank-page problem for new adopters and ensures the generated baseline reflects the server's actual current schema.

### 6.1 Scaffold Mode (`test generate --scaffold`)

Connects to a running server, calls `tools/list`, and emits a test definition with one stub per tool. Parameters are left as placeholder values that must be filled in before the tests are meaningful. Designed for first-time setup.
```bash
mcptest generate --scaffold --server http://localhost:8000/mcp
# → writes tests.json with one stub per tool
```
```json
{
  "$schema": "https://zeromcp.dev/schemas/testkit.v1.json",
  "version": "1",
  "server": "http://localhost:8000/mcp",
  "tests": [
    {
      "tool": "search",
      "_generated": true,
      "params": { "query": "__FILL_ME__" },
      "expect": {
        "schema_valid": true,
        "deterministic": false
      }
    }
  ]
}
```

> **The `_generated` flag:** Stubs carry a `"_generated": true` marker. A CI step can be configured to fail on any test still containing this flag or any `__FILL_ME__` value, enforcing that generated stubs are completed before merge. This prevents generated-but-unreviewed tests from silently passing.

### 6.2 Known-Good Mode (`test generate --known-good`)

Connects to a running server, executes each tool with provided parameters, and captures the full response — including the complete input schema for each tool — as a locked baseline. Future runs diff against this baseline to detect regressions.
```bash
mcptest generate --known-good --server http://localhost:8000/mcp \
  --params search:'{"query":"hello"}' \
  --out baseline.json
```

The generated baseline captures:

- Full input schema for each tool — renames and type changes are caught
- Actual response shape — structural regressions are caught
- Ignore paths for known non-deterministic fields (timestamps, IDs)

### 6.3 Drift Detection (`test diff`)

Compares a committed baseline against the current server state without running the full test suite. Designed for CI gates on schema changes.
```bash
# In CI — catch schema regressions before merge
mcptest diff --baseline baseline.json --server http://localhost:8000/mcp
```

Reports:

- Added tools — new tools not in baseline
- Removed tools — tools in baseline no longer present
- Schema changes — parameter renames, type changes, required field changes
- Response shape changes — structural differences in tool output

> **Workflow:** `--scaffold` generates stubs for authoring → `--known-good` locks a verified baseline → `diff` detects drift in CI. The three commands form a complete lifecycle without requiring manual test definition authorship for schema regression coverage.

---

## 7. Recording and Replay

The engine supports recording all inputs and outputs to a structured JSON session file, and replaying that session deterministically without a live server. This is in scope for Phase 1 because the engine already serializes all I/O to JSON — replay is a natural consequence of that design, and the debugging value is high enough to justify it early.

- `record` mode — run tests against a live server and write session file
- `replay` mode — run tests against a recorded session (no server required)
- `diff` mode — compare two session files for regression detection

> **Use Cases:** Replay enables offline CI runs, snapshot regression testing, and debugging of flaky tests without a live environment. It also enables community sharing of conformance test sessions.

---

## 8. Reporting and Output

### 8.1 JSON Output

The core engine always returns structured JSON. This is the canonical output format regardless of which language DSL or runner is used.

### 8.2 Language-Specific Reporting

Each DSL integrates results with its native test framework:

| Language | Frameworks |
|----------|------------|
| .NET | xUnit, NUnit, MSTest |
| Node.js | Jest, Vitest |
| Python | pytest |
| Go | testing package |

### 8.3 CI Integration

- CLI runner supports exit codes
- GitHub Action publishes annotated results
- Optional Markdown summary for pull request comments
- `mcprelay validate --strict` compatible exit code convention

---

## 9. Roadmap

### Phase 1 — CLI + Core Engine

The CLI runner and the JSON test definition format are the Phase 1 deliverable. This is the thing anyone on any stack can adopt immediately, and it forces the format to stabilize before DSLs are built on top of it.

- Rust engine MVP
- JSON test definition format v1 with schema
- CLI runner with CI exit codes
- Protocol, schema, and error path test categories
- Recording and replay
- `test generate --scaffold` and `--known-good`
- `test diff` for baseline drift detection

### Phase 2 — Language DSLs

- .NET fluent API with Visual Studio Test Explorer support
- Node.js wrapper
- Python wrapper
- Streaming and concurrency test categories

### Phase 3 — Auth and Relay Validation

- Authentication test category
- Relay passthrough tests
- Multi-relay and multi-upstream scenarios

### Phase 4 — Ecosystem Adoption

- GitHub Action
- VSCode extension
- Public conformance suite and community test definitions
- Go DSL

---

## 10. Open Questions

| Question | Notes |
|----------|-------|
| Plugin-based custom assertions in the core engine? | Likely no — keeps the engine lean and portable. Custom assertions belong in the DSL layer where they are just native code. |
| Should `ignore_paths` use JSONPath or a simpler format? | JSONPath is expressive but adds a dependency. A simple dot-notation format may be sufficient for most cases. |
| Should the public conformance suite be versioned independently of the engine? | Yes — test definitions are a public contract and should have their own versioning lifecycle. |

---

## 11. Success Metrics

- Adoption by MCP server projects outside the ZeroMCP ecosystem
- CLI downloads across non-.NET language ecosystems
- Community-contributed test definitions
- CI integrations in public MCP projects
- Protocol issues caught before production across all users
- Reduction in protocol-related issues in MCP server implementations generally
