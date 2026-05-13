# Interfaces

This document describes the current compute request/response contract used by the Rust CLI and the TypeScript MCP wrapper.

## Generic Compute Request

```json
{
  "operation": "arithmetic.add",
  "input": {
    "left": { "kind": "integer", "value": "20" },
    "right": { "kind": "integer", "value": "22" }
  },
  "precision": {
    "decimalPlaces": 2,
    "rounding": "half-away-from-zero"
  },
  "trace": false
}
```

Fields:

- `operation`: stable operation identifier.
- `input`: operation-specific payload.
- `precision`: optional decimal output policy for operations that produce numeric values.
- `trace`: optional deterministic step metadata.

The public JSON Schema currently validates the generic envelope and fully specializes verification and expected-value generation payloads. Arithmetic, expression, and finance payloads are documented below and validated by the Rust and MCP runtime layers.

## Numeric Values

```json
{ "kind": "integer", "value": "42" }
```

```json
{ "kind": "decimal", "value": "0.05", "scale": 2 }
```

Integers and decimals are serialized as strings. Decimal `scale` must equal the number of fractional digits in `value`. The maximum decimal scale is 38.

Precision policy:

```json
{
  "decimalPlaces": 2,
  "rounding": "half-away-from-zero"
}
```

Supported rounding modes:

- `exact`: reject values that cannot be represented exactly at the requested scale.
- `truncate`: drop excess fractional places toward zero.
- `half-away-from-zero`: round halves away from zero.

## Generic Compute Response

```json
{
  "ok": true,
  "result": {
    "operation": "arithmetic.add",
    "value": { "kind": "integer", "value": "42" },
    "metadata": {
      "engineVersion": "0.1.0",
      "numericKind": "integer",
      "precision": { "rounding": "exact" },
      "deterministic": true
    }
  },
  "diagnostics": [],
  "version": "0.1.0"
}
```

Failures use:

```json
{
  "ok": false,
  "error": {
    "code": "division-by-zero",
    "message": "division by zero"
  },
  "diagnostics": [],
  "version": "0.1.0"
}
```

Stable core error codes are `invalid-input`, `division-by-zero`, `precision-issue`, and `overflow`.

## Arithmetic Operations

Operations:

- `arithmetic.add`
- `arithmetic.subtract`
- `arithmetic.multiply`
- `arithmetic.divide`

Input:

```json
{
  "left": { "kind": "integer", "value": "2" },
  "right": { "kind": "integer", "value": "3" }
}
```

Exact division without a terminating decimal expansion returns `precision-issue` unless a precision policy uses `truncate` or `half-away-from-zero`.

## Expression Operation

Operation:

- `expression.evaluate`

Input:

```json
{
  "expression": "(2 + 3) * 4"
}
```

The Rust CLI supports numeric literals, `+`, `-`, `*`, `/`, parentheses, and unary minus. It applies the same deterministic arithmetic and final precision policy as the core. The MCP `compute_expression` tool is not yet wired to the CLI and returns a wrapper-level not-implemented response.

## Finance Operations

Operations:

- `finance.simple-interest`
- `finance.compound-interest`
- `finance.loan-payment`
- `finance.percentage-change`
- `finance.margin-markup`
- `finance.cagr`

Finance rates are decimal rates per period, not percentage whole numbers.

Simple interest and compound interest:

```json
{
  "principal": { "kind": "integer", "value": "1000" },
  "periodicRate": { "kind": "decimal", "value": "0.05", "scale": 2 },
  "periods": 2
}
```

Loan payment:

```json
{
  "principal": { "kind": "integer", "value": "1000" },
  "periodicRate": { "kind": "decimal", "value": "0.01", "scale": 2 },
  "periods": 12
}
```

Loan payments assume fixed end-of-period payments. Fees, taxes, escrow, and prepayments are excluded. `totalPaid` and `totalInterest` are computed from the displayed rounded payment and use `basis: "displayed-payment"`.

Percentage change:

```json
{
  "oldValue": { "kind": "integer", "value": "80" },
  "newValue": { "kind": "integer", "value": "100" }
}
```

Margin/markup:

```json
{
  "cost": { "kind": "integer", "value": "60" },
  "revenue": { "kind": "integer", "value": "100" }
}
```

CAGR:

```json
{
  "beginningValue": { "kind": "integer", "value": "100" },
  "endingValue": { "kind": "integer", "value": "121" },
  "periods": 2
}
```

CAGR requires an explicit `precision.decimalPlaces` policy. It intentionally supports only roots exactly representable at the requested scale; non-exact roots return `precision-issue`.

## Verification Operation

Operation:

- `verification.compare`

Input:

```json
{
  "expected": { "kind": "decimal", "value": "10.00", "scale": 2 },
  "actual": { "kind": "decimal", "value": "10.01", "scale": 2 },
  "tolerance": {
    "kind": "absolute",
    "value": { "kind": "decimal", "value": "0.02", "scale": 2 }
  }
}
```

Without `tolerance`, verification performs scale-normalized exact numeric equality. With tolerance it supports:

- `absolute`: difference must be less than or equal to the tolerance value.
- `relative`: difference must be less than or equal to `abs(expected) * tolerance`.

The generic result `value` is the absolute difference. `result.details` contains `status`, `passed`, `mode`, `expected`, `actual`, `difference`, and optional tolerance metadata.

## Test Generation Operation

Operation:

- `test-generation.generate-expected-values`

Input:

```json
{
  "cases": [
    {
      "id": "integer-addition",
      "operation": "arithmetic.add",
      "input": {
        "left": { "kind": "integer", "value": "20" },
        "right": { "kind": "integer", "value": "22" }
      }
    }
  ],
  "failOnCaseError": false,
  "maxCases": 100
}
```

Cases are evaluated in input order through the same core dispatcher used by normal CLI and MCP calls. The operation does not generate random inputs.

Bounds:

- `cases`: 1 to 100 cases.
- `maxCases`: optional caller limit from 1 to 100.
- `case.id`: non-empty and at most 128 UTF-8 bytes.
- `case.input`: serialized JSON must be at most 16384 bytes.
- Recursive `test-generation.generate-expected-values` cases are rejected.

The generic result `value` is the generated case count. `result.details.cases` contains each case id, normalized request, and deterministic compute response. With `failOnCaseError: true`, the top-level error `detail` is a JSON string containing the failing case id, index, operation, nested error, nested response, and generated-case count before failure.

## MCP Tools

Registered tools:

- `compute_arithmetic`
- `calculate_finance`
- `verify_result`
- `generate_expected_values`
- `compute_expression`

`compute_arithmetic`, `calculate_finance`, `verify_result`, and `generate_expected_values` invoke the Rust CLI and return both JSON text content and `structuredContent`:

```json
{
  "tool": "compute_arithmetic",
  "request": {
    "operation": "arithmetic.add",
    "input": {
      "left": { "kind": "integer", "value": "20" },
      "right": { "kind": "integer", "value": "22" }
    },
    "trace": false
  },
  "response": {
    "ok": true
  }
}
```

`compute_expression` currently returns:

```json
{
  "ok": false,
  "error": {
    "code": "not-implemented",
    "message": "expression.evaluate is not implemented by the current Rust CLI"
  },
  "version": "mcp-wrapper"
}
```

That message is stale relative to the Rust CLI; the integration boundary is that the wrapper has not yet been updated to invoke the CLI for expressions.
