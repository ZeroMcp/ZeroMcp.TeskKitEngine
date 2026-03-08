# ZeroMCP.TestKit — Product Specification

## 1. Purpose and Vision

ZeroMCP.TestKit provides a unified, language‑agnostic suite for validating MCP servers, ensuring they behave correctly, consistently, and interoperably across the ZeroMCP ecosystem. It defines a standard for MCP server correctness by combining a standalone protocol‑level test engine with fluent, language‑specific DSLs.

The goal is to make MCP server validation:

- Deterministic across repeated runs.

- Protocol‑correct in handshake, frames, errors, and streaming.

- Schema‑accurate for tool metadata and results.

- Interoperable across languages and runtimes.

- Relay‑safe when routed through ZeroMCP.Relay.

ZeroMCP.TestKit becomes the canonical correctness oracle for the entire ecosystem.

## 2. High-Level Architecture

### 2.1 Core Test Engine (Standalone)

A language‑neutral executable responsible for executing MCP protocol tests. It accepts a JSON test definition and returns a structured JSON result.

#### Responsibilities

- Connect to MCP servers (direct or via relay).

- Validate handshake and session lifecycle.

- Execute tool calls with provided parameters.

- Validate JSON schema compliance.

- Validate streaming semantics and chunk ordering.

- Validate cancellation behaviour.

- Validate concurrency handling.

- Validate determinism across repeated runs.

- Validate error frames and error codes.

- Validate metadata stability.

- Validate relay passthrough behaviour.

#### Non‑Responsibilities

- Test discovery.

- Reporting formats beyond JSON.

- Language‑specific DSLs.

- Business logic validation.

#### Implementation Notes

- Rust preferred for portability, safety, and performance.

- Outputs deterministic JSON for CI and tooling integration.

- Single static binary with no runtime dependencies.

### 2.2 Language-Specific Fluent DSLs

Thin wrappers that generate test definitions and invoke the core engine.

| Initial languages |
| ------- |

| .NET (ZeroMCP.TestKit) |

| Node.js (ZeroMCP.TestKit.Node) |

| Python (ZeroMCP.TestKit.Py) |

| Go (ZeroMCP.TestKit.Go) |


#### Responsibilities

Provide fluent, expressive test definitions.

Serialize test definitions to JSON.

Invoke the core engine (process or FFI).

Parse results and integrate with native test frameworks.

Provide idiomatic assertion styles.

#### Example (.NET)

await McpTest
    .Server("ws://localhost:8000")
    .Tool("search")
        .WithParams(new { query = "hello" })
        .ExpectSchemaMatch()
        .ExpectDeterministic()
    .RunAsync();

#### Example (Node)

await mcpTest()
  .server("ws://localhost:8000")
  .tool("search")
    .params({ query: "hello" })
    .expectSchemaValid()
    .expectDeterministic()
  .run();

### 2.3 Runners and Integrations

Optional components that wrap the core engine for different environments.

#### Examples

CLI runner for CI pipelines.

GitHub Action for MCP server validation.

VSCode extension for local development.

JetBrains plugin for IDE integration.

These runners orchestrate the engine but contain no test logic.

## 3. Test Definition Format (JSON)

### 3.1 Structure

A test definition is a JSON document containing:

server — MCP endpoint (ws, http, relay).

tests — array of test cases.

config — optional settings (timeouts, retries, determinism runs).

### 3.2 Example

{
  "server": "ws://localhost:8000",
  "tests": [
    {
      "tool": "search",
      "params": { "query": "hello" },
      "expect": {
        "schema_valid": true,
        "deterministic": true,
        "stream_min_chunks": 0
      }
    }
  ]
}

### 3.3 Expected Result Format

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

## 4. Test Categories

### 4.1 Protocol Validation

Handshake correctness.

Session lifecycle.

Frame structure and ordering.

Error frame correctness.

### 4.2 Tool Metadata Validation

JSON schema correctness.

Parameter validation.

Return type validation.

Optional field behaviour.

Documentation presence.

### 4.3 Determinism Validation

Multiple runs with identical inputs.

Metadata stability.

Output stability.

### 4.4 Streaming Validation

Chunk ordering.

Minimum chunk count.

Backpressure handling.

Cancellation mid-stream.

### 4.5 Concurrency Validation

Multiple simultaneous tool calls.

Isolation of tool state.

No cross‑request contamination.

### 4.6 Relay Validation

Behaviour through ZeroMCP.Relay.

Metadata passthrough.

Header/token preservation.

No mutation of frames.

## 5. Integration with ZeroMCP.Relay

ZeroMCP.TestKit validates relay behaviour by running tests:

Directly against the server.

Through a single relay.

Through multiple chained relays.

With multiple upstream APIs.

This ensures the relay is transparent and protocol‑correct.

## 6. Reporting and Output

### 6.1 JSON Output

The core engine always returns structured JSON.

### 6.2 Language-Specific Reporting

Each DSL integrates with its native test framework:

.NET: xUnit/NUnit/MSTest

Node: Jest/Vitest

Python: pytest

Go: testing package

### 6.3 CI Integration

CLI runner supports exit codes.

GitHub Action publishes annotated results.

Optional Markdown summary for PRs.

## 7. Roadmap

### Phase 1 — Core Engine + .NET DSL

Rust engine MVP.

.NET fluent API.

Basic protocol + schema tests.

### Phase 2 — Node + Python DSLs

Node wrapper.

Python wrapper.

Streaming + concurrency tests.

### Phase 3 — Relay Validation

Relay passthrough tests.

Multi‑relay scenarios.

Multi‑upstream scenarios.

### Phase 4 — Ecosystem Adoption

GitHub Action.

VSCode extension.

Public conformance suite.

## 8. Success Metrics

Number of MCP servers validated using TestKit.

Adoption across languages (Node, Go, Python).

CI integrations in public MCP projects.

Reduction in protocol‑related issues in ZeroMCP repos.

Community contributions to test definitions.

## 9. Open Questions

Should the core engine support plugin-based custom assertions?

Should determinism tests allow configurable tolerances for nondeterministic APIs?

Should the engine support recording/replay for debugging?
