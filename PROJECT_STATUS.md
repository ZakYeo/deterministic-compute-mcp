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

## Next Workstreams

- `agent/rust-cli`: expose compute-core through a stable JSON CLI.
- `agent/typescript-mcp-server`: implement MCP SDK wiring, tool registration, schema validation, and CLI process integration.
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
