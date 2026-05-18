---
name: deterministic-finance
description: Exact finance and business calculation workflow for deterministic-compute MCP. Use when Codex needs to calculate, explain, verify, or create examples for VAT, simple interest, compound interest, fixed loan payments, percentage change, margin, markup, or CAGR with deterministic precision and structured numeric outputs.
---

# Deterministic Finance

Use the deterministic-compute MCP server for finance math instead of mental arithmetic or floating-point snippets.

## Workflow

1. Identify the finance operation: `simple-interest`, `compound-interest`, `loan-payment`, `vat`, `percentage-change`, `margin-markup`, or `cagr`.
2. Convert all numbers to JSON-safe numeric values:
   - integers as `{ "kind": "integer", "value": "1000" }`
   - decimals as `{ "kind": "decimal", "value": "0.20", "scale": 2 }`
3. Use decimal rates, not percentage whole numbers. For example, 20% is `0.20`.
4. Provide an explicit precision policy whenever a displayed decimal result is expected.
5. Call the MCP `calculate_finance` tool when available. If working at the CLI layer, use the matching `finance.*` operation.
6. Return the machine result plus a short explanation of inputs, rounding, and assumptions.

## MCP Tool Shape

For `calculate_finance`, pass the plain tool operation name and operation-specific fields:

```json
{
  "operation": "vat",
  "netAmount": { "kind": "integer", "value": "100" },
  "vatRate": { "kind": "decimal", "value": "0.20", "scale": 2 },
  "precision": { "decimalPlaces": 2, "rounding": "exact" },
  "trace": false
}
```

Other finance operations use the same JSON-safe numeric values and decimal rates. Use `simple-interest`, `compound-interest`, `loan-payment`, `percentage-change`, `margin-markup`, or `cagr` as the tool operation name.

## Operation Notes

- VAT starts from a non-negative net amount and decimal VAT rate; report net, VAT, and gross amounts.
- Loan payments are fixed end-of-period payments. Totals are based on the displayed rounded payment.
- CAGR requires explicit decimal places and only succeeds when the root is exactly representable at the requested scale.
- Do not use binary floating-point math to "check" deterministic-compute outputs.

## Review Checklist

- Rates are decimal rates per period.
- Decimal `scale` matches the fractional digits in `value`.
- Rounding mode is intentional: `exact`, `truncate`, or `half-away-from-zero`.
- Returned explanation does not hide precision issues or failed exactness checks.
