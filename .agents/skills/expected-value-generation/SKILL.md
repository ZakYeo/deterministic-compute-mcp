---
name: expected-value-generation
description: Deterministic expected-value generation workflow for tests, golden fixtures, examples, schema fixtures, and regression cases using deterministic-compute MCP. Use when Codex needs to create repeatable expected outputs for arithmetic, expression, unit conversion, finance, or verification cases.
---

# Expected Value Generation

Use deterministic-compute to generate expected values for explicit test cases. The generator evaluates caller-provided cases; it does not invent random inputs.

## Workflow

1. Write explicit bounded cases with stable IDs.
2. Use supported operations: arithmetic, expression, units, finance, or verification.
3. Encode numeric inputs as JSON-safe values and include precision policies where results are displayed.
4. Call the MCP `generate_expected_values` tool when available. If working at the CLI layer, use `test-generation.generate-expected-values`.
5. Store or paste only the relevant deterministic response fields for the target test fixture.

## MCP Tool Shape

For `generate_expected_values`, pass the case list directly:

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
  "trace": false
}
```

## Case Design

- Keep case IDs short, descriptive, and stable.
- Include edge cases that exercise precision, rounding, units, tolerances, and finance assumptions.
- Set `failOnCaseError` to `true` when a fixture must fail fast on any invalid case.
- Set it to `false` when recording both successful and intentionally failing cases.
- Avoid recursive expected-value generation cases.

## Review Checklist

- Cases are explicit and deterministic.
- The same dispatcher behavior is used as production compute calls.
- Generated values include enough metadata to explain precision and operation assumptions.
- Fixtures do not contain irrelevant trace output unless the trace itself is under test.
