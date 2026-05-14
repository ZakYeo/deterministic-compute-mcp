# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust-first deterministic compute engine with a TypeScript MCP wrapper.

- `crates/compute-core/`: deterministic arithmetic, expression, units, finance, verification, and expected-value logic.
- `crates/compute-cli/`: JSON CLI boundary around `compute-core`.
- `apps/mcp-server-ts/`: TypeScript MCP stdio server, schemas, tool mapping, and MCP tests.
- `schemas/`: JSON Schema contracts for compute request and response envelopes.
- `examples/`: runnable JSON request/response fixtures.
- `docs/`: architecture, interface, and roadmap notes.

## Build, Test, and Development Commands

Run Rust checks from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p compute-cli
```

Run the CLI against an example request:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/compute-request.json
```

Install and validate the MCP server:

```sh
npm ci --prefix apps/mcp-server-ts
npm --prefix apps/mcp-server-ts run typecheck
npm --prefix apps/mcp-server-ts run build
npm --prefix apps/mcp-server-ts test
```

Use `npm --prefix apps/mcp-server-ts run dev` for local MCP server development.

## Coding Style & Naming Conventions

Rust uses edition 2021 with workspace lints. `unsafe_code` is forbidden, and Clippy denies `unwrap`, `expect`, and `panic`; return structured errors instead. Keep Rust modules focused by domain and use snake_case for functions, modules, and tests.

TypeScript is strict ESM (`module: NodeNext`, target `ES2022`). Keep source in `apps/mcp-server-ts/src/`, import local modules with `.js` extensions, and prefer explicit schemas/types at tool boundaries.

## Testing Guidelines

Place Rust unit tests in each module’s `mod tests` block and name tests after behavior, for example `division_requires_precision_for_repeating_decimal`. TypeScript tests use Node’s built-in `node:test` and live beside source as `*.test.ts`; build before running because tests execute from `dist/**/*.test.js`.

Cover changes at the boundary they affect: core computation in Rust, CLI JSON behavior in `compute-cli`, and MCP validation/mapping in TypeScript.

## Commit & Pull Request Guidelines

Recent commits use short, imperative summaries such as `Close MVP integration gaps` and `Update user documentation and examples`. Follow that style: one concise subject, no trailing period.

Pull requests should describe the behavioral change, list validation commands run, and call out schema, example, or MCP contract changes. Link related issues when available and include sample JSON or CLI output for user-visible computation changes.

## Security & Configuration Tips

Do not commit local MCP client configuration or secrets. If overriding the CLI used by the MCP server, prefer absolute paths via `DETERMINISTIC_COMPUTE_CLI_COMMAND` and JSON arguments via `DETERMINISTIC_COMPUTE_CLI_ARGS_JSON`.
