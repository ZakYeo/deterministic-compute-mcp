# Examples

This directory contains runnable compute request payloads for the Rust CLI plus one sample response.

Run one example:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml -- examples/arithmetic-request.json
```

Files:

- `arithmetic-request.json`: rounded deterministic division.
- `expression-request.json`: arithmetic expression evaluation.
- `units-request.json`: deterministic unit conversion.
- `compute-request.json`: fixed loan payment with trace output.
- `vat-request.json`: VAT calculation from net amount and decimal VAT rate.
- `compute-response.json`: sample response for `compute-request.json`.
- `verification-request.json`: absolute-tolerance comparison.
- `generate-expected-values-request.json`: expected-value generation across arithmetic, expression, units, finance, and verification cases.

All numeric values are JSON-safe tagged values. Decimal `scale` must match the fractional digit count in `value`.
