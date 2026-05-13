# Architecture

`deterministic-compute-mcp` is organized around a Rust-first deterministic computation core with thin integration layers.

## Layers

- `compute-core`: deterministic domain logic, precision rules, parsing, unit conversion, finance calculators, verification, and test-value generation.
- `compute-cli`: stable process boundary for JSON requests and responses.
- `mcp-server-ts`: TypeScript MCP stdio server that validates tool inputs, invokes the CLI, and returns machine-readable MCP content.

## Boundary Principles

- The Rust core owns computation correctness.
- The CLI owns stable JSON process contracts.
- The MCP server owns agent-facing tool registration and transport behavior.
- Schemas should describe all cross-process payloads.
- Outputs should include enough metadata to explain precision, rounding, tolerance, and assumptions.

## Non-Goals For Foundation

- No expression parser.
- No unit conversion table.
- No finance formulas.
- No MCP SDK integration.
- No production CLI command model.
