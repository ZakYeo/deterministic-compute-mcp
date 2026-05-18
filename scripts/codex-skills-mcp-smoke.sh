#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
mcp_package_dir="$repo_root/apps/mcp-server-ts"
server_entry="$mcp_package_dir/dist/index.js"
compute_cli="$repo_root/target/debug/compute-cli"
validator="$HOME/.codex/skills/.system/skill-creator/scripts/quick_validate.py"
result_file="$(mktemp)"
transcript_file="$(mktemp)"

cleanup() {
  rm -f "$result_file" "$transcript_file"
}
trap cleanup EXIT

for command_name in codex cargo node npm python3; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: $command_name" >&2
    exit 1
  fi
done

if [ ! -f "$validator" ]; then
  echo "missing skill validator: $validator" >&2
  exit 1
fi

if [ ! -d "$mcp_package_dir/node_modules" ]; then
  echo "missing MCP server dependencies; run: npm ci --prefix apps/mcp-server-ts" >&2
  exit 1
fi

echo "validating repo-local skills..."
for skill_name in deterministic-finance numeric-result-verification expected-value-generation; do
  python3 "$validator" "$repo_root/.agents/skills/$skill_name"
done

echo "building local compute CLI..."
cargo build -p compute-cli

echo "building local MCP server..."
npm --prefix "$mcp_package_dir" run build

echo "running Codex with repo skills and transient deterministic-compute-local MCP config..."
prompt='Use $deterministic-finance, $numeric-result-verification, and $expected-value-generation with the deterministic-compute-local MCP server.

Calculate VAT for a net amount of 100 at 20%, with results shown to 2 decimal places.
Verify whether 10.01 is within an absolute tolerance of 0.02 from 10.00.
Generate deterministic expected values for a single addition case: 20 plus 22.

Use the relevant MCP tool for each skill task. Follow each skill'\''s MCP Tool Shape section exactly. Do not explain the MCP request shapes. Reply briefly with the final result from each skill.'

codex exec \
  -C "$repo_root" \
  --output-last-message "$result_file" \
  -c 'mcp_servers.deterministic-compute-local.command="node"' \
  -c "mcp_servers.deterministic-compute-local.args=[\"$server_entry\"]" \
  -c "mcp_servers.deterministic-compute-local.env.DETERMINISTIC_COMPUTE_CLI_COMMAND=\"$compute_cli\"" \
  -c 'mcp_servers.deterministic-compute-local.env.DETERMINISTIC_COMPUTE_CLI_ARGS_JSON="[]"' \
  -c 'mcp_servers.deterministic-compute-local.tools.calculate_finance.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.verify_result.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.generate_expected_values.approval_mode="approve"' \
  "$prompt" 2>&1 | tee "$transcript_file"

echo "validating Codex skill smoke output..."
if grep -q 'mcp: deterministic-compute-local/.*(failed)' "$transcript_file"; then
  echo "one or more deterministic-compute-local MCP calls failed" >&2
  sed 's/^/  /' "$transcript_file" >&2
  exit 1
fi

normalized_result="$(tr '[:upper:]' '[:lower:]' < "$result_file" | tr -d '[:space:]')"

for expected in "120.00" "20.00" "pass" "42"; do
  if [[ "$normalized_result" != *"$expected"* ]]; then
    echo "missing expected skill smoke output: $expected" >&2
    echo "last Codex message:" >&2
    sed 's/^/  /' "$result_file" >&2
    exit 1
  fi
done

echo "Codex skills + MCP smoke test passed."
