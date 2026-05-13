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

## Expression Engine Status

Branch: `agent/expression-engine`

Status: complete and merged

Review history:

- Initial review: 82/100, changes requested for precision timing, depth safety, unary `i128::MIN`, crate-private precision exposure, and tokenization edge cases.
- Second review: 91/100, passed with minor trace and test improvements requested.
- Added final precision trace coverage, a structured repeating-decimal helper, and stricter depth-guard test assertions before merge.

The expression engine now includes:

- Safe tokenizer and recursive-descent parser for numbers, `+`, `-`, `*`, `/`, parentheses, and unary minus.
- AST evaluation using existing compute-core numeric and arithmetic primitives.
- Final-result precision application with special handling for rounded repeating division.
- Structured invalid-input and precision errors for malformed expressions and unsupported tokens.
- Deterministic trace steps for expression arithmetic and final precision adjustment.
- Input length, token count, parser depth, and evaluator depth guards.
- Tests for precedence, parentheses, unary minus, decimals, division and rounding, precision cancellation, invalid tokens, depth limits, trace determinism, and `i128::MIN`.

Ownership note: changes stayed in `crates/compute-core/src/expression/**`, `crates/compute-core/src/precision/**`, and minimal `crates/compute-core/src/lib.rs` exposure.

## Expression Engine Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 48 `compute-core` tests and 13 `compute-cli` tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

## Units Status

Branch: `agent/units`

Status: complete and merged

Review history:

- Initial review: 78/100, changes requested for reduced linear conversion factors, affine temperature precision/metadata, case-sensitive symbols, and overflow coverage.
- Second review: 88/100, changes requested for final requested-precision scaling overflow and safer GCD handling.
- Third review: 88/100, changes requested for default exact decimal scaling overflow.
- Split child workstream review: 96/100, passed. The focused default exact scaling fix and identity metadata snapshot were added before merge.

The units module now includes:

- Deterministic conversions for length, mass, time, and temperature.
- Dimensional analysis with structured errors for incompatible dimensions and unknown units.
- Reduced rational source-to-target conversion factors for linear units.
- Exact affine temperature transforms with one final precision application.
- Case-sensitive temperature symbols (`C`, `F`, `K`) with lowercase full names accepted.
- Machine-readable conversion result, metadata, conversion kind, factor/scale/offset fields, assumptions, and deterministic trace steps.
- Tests for exact conversions, decimal factors, precision and rounding, invalid units, incompatible dimensions, temperature affine conversions, metadata serialization, overflow boundaries, and `i128::MIN` scaling.

Ownership note: changes stayed in `crates/compute-core/src/units/**` plus minimal `crates/compute-core/src/lib.rs` module exposure.

## Units Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 72 `compute-core` tests and 13 `compute-cli` tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

## Finance Status

Branch: `agent/finance`

Status: complete and merged

Review history:

- Initial review: 72/100, changes requested for public integration, CAGR exact-root overflow/cap behavior, loan summary semantics, and edge coverage.
- Second review: 91/100, passed. Added MCP CAGR precision validation before merge.

The finance module now includes:

- Deterministic finance and business calculators for simple interest, compound future value, fixed loan payment, percentage change, margin/markup, and exact-representable CAGR.
- Fixed decimal/rational arithmetic with checked overflow paths and no floating-point calculations.
- Machine-readable finance metadata, assumptions, deterministic trace steps, and loan summary details.
- Loan totals computed from the displayed rounded payment with `basis: "displayed-payment"`.
- Exact-only CAGR contract documented in code, docs, schemas, and MCP tool descriptions.
- Core generic request dispatch for arithmetic, expression, and finance operations.
- Rust CLI finance support through the generic JSON compute request path.
- TypeScript MCP `calculate_finance` tool, schemas, request builder, and tests.
- Updated interface docs, JSON schemas, and example request/response files.

Ownership note: the workstream expanded beyond `crates/compute-core/src/finance/**` to satisfy review-requested public integration through core dispatch, CLI, MCP, schemas, examples, and docs.

## Finance Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 92 `compute-core` tests and 14 `compute-cli` tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `npm --prefix apps/mcp-server-ts run typecheck`: passed.
- `npm --prefix apps/mcp-server-ts run build`: passed.
- `npm --prefix apps/mcp-server-ts test`: passed with 14 MCP tests.

## Verification Status

Branch: `agent/verification`

Status: complete and merged

Review history:

- Initial review: 88/100, changes requested for public JSON schema definitions, MCP negative tolerance validation, and edge-case coverage.
- Second review: 94/100, passed. Added a schema clarity note that `precision` is ignored for `verification.compare` before merge.

The verification module now includes:

- Public `verification.compare` operation for exact and tolerance-based numeric result verification.
- Scale-normalized exact comparisons for JSON-safe integer and decimal values.
- Absolute tolerance and relative tolerance using `abs(expected) * tolerance`.
- Machine-readable comparison status, mode, pass/fail flag, expected/actual values, difference, and tolerance details.
- Deterministic trace output with absolute difference as the generic result value.
- Structured invalid-input and overflow errors for negative tolerances and unrepresentable differences.
- Core generic request dispatch, Rust CLI support, MCP `verify_result` tool, and request builders.
- Public JSON schema definitions for verification input, tolerance variants, details, statuses, modes, and tolerance metadata.
- Tests for exact matches/mismatches, decimal scale normalization, tolerance pass/fail, zero/negative expected relative tolerance, exact equality with tolerance, overflow, CLI dispatch, MCP mapping, and schema contract coverage.

Ownership note: the workstream expanded beyond `crates/compute-core/src/verification/**` to expose the public operation through core dispatch, CLI, MCP, schemas, and interface docs.

## Verification Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 104 `compute-core` tests and 15 `compute-cli` tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `npm --prefix apps/mcp-server-ts run typecheck`: passed.
- `npm --prefix apps/mcp-server-ts run build`: passed.
- `npm --prefix apps/mcp-server-ts test`: passed with 19 MCP tests.

## Test Generation Status

Branch: `agent/test-generation`

Status: complete and merged

Review history:

- Initial review: 88/100, changes requested for structured fail-fast detail, size bounds, richer trace metadata, schema-contract tests, and documentation language.
- Second review: 93/100, passed. Added an MCP recursion schema regression test before merge.

The test generation module now includes:

- Public `test-generation.generate-expected-values` operation for deterministic expected-value generation.
- Explicit bounded case arrays evaluated in input order without randomness.
- Reuse of the core compute dispatcher for arithmetic, expression, finance, and verification behavior.
- Full per-case normalized request and deterministic compute response in `result.details.cases`.
- `failOnCaseError` support with structured JSON detail containing case id, index, operation, nested error, nested response, and generated case count.
- Hard bounds for max cases, case id byte length, and serialized case input size.
- Recursive generation case rejection in Rust and MCP schema boundaries.
- Optional trace metadata with per-case id/index/operation/status plus summary evaluated and failed counts.
- Core dispatch, Rust CLI support, MCP `generate_expected_values` tool, public schemas, docs, and example request coverage.
- Tests for repeatability, expression generation, recorded case failures, fail-fast detail, max case boundaries, size bounds, trace metadata, CLI dispatch, MCP mapping, recursion rejection, and schema contract coverage.

Ownership note: the workstream expanded beyond `crates/compute-core/src/test_generation/**` to expose the public operation through core dispatch, CLI, MCP, schemas, docs, and examples. Shared `TraceStep` gained optional metadata for higher-level operation traces.

## Test Generation Checks

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: passed with 113 `compute-core` tests and 17 `compute-cli` tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `npm --prefix apps/mcp-server-ts run typecheck`: passed.
- `npm --prefix apps/mcp-server-ts run build`: passed.
- `npm --prefix apps/mcp-server-ts test`: passed with 24 MCP tests.

## Next Workstreams

- `agent/docs`: expand installation, usage, and integration docs.

## Coordination Notes

- Each worker should use its assigned branch and owned paths.
- Review should focus on deterministic behavior, schema clarity, tests, and integration boundaries.
- If a worker must edit outside its owned paths, note that deviation in its handoff.
