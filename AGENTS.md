# Repository Guidelines

This is a Rust-first deterministic compute engine with a TypeScript MCP stdio wrapper.

## Layout

- `crates/compute-core/`: arithmetic, expressions, units, finance, verification, and expected-value logic.
- `crates/compute-cli/`: JSON CLI boundary around `compute-core`.
- `apps/mcp-server-ts/`: MCP server, schemas, tool mapping, and tests.
- `schemas/`: public JSON Schema contracts.
- `examples/`: runnable request/response fixtures.
- `docs/`: architecture, interface, and roadmap notes.

## Commands

Rust:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p compute-cli
```

MCP server:

```sh
npm ci --prefix apps/mcp-server-ts
npm --prefix apps/mcp-server-ts run typecheck
npm --prefix apps/mcp-server-ts run build
npm --prefix apps/mcp-server-ts test
```

Run an example:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/compute-request.json
```

## Style

- Rust uses edition 2021, focused domain modules, snake_case names, and structured errors.
- `unsafe_code` is forbidden; avoid `unwrap`, `expect`, and `panic` in production code.
- TypeScript is strict ESM with `module: NodeNext`; keep local imports using `.js` extensions.
- Keep schemas and tool-boundary types explicit.

## Tests

- Put Rust unit tests in each module's `mod tests`.
- Put TypeScript tests beside source as `*.test.ts`; build before running because tests execute from `dist/**/*.test.js`.
- Cover the boundary affected by the change: Rust core, CLI JSON behavior, MCP mapping, schemas, or examples.

## Security And Config

- Do not commit local MCP client config or secrets.
- Prefer absolute paths in `DETERMINISTIC_COMPUTE_CLI_COMMAND`.
- Use `DETERMINISTIC_COMPUTE_CLI_ARGS_JSON` for JSON CLI arguments.
