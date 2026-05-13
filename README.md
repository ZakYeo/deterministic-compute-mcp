# deterministic-compute-mcp

`deterministic-compute-mcp` is planned as a production-grade deterministic computation engine for AI agents. The goal is to give Codex, Claude, Cursor, and other agent systems a trusted tool for exact calculations, unit conversions, finance/business math, expected-value generation, and result verification.

The project is intentionally Rust-first:

- `crates/compute-core` will contain deterministic computation primitives.
- `crates/compute-cli` will expose the core through a stable command-line interface.
- `apps/mcp-server-ts` will provide a TypeScript MCP stdio wrapper for agent integrations.
- `schemas` will define machine-readable request and response contracts.
- `examples` will show expected JSON interactions.

This foundation commit does not implement the computation engine. It creates the repository structure, compiling Rust crate skeletons, a TypeScript MCP server scaffold, and planning documentation for future focused workstreams.

## Repository Layout

```text
deterministic-compute-mcp/
  apps/
    mcp-server-ts/       TypeScript MCP stdio wrapper scaffold
  crates/
    compute-core/        Rust deterministic compute core skeleton
    compute-cli/         Rust CLI skeleton
  docs/                  Architecture and interface notes
  examples/              Planned request/response examples
  schemas/               Planned JSON schemas
```

## Quick Checks

```sh
cargo check --workspace
```

The TypeScript app is currently a scaffold. Once dependencies are installed for that app, future workers should use:

```sh
npm --prefix apps/mcp-server-ts run typecheck
npm --prefix apps/mcp-server-ts run build
```

## Planned Capabilities

- Deterministic arithmetic with explicit precision and rounding rules.
- Safe expression parsing and proof traces.
- Unit conversion with dimensional analysis.
- Finance and business calculators with documented assumptions.
- Expected-value generation for tests.
- Exact and tolerance-based verification of computed results.
- MCP tools with machine-readable, schema-backed outputs.

## Current Status

See [PROJECT_STATUS.md](PROJECT_STATUS.md).
