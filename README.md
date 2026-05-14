# deterministic-compute-mcp

`deterministic-compute-mcp` is a Rust-first deterministic computation engine with a TypeScript MCP wrapper for agent workflows. It gives Codex users a local tool for exact numeric calculations, finance/business math, result verification, and expected-value generation with machine-readable JSON output.

The project currently implements the Rust compute core, JSON CLI, and MCP stdio wrapper for the exposed operations below.

## Implemented Status

- Rust core: JSON-safe integer and fixed-scale decimal arithmetic, expression evaluation, unit conversion, finance calculators, verification, and expected-value generation.
- Rust CLI: reads a compute request from stdin or a file and writes a `ComputeResponse` JSON document.
- TypeScript MCP server: exposes agent tools for arithmetic, expressions, unit conversion, finance, verification, and expected-value generation through the Rust CLI.
- Schemas/examples/docs: describe the generic request/response contracts and runnable example payloads.

## Architecture

```text
apps/mcp-server-ts/      TypeScript MCP stdio wrapper
crates/compute-core/     Rust deterministic compute primitives and dispatcher
crates/compute-cli/      Rust JSON CLI process boundary
docs/                    Architecture and interface documentation
examples/                Runnable JSON compute requests and sample response
schemas/                 JSON schemas for compute requests and responses
```

The Rust core owns computation correctness. The CLI owns the stable process contract. The MCP server owns agent-facing tool registration, input validation, and CLI invocation.

## Prerequisites

- Rust toolchain compatible with workspace `rust-version = "1.76"`.
- Node.js `>=20` and npm for the MCP server.

Install TypeScript dependencies:

```sh
npm ci --prefix apps/mcp-server-ts
```

## Build And Test

Rust:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p compute-cli
```

TypeScript MCP server:

```sh
npm --prefix apps/mcp-server-ts run typecheck
npm --prefix apps/mcp-server-ts run build
npm --prefix apps/mcp-server-ts test
```

## CLI Usage

Run with a request file:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/compute-request.json
```

Run with stdin:

```sh
printf '%s\n' '{"operation":"arithmetic.add","input":{"left":{"kind":"integer","value":"20"},"right":{"kind":"integer","value":"22"}}}' \
  | cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml --
```

After building the CLI, you can run the binary directly:

```sh
target/debug/compute-cli examples/compute-request.json
```

Requests have this shape:

```json
{
  "operation": "arithmetic.divide",
  "input": {
    "left": { "kind": "integer", "value": "2" },
    "right": { "kind": "integer", "value": "3" }
  },
  "precision": {
    "decimalPlaces": 2,
    "rounding": "half-away-from-zero"
  },
  "trace": true
}
```

The JSON Schema validates the generic request envelope and includes operation-specific branches for units, VAT, verification, and expected-value generation. Arithmetic, expression, and other finance input payloads are validated by the Rust core and MCP schemas at runtime.

Numbers are JSON-safe tagged values:

- Integer: `{ "kind": "integer", "value": "42" }`
- Decimal: `{ "kind": "decimal", "value": "0.05", "scale": 2 }`

`scale` must equal the number of fractional digits in the decimal string. Supported rounding modes are `exact`, `truncate`, and `half-away-from-zero`.

## Supported CLI Operations

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

Finance rates are decimal rates per period, not percentage whole numbers. VAT rates are also decimal rates; for example, 20% VAT is `0.20`. `finance.vat` computes from a non-negative net amount and returns machine-readable `netAmount`, `vatAmount`, and `grossAmount` details. When output rounding is requested, `grossAmount` is the displayed `netAmount + vatAmount` so returned components stay internally consistent. CAGR requires `precision.decimalPlaces` and returns a `precision-issue` error when the root is not exactly representable at that scale.

`verification.compare` returns the absolute difference as `result.value` and comparison metadata in `result.details`. `test-generation.generate-expected-values` evaluates bounded explicit cases through the same dispatcher and returns nested deterministic responses in `result.details.cases`.

## MCP Server Usage

### Install In Codex

After the npm package is published, add the MCP server to Codex with:

```sh
codex mcp add deterministic-compute -- npx -y @deterministic-compute/mcp-server
```

Restart Codex, then run `/mcp` in the TUI or `codex mcp list` to verify the server is enabled. Codex stores the entry in `~/.codex/config.toml`:

```toml
[mcp_servers.deterministic-compute]
command = "npx"
args = ["-y", "@deterministic-compute/mcp-server"]
```

The current npm package build is intended for Linux x64 testing and ships the TypeScript stdio server plus `compute-cli-linux-x64`. Other platforms can still use the MCP wrapper by building `compute-cli` locally and setting `DETERMINISTIC_COMPUTE_CLI_COMMAND` to its absolute path.

### Local Checkout

Build the server:

```sh
npm --prefix apps/mcp-server-ts run build
```

Run the stdio server:

```sh
node apps/mcp-server-ts/dist/index.js
```

By default the wrapper invokes:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml --
```

To use a prebuilt CLI binary, set:

```sh
export DETERMINISTIC_COMPUTE_CLI_COMMAND="$PWD/target/debug/compute-cli"
export DETERMINISTIC_COMPUTE_CLI_ARGS_JSON='[]'
```

Use an absolute CLI path when an MCP client may launch the server from a different working directory.

Registered MCP tools:

- `compute_arithmetic`: `add`, `subtract`, `multiply`, `divide`.
- `compute_expression`: deterministic arithmetic expressions.
- `convert_units`: deterministic unit conversion.
- `calculate_finance`: simple interest, compound interest, loan payment, VAT, percentage change, margin/markup, CAGR.
- `verify_result`: exact, absolute-tolerance, and relative-tolerance comparisons.
- `generate_expected_values`: deterministic expected values for supported compute operations.

Example MCP tool input for arithmetic:

```json
{
  "operation": "divide",
  "operands": [
    { "kind": "integer", "value": "2" },
    { "kind": "integer", "value": "3" }
  ],
  "precision": {
    "decimalPlaces": 2,
    "rounding": "half-away-from-zero"
  },
  "trace": true
}
```

Tool results include both JSON text content and `structuredContent` with the wrapper tool name, generated CLI request, and CLI response.

## Examples

Runnable request files are in `examples/`:

- `arithmetic-request.json`
- `expression-request.json`
- `units-request.json`
- `vat-request.json`
- `compute-request.json`
- `verification-request.json`
- `generate-expected-values-request.json`

Run any request with:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/arithmetic-request.json
```

See [docs/interfaces.md](docs/interfaces.md) for operation-specific payloads and [docs/architecture.md](docs/architecture.md) for integration boundaries.
