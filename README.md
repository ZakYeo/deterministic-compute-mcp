# deterministic-compute-mcp

Deterministic compute for AI agents.

`deterministic-compute-mcp` is an MCP server that gives Codex, Claude, and other MCP clients a local, exact, machine-readable calculator for work where "close enough" is not good enough: arithmetic, business math, unit conversion, result verification, and repeatable expected-value generation.

The project is Rust-first for correctness and exposes a TypeScript stdio MCP server for agent workflows. It is currently pre-release: the core, CLI, MCP wrapper, schemas, examples, and tests are implemented, while broad release packaging and client-specific polish are still active contribution areas.

## Features

- Exact JSON-safe integers and fixed-scale decimals.
- Deterministic arithmetic: add, subtract, multiply, divide.
- Arithmetic expression evaluation with `+`, `-`, `*`, `/`, parentheses, and unary minus.
- Unit conversion for length, mass, time, and temperature.
- Finance and business calculators:
  - simple interest
  - compound interest
  - fixed loan payment
  - VAT
  - percentage change
  - margin and markup
  - exact-representable CAGR
- Result verification with exact, absolute-tolerance, and relative-tolerance comparisons.
- Expected-value generation over explicit bounded test cases.
- Stable JSON CLI boundary for scripts, tests, and non-MCP integrations.
- MCP tools with both text JSON and structured content responses.

## Why Agents Need This

LLMs are useful at reasoning, but they should not be trusted to mentally calculate finance, verify numeric outputs, or generate golden test values. This server gives agents a deterministic compute tool with explicit precision, rounding, schemas, structured errors, and repeatable responses.

Use it when an agent needs to:

- calculate exact numeric results;
- verify a proposed answer against expected values;
- generate deterministic fixtures for tests;
- explain or audit business math;
- keep calculations local instead of sending them to a hosted service.

## Current Status

Implemented:

- Rust compute core in `crates/compute-core/`.
- Rust JSON CLI in `crates/compute-cli/`.
- TypeScript MCP stdio server in `apps/mcp-server-ts/`.
- JSON schemas in `schemas/`.
- Runnable examples in `examples/`.
- Interface and architecture docs in `docs/`.

Pre-release notes:

- The npm package name is `@deterministic-compute/mcp-server`.
- The current package flow is intended for pre-release testing and Linux x64 packaged CLI work.
- Other platforms can use the MCP wrapper by building `compute-cli` locally and pointing the server at that binary.
- Contributions to packaging, cross-platform binaries, and client setup docs are welcome.

## MCP Tools

The server registers these tools:

- `compute_arithmetic`: exact add, subtract, multiply, and divide.
- `compute_expression`: deterministic arithmetic expressions.
- `convert_units`: deterministic unit conversion.
- `calculate_finance`: finance and business calculators.
- `verify_result`: exact and tolerance-based comparisons.
- `generate_expected_values`: deterministic expected values for supported operations.

## Quick Start From Source

Requirements:

- Rust toolchain compatible with workspace `rust-version = "1.76"`.
- Node.js `>=20`.
- npm.

Install dependencies and build:

```sh
npm ci --prefix apps/mcp-server-ts
cargo build -p compute-cli
npm --prefix apps/mcp-server-ts run build
```

Run the MCP server from the local checkout:

```sh
DETERMINISTIC_COMPUTE_CLI_COMMAND="$PWD/target/debug/compute-cli" \
DETERMINISTIC_COMPUTE_CLI_ARGS_JSON='[]' \
node apps/mcp-server-ts/dist/index.js
```

The server uses stdio, so it is normally launched by an MCP client rather than run directly in a terminal session.

## Use With Codex

For local development, build the CLI and server first:

```sh
cargo build -p compute-cli
npm --prefix apps/mcp-server-ts run build
```

Then add the checkout-backed server:

```sh
codex mcp add deterministic-compute-local \
  --env DETERMINISTIC_COMPUTE_CLI_COMMAND="$PWD/target/debug/compute-cli" \
  --env DETERMINISTIC_COMPUTE_CLI_ARGS_JSON='[]' \
  -- node "$PWD/apps/mcp-server-ts/dist/index.js"
```

Restart Codex, then run `/mcp` in the TUI or:

```sh
codex mcp list
```

The target package install command is:

```sh
codex mcp add deterministic-compute -- npx -y @deterministic-compute/mcp-server
```

Use the source-based setup above until release packaging for your platform is available.

## Use With Claude Desktop

Build the local CLI and MCP server, then add a stdio server entry to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "deterministic-compute": {
      "command": "node",
      "args": ["/absolute/path/to/deterministic-compute-mcp/apps/mcp-server-ts/dist/index.js"],
      "env": {
        "DETERMINISTIC_COMPUTE_CLI_COMMAND": "/absolute/path/to/deterministic-compute-mcp/target/debug/compute-cli",
        "DETERMINISTIC_COMPUTE_CLI_ARGS_JSON": "[]"
      }
    }
  }
}
```

Use absolute paths because MCP clients may launch servers from a different working directory.

## CLI Usage

Run a request file:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/arithmetic-request.json
```

Run with stdin:

```sh
printf '%s\n' '{"operation":"arithmetic.add","input":{"left":{"kind":"integer","value":"20"},"right":{"kind":"integer","value":"22"}}}' \
  | cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml --
```

Example request:

```json
{
  "operation": "arithmetic.divide",
  "input": {
    "left": { "kind": "integer", "value": "2" },
    "right": { "kind": "integer", "value": "3" }
  },
  "precision": {
    "decimalPlaces": 2,
    "rounding": "half-away-from-zero"
  },
  "trace": true
}
```

Numbers are JSON-safe tagged values:

```json
{ "kind": "integer", "value": "42" }
```

```json
{ "kind": "decimal", "value": "0.05", "scale": 2 }
```

Decimal `scale` must match the number of fractional digits in `value`. Supported rounding modes are `exact`, `truncate`, and `half-away-from-zero`.

## Development

Rust checks:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p compute-cli
```

TypeScript MCP checks:

```sh
npm ci --prefix apps/mcp-server-ts
npm --prefix apps/mcp-server-ts run typecheck
npm --prefix apps/mcp-server-ts run build
npm --prefix apps/mcp-server-ts test
```

Run the real Codex smoke test from a local checkout with Codex already authenticated:

```sh
scripts/codex-mcp-smoke.sh
```

## Contributing

Contributions are welcome. Good first areas:

- cross-platform release packaging for the Rust CLI and MCP server;
- setup docs for Codex, Claude Desktop, Cursor, and other MCP clients;
- more unit conversions and finance calculators;
- stronger schema coverage and examples;
- resource-limit and security review;
- real-world agent workflow smoke tests.

Please keep computation deterministic, avoid floating-point math in core calculations, return structured errors, and add tests at the boundary your change affects.

## Repository Layout

```text
apps/mcp-server-ts/      TypeScript MCP stdio wrapper
crates/compute-core/     Deterministic compute primitives and dispatcher
crates/compute-cli/      JSON CLI process boundary
docs/                    Architecture, interfaces, and roadmap
examples/                Runnable request and response fixtures
schemas/                 JSON Schema contracts
```

## More Docs

- [Architecture](docs/architecture.md)
- [Interfaces](docs/interfaces.md)
- [Roadmap](docs/roadmap.md)
- [Examples](examples/README.md)
