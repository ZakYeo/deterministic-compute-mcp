# @deterministic-compute/mcp-server

Deterministic compute MCP stdio server for Codex and other MCP clients. It exposes exact arithmetic, expressions, unit conversion, finance calculators, result verification, and expected-value generation through the Rust `compute-cli`.

## Codex Install

Install the server into Codex with one command:

```sh
codex mcp add deterministic-compute -- npx -y @deterministic-compute/mcp-server
```

Restart Codex, then run `/mcp` in the TUI or:

```sh
codex mcp list
```

Codex stores the server in `~/.codex/config.toml` as a local stdio MCP server:

```toml
[mcp_servers.deterministic-compute]
command = "npx"
args = ["-y", "@deterministic-compute/mcp-server"]
```

## Requirements

- Node.js `>=20`.
- Linux x64 for the current packaged native `compute-cli` binary.

Other platforms can still use the server by building `compute-cli` from this repository and pointing the MCP wrapper at it:

```sh
export DETERMINISTIC_COMPUTE_CLI_COMMAND="/absolute/path/to/compute-cli"
export DETERMINISTIC_COMPUTE_CLI_ARGS_JSON="[]"
```

## Local Development

From the repository root:

```sh
npm ci --prefix apps/mcp-server-ts
npm --prefix apps/mcp-server-ts run build
node apps/mcp-server-ts/dist/index.js
```

When no packaged native binary is present, the server falls back to:

```sh
cargo run --quiet --manifest-path crates/compute-cli/Cargo.toml --
```

## Release Packaging

The current test package stages the Linux x64 Rust CLI binary and builds the TypeScript server:

```sh
npm --prefix apps/mcp-server-ts run build:package
npm --prefix apps/mcp-server-ts pack --dry-run
```

For now, the package includes:

- `compute-cli-linux-x64`

Future general releases should add the remaining platform binaries before documenting broad `npx` support.
