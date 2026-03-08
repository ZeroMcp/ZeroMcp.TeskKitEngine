# mcptest — ZeroMCP TestKit Engine

Universal testing toolkit for MCP (Model Context Protocol) servers. Validates any MCP-compliant server regardless of implementation language, runtime, or framework.

## Overview

`mcptest` is a standalone Rust binary that accepts JSON test definitions and produces structured JSON results. It is the core engine behind ZeroMCP.TestKit — language-specific DSLs (C#, Node.js, Python, Go) delegate all correctness decisions to this engine.

## Features

- **Protocol validation** — handshake, session lifecycle, JSON-RPC frame structure, error codes
- **Schema validation** — JSON Schema compliance for tool inputs and outputs
- **Determinism validation** — multi-run comparison with full JSONPath `ignore_paths` support
- **Error path validation** — unknown tools, malformed params, expected error codes
- **Tool metadata validation** — checks all tools have name, description, and valid inputSchema
- **Auto error-path testing** — automatically tests unknown tool rejection and malformed param handling
- **Test generation** — scaffold stubs or known-good baselines from a live server
- **Recording & replay** — capture sessions for offline CI and debugging
- **Baseline diffing** — detect schema drift between releases

## Installation

### From source

```bash
cargo install --path .
```

### Requirements

- Rust 1.85+ (edition 2024)
- On Windows with MSVC: `dlltool.exe` from MinGW must be on PATH

## Usage

### Run tests

```bash
mcptest run --file tests.json --server http://localhost:8000/mcp
```

### Run with advanced validation

```bash
mcptest run --file tests.json --server http://localhost:8000/mcp \
  --validate-protocol --validate-metadata --auto-error-tests
```

### Record a session

```bash
mcptest run --file tests.json --server http://localhost:8000/mcp \
  --record session.json
```

### Replay a recorded session (offline)

```bash
mcptest run --file tests.json --replay session.json
```

### Generate scaffold

```bash
mcptest generate --scaffold --server http://localhost:8000/mcp --out tests.json
```

### Generate known-good baseline

```bash
mcptest generate --known-good --server http://localhost:8000/mcp \
  --params search:'{"query":"hello"}' --out baseline.json
```

### Detect schema drift

```bash
mcptest diff --baseline baseline.json --server http://localhost:8000/mcp
```

## Test Definition Format

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
        "ignore_paths": ["$.result.timestamp"]
      }
    }
  ],
  "config": {
    "timeout_ms": 30000,
    "determinism_runs": 3,
    "validate_protocol": true,
    "validate_metadata": true,
    "auto_error_tests": true
  }
}
```

## Result Format

```json
{
  "status": "passed",
  "results": [
    {
      "tool": "search",
      "passed": true,
      "schema_valid": true,
      "deterministic": true,
      "errors": [],
      "elapsed_ms": 42
    }
  ],
  "elapsed_ms": 100
}
```

## CI Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed |
| 2 | Engine error (connection failure, invalid definition, etc.) |

## Transports

| Protocol | Server URL format |
|----------|-------------------|
| Streamable HTTP + SSE | `http://localhost:8000/mcp` or `https://...` |
| stdio | `stdio:python server.py` or just `python server.py` |
| WebSocket | `ws://localhost:8000/mcp` or `wss://...` (planned) |

## Validators

| Validator | Config key | What it checks |
|-----------|-----------|---------------|
| Schema | `schema_valid: true` | Tool output conforms to declared JSON Schema |
| Determinism | `deterministic: true` | Repeated calls produce identical results (after `ignore_paths` stripping) |
| Protocol | `validate_protocol: true` | Handshake correctness, JSON-RPC frame validity |
| Metadata | `validate_metadata: true` | All tools have name, description, valid inputSchema |
| Error path | `expect_error: true` | Tool call returns an error response |
| Error code | `expect_error_code: -32601` | Error response has a specific JSON-RPC error code |
| Auto errors | `auto_error_tests: true` | Auto-generates tests for unknown tool + malformed params |

## Architecture

```
CLI (clap)
  └── Engine
       ├── Test Definition Parser (JSON Schema validated)
       ├── Transport Layer (stdio, HTTP+SSE, recording middleware)
       ├── MCP Protocol Client (JSON-RPC 2.0)
       │    └── Session State Machine
       ├── Validators (schema, determinism, protocol, error path, metadata)
       ├── Test Generator (scaffold, known-good)
       ├── Recorder / Replay (session capture + offline testing)
       └── Diff Engine (baseline comparison)
```

## License

MIT
