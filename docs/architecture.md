# Architecture

`deterministic-compute-mcp` is organized around a deterministic Rust compute core with thin process and MCP integration layers.

## Layers

- `crates/compute-core`: deterministic domain logic, JSON-friendly request/response models, numeric parsing, precision/rounding, arithmetic, expression evaluation, finance calculators, verification, unit conversion primitives, and expected-value generation.
- `crates/compute-cli`: stable JSON process boundary. It reads one compute request from stdin or a file and writes one compute response to stdout.
- `apps/mcp-server-ts`: TypeScript MCP stdio server. It validates agent-facing tool inputs with Zod, maps them to compute requests, invokes the CLI, and returns JSON text plus `structuredContent`.
- `schemas`: JSON Schema documents for the generic request and response contracts.
- `examples`: request payloads that can be run through the CLI.

## Integration Boundaries

- The Rust core owns deterministic behavior, checked arithmetic, precision rules, error codes, and operation dispatch.
- The CLI owns process-level input/output and converts invalid JSON into structured compute errors.
- The MCP wrapper owns tool names, tool input schemas, process timeout/output limits, and wrapper-level failures such as `cli-timeout` or `cli-invalid-json`.
- Public payloads should stay schema-backed and use string-encoded numeric values to avoid host JSON number drift.

## Current Public Dispatcher

The generic Rust dispatcher accepts:

- `arithmetic.add`
- `arithmetic.subtract`
- `arithmetic.multiply`
- `arithmetic.divide`
- `expression.evaluate`
- `units.convert`
- `finance.simple-interest`
- `finance.compound-interest`
- `finance.loan-payment`
- `finance.vat`
- `finance.percentage-change`
- `finance.margin-markup`
- `finance.cagr`
- `verification.compare`
- `test-generation.generate-expected-values`

The TypeScript MCP wrapper exposes arithmetic, expression evaluation, unit conversion, finance, verification, and expected-value generation through the CLI.

## Determinism Rules

- Numeric inputs are tagged integers or fixed-scale decimals serialized as strings.
- Decimal `scale` must match the fractional digit count in `value`.
- Arithmetic avoids floating-point calculations.
- Precision is explicit through `decimalPlaces` and `rounding`.
- Failed exact division, non-exact CAGR roots, overflow, and invalid inputs return structured JSON errors.
- Finance and verification responses include machine-readable assumptions or details where relevant.
