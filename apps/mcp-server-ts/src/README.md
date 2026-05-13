# MCP Server Source

`index.ts` starts the SDK-backed MCP stdio server and registers the current deterministic compute tools.

The wrapper invokes the Rust compute CLI with JSON requests. Set `DETERMINISTIC_COMPUTE_CLI_COMMAND` to use a prebuilt CLI binary instead of the default `cargo run --manifest-path crates/compute-cli/Cargo.toml`, and set `DETERMINISTIC_COMPUTE_CLI_ARGS_JSON` to a JSON string array for extra command arguments.
