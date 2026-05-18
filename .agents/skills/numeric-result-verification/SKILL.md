---
name: numeric-result-verification
description: Deterministic numeric verification workflow for checking calculated answers, test outputs, examples, pricing logic, migrations, or agent-generated numbers. Use when Codex needs to compare expected and actual numeric values with exact equality, absolute tolerance, or relative tolerance through the deterministic-compute MCP server.
---

# Numeric Result Verification

Use deterministic-compute verification when correctness depends on a numeric comparison. Do not eyeball decimals or rely on approximate model reasoning.

## Workflow

1. Identify `expected` and `actual` values.
2. Encode both as JSON-safe numeric values:
   - integers as `{ "kind": "integer", "value": "42" }`
   - decimals as `{ "kind": "decimal", "value": "10.01", "scale": 2 }`
3. Choose comparison mode:
   - no tolerance for exact scale-normalized numeric equality
   - `absolute` tolerance for fixed allowed difference
   - `relative` tolerance for allowed difference based on `abs(expected) * tolerance`
4. Call the MCP `verify_result` tool when available. If working at the CLI layer, use `verification.compare`.
5. Report pass/fail, difference, tolerance mode, and the encoded values used.

## MCP Tool Shape

For `verify_result`, pass encoded numeric objects directly:

```json
{
  "expected": { "kind": "decimal", "value": "10.00", "scale": 2 },
  "actual": { "kind": "decimal", "value": "10.01", "scale": 2 },
  "tolerance": {
    "kind": "absolute",
    "value": { "kind": "decimal", "value": "0.02", "scale": 2 }
  },
  "trace": false
}
```

Always pass `expected`, `actual`, and `tolerance.value` as numeric objects. Never pass plain strings or bare numbers, even when the user states values in prose.

## Guidance

- Prefer exact comparison for deterministic fixtures, integer counts, and ledger-like outputs.
- Use absolute tolerance for displayed currency or rounded values.
- Use relative tolerance for ratios, rates, and scale-dependent values.
- Tolerances must be non-negative.
- Do not apply output `precision` to `verification.compare`; verification returns comparison details directly.
