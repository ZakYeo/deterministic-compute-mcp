# Agent Instructions

This repository is being built through focused worker branches. Do not revert or overwrite edits made by other agents. Keep changes scoped to the paths owned by your current assignment.

## Product Direction

`deterministic-compute-mcp` is a production-grade deterministic computation engine for AI agents. It uses a Rust-first compute core with a TypeScript MCP wrapper.

Primary goals:

- Verify calculations.
- Generate exact expected values for tests.
- Perform deterministic unit conversions.
- Run finance and business calculations with explicit assumptions.
- Compare expected and actual values.
- Return deterministic, machine-readable outputs.

## Branches

- `agent/foundation`
- `agent/rust-compute-core`
- `agent/rust-cli`
- `agent/typescript-mcp-server`
- `agent/expression-engine`
- `agent/units`
- `agent/finance`
- `agent/verification`
- `agent/test-generation`
- `agent/docs`
- `agent/integration-fixes`

## Repository Structure

```text
apps/mcp-server-ts/      TypeScript MCP stdio wrapper
crates/compute-core/     Rust compute primitives
crates/compute-cli/      Rust CLI wrapper
docs/                    Architecture and interface documentation
examples/                Example requests and responses
schemas/                 JSON schemas for planned interfaces
```

## Engineering Rules

- Prefer small, reviewable changes.
- Preserve deterministic behavior over convenience.
- Keep public interfaces schema-backed and machine-readable.
- Document precision, rounding, tolerance, and financial assumptions.
- Add tests with feature work.
- Run relevant checks before handing off.

## Context7

Use Context7 MCP to fetch current documentation whenever asking about a library, framework, SDK, API, CLI tool, or cloud service. Start with `resolve-library-id` unless an exact `/org/project` library ID is provided, then query docs with the full user question.
