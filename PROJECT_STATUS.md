# Project Status

## Foundation Status

Branch: `agent/foundation`

Status: complete

The repository foundation has been created with:

- Root Rust workspace configuration.
- Compiling skeleton crates for `compute-core` and `compute-cli`.
- TypeScript MCP server scaffold under `apps/mcp-server-ts`.
- Initial README and agent instructions.
- Planning docs, example payloads, and draft schemas.

This work intentionally avoids major product implementation.

## Foundation Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed.
- `npm install --prefix apps/mcp-server-ts`: passed.
- `npm --prefix apps/mcp-server-ts run typecheck`: passed.
- `npm --prefix apps/mcp-server-ts run build`: passed.

## Rust Compute Core Status

Branch: `agent/rust-compute-core`

Status: complete and merged

Review history:

- Initial review: 74/100, changes requested for decimal overflow, checked division, scale validation, and edge-case coverage.
- Second review: 86/100, changes requested for high-scale decimal division cancellation and `i128::MIN` boundary handling.
- Third review: 88/100, changes requested for `i128::MIN` decimal wire round-trip.
- Final review: 94/100, passed. A recommended oversized-positive decimal regression test was added before merge.

The compute core now includes:

- JSON-serializable request, response, result, diagnostic, trace, and structured error models.
- Deterministic integer and fixed-scale decimal arithmetic primitives.
- Explicit precision policy and rounding modes.
- Basic add, subtract, multiply, and divide operations.
- Structured invalid-input, division-by-zero, precision, and overflow errors.
- Edge-case coverage for decimal scale validation, high-scale division cancellation, `i128` boundaries, and wire-format round trips.

Ownership note: `Cargo.lock` changed outside `crates/compute-core/**` because `compute-core` added `serde` and `serde_json` dependencies.

## Rust Compute Core Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 30 `compute-core` tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

## Rust CLI Status

Branch: `agent/rust-cli`

Status: complete and merged

Review history:

- Review: 91/100, passed with minor requested coverage improvements.
- Added file-input and subtract-operation regression tests before merge.
- Updated `--version` to report the CLI package version.

The Rust CLI now includes:

- JSON compute request input from stdin or a single request file.
- JSON `ComputeResponse` output to stdout.
- Arithmetic dispatch for add, subtract, multiply, and divide.
- Operand parsing through `compute-core` JSON-safe numeric types.
- Precision and rounding policy pass-through to `compute-core`.
- Structured compute errors as JSON responses.
- Runtime/CLI errors reported as nonzero exit failures.
- Minimal help and version commands.

Ownership note: `Cargo.lock` changed outside `crates/compute-cli/**` because `compute-cli` added direct `serde` and `serde_json` dependencies.

## Rust CLI Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 13 `compute-cli` tests and 30 `compute-core` tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.

## TypeScript MCP Server Status

Branch: `agent/typescript-mcp-server`

Status: complete and merged

Review history:

- Initial review: 88/100, changes requested for `structuredContent`, decimal scale validation, process hardening, and dependency pinning.
- Second review: 91/100, passed. New files were staged before merge.

The TypeScript MCP server now includes:

- SDK-backed MCP stdio server using `@modelcontextprotocol/server`.
- `compute_arithmetic` tool wired to the Rust compute CLI.
- `compute_expression` deterministic not-implemented response until the expression engine lands.
- Zod v4 input schemas for arithmetic, expression, precision, rounding, trace, and JSON-safe numeric values.
- Decimal value/scale validation aligned with `compute-core`.
- Machine-readable tool results through both JSON text content and `structuredContent`.
- Configurable CLI invocation with deterministic timeout, output-size, nonzero-exit, and invalid-JSON failure responses.
- Tests for schema validation, request building, structured tool content, CLI wrapper behavior, and process hardening.

Ownership note: all edits stayed under `apps/mcp-server-ts/**`.

## TypeScript MCP Server Checks

- `npm --prefix apps/mcp-server-ts run typecheck`: passed.
- `npm --prefix apps/mcp-server-ts run build`: passed.
- `npm --prefix apps/mcp-server-ts test`: passed.
- `cargo test -p compute-cli`: passed with 13 tests.

## Next Workstreams

- `agent/expression-engine`: add safe expression parsing, AST evaluation, and proof traces.
- `agent/units`: add unit conversion and dimensional analysis.
- `agent/finance`: add finance/business calculators with documented assumptions.
- `agent/verification`: add exact and tolerance-based result comparison.
- `agent/test-generation`: add deterministic expected-value generation.
- `agent/docs`: expand installation, usage, and integration docs.

## Coordination Notes

- Each worker should use its assigned branch and owned paths.
- Review should focus on deterministic behavior, schema clarity, tests, and integration boundaries.
- If a worker must edit outside its owned paths, note that deviation in its handoff.
