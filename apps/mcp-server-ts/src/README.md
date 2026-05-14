# MCP Server Source

`index.ts` starts the SDK-backed MCP stdio server and registers the current deterministic compute tools.

The wrapper invokes the Rust compute CLI with JSON requests. It resolves the CLI in this order:

1. `DETERMINISTIC_COMPUTE_CLI_COMMAND`, with optional JSON string-array args from `DETERMINISTIC_COMPUTE_CLI_ARGS_JSON`.
2. A packaged native binary in `bin/compute-cli-<platform>-<arch>`.
3. The repository development fallback: `cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml --`.

This keeps `npx -y @deterministic-compute/mcp-server` self-contained for published packages while preserving local checkout development.
