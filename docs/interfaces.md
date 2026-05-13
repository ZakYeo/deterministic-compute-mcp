# Planned Interfaces

The interfaces in this document are directional. They are not yet implemented.

## Compute Request

Planned requests should include:

- `operation`: stable operation identifier.
- `input`: operation-specific payload.
- `precision`: decimal precision and rounding options when relevant.
- `trace`: whether to return proof or step metadata.

## Compute Response

Planned responses should include:

- `ok`: success flag.
- `result`: deterministic operation result.
- `diagnostics`: warnings, assumptions, and validation messages.
- `trace`: optional proof steps.
- `version`: compute engine version.

## MCP Tools

Initial MCP tools are expected to map to stable compute domains:

- `compute_expression`
- `convert_units`
- `calculate_finance`
- `verify_result`
- `generate_expected_values`

Tool names and schemas should be finalized by the TypeScript MCP server worker.

## Finance Operations

Implemented generic compute operations include:

- `finance.simple-interest`
- `finance.compound-interest`
- `finance.loan-payment`
- `finance.percentage-change`
- `finance.margin-markup`
- `finance.cagr`

Finance rates are decimal rates per period, not percentage whole numbers. Loan payments assume fixed end-of-period payments and exclude fees, taxes, escrow, and prepayments. Loan `totalPaid` and `totalInterest` are computed from the displayed rounded payment and marked with `basis: "displayed-payment"`.

`finance.cagr` intentionally supports only roots exactly representable at the requested `precision.decimalPlaces`; non-exact roots return a structured `precision-issue` error.

## Verification Operations

Implemented generic verification operations include:

- `verification.compare`

`verification.compare` accepts `expected`, `actual`, and optional `tolerance`. Without tolerance it performs scale-normalized exact numeric equality. With tolerance it supports `absolute` tolerance and `relative` tolerance, where relative tolerance is applied to `abs(expected)`. The generic result `value` is the absolute difference; structured comparison status and tolerance metadata are emitted in `result.details`.
