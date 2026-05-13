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
