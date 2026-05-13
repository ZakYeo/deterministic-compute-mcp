# Roadmap

This project has moved past the foundation-only phase. The current repository contains a working Rust core, JSON CLI, and MCP wrapper for the main deterministic compute workflows.

## Complete

- Repository structure and Rust workspace.
- JSON-safe integer and fixed-scale decimal model.
- Deterministic arithmetic with precision and rounding policies.
- Safe arithmetic expression parser/evaluator in the Rust core and CLI.
- Deterministic unit conversion primitives in the Rust core module.
- Finance/business calculators for simple interest, compound interest, loan payment, percentage change, margin/markup, and exact-representable CAGR.
- Verification comparisons with exact, absolute-tolerance, and relative-tolerance modes.
- Expected-value generation over bounded explicit cases.
- Rust CLI for generic compute requests.
- TypeScript MCP wrapper for arithmetic, finance, verification, and expected-value generation.
- Public request/response schemas and runnable examples.

## Remaining Integration Work

- Wire `expression.evaluate` through the MCP `compute_expression` tool.
- Expose `units.convert` through the generic Rust dispatcher, CLI, schemas, examples, and MCP wrapper.
- Add release packaging so Codex users can install a prebuilt CLI/MCP server without running through `cargo run`.
- Expand compatibility docs for specific MCP clients.
- Continue security and resource-limit review for production use.
