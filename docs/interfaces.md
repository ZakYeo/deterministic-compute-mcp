# Interfaces

This document describes the current generic compute request/response contract and notes planned domains where they are still pending.

## Compute Request

Requests include:

- `operation`: stable operation identifier.
- `input`: operation-specific payload.
- `precision`: decimal precision and rounding options when relevant.
- `trace`: whether to return proof or step metadata.

## Compute Response

Responses include:

- `ok`: success flag.
- `result`: deterministic operation result.
- `diagnostics`: warnings, assumptions, and validation messages.
- `trace`: optional proof steps.
- `version`: compute engine version.

## MCP Tools

MCP tools map to stable compute domains:

- `compute_expression`
- `convert_units`
- `calculate_finance`
- `verify_result`
- `generate_expected_values`

Some planned domains may still be absent from the TypeScript MCP wrapper until their core implementation lands.

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

## Test Generation Operations

Implemented generic test generation operations include:

- `test-generation.generate-expected-values`

`test-generation.generate-expected-values` accepts a bounded `cases` array of explicit compute requests. Cases are evaluated in input order through the same core dispatcher used by CLI and MCP calls, so expected values reuse arithmetic, expression, finance, and verification behavior instead of duplicating formulas. The operation does not generate random inputs.

Bounds:

- `cases`: 1 to 100 cases.
- `maxCases`: optional caller limit from 1 to 100.
- `case.id`: non-empty and at most 128 UTF-8 bytes.
- `case.input`: serialized JSON must be at most 16384 bytes.

The generic result `value` is the generated case count; `result.details.cases` contains each case id, normalized request, and deterministic compute response. Generator trace metadata includes evaluated and failed case counts plus per-case id, operation, index, and success status without duplicating full nested responses. With `failOnCaseError: true`, the top-level error `detail` is a JSON object string containing `caseId`, `caseIndex`, `operation`, nested `error.code`, nested `error.message`, the nested response, and the number of cases generated before the failure.
