# mcptest — ZeroMCP TestKit Engine

Universal testing toolkit for MCP (Model Context Protocol) servers. Validates any MCP-compliant server regardless of implementation language, runtime, or framework.

## Overview

`mcptest` is a standalone Rust binary that accepts JSON test definitions and produces structured JSON results. It is the core engine behind ZeroMCP.TestKit — language-specific DSLs (C#, Node.js, Python, Go) delegate all correctness decisions to this engine.

## Features

- **Protocol validation** — handshake, session lifecycle, frame structure, error codes
- **Schema validation** — JSON Schema compliance for tool inputs and outputs
- **Determinism validation** — multi-run comparison with configurable `ignore_paths`
- **Error path validation** — unknown tools, malformed params, timeouts
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
  ]
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
| WebSocket | `ws://localhost:8000/mcp` or `wss://...` |

## Architecture

```
CLI (clap)
  └── Engine
       ├── Test Definition Parser (JSON Schema validated)
       ├── Transport Layer (stdio, HTTP+SSE)
       ├── MCP Protocol Client (JSON-RPC 2.0)
       │    └── Session State Machine
       ├── Validators (schema, determinism, protocol, error path)
       ├── Test Generator (scaffold, known-good)
       ├── Recorder / Replay
       └── Diff Engine (baseline comparison)
```

## License

MIT
