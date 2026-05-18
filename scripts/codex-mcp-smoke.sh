#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
mcp_package_dir="$repo_root/apps/mcp-server-ts"
server_entry="$mcp_package_dir/dist/index.js"
compute_cli="$repo_root/target/debug/compute-cli"
result_file="$(mktemp)"

cleanup() {
  rm -f "$result_file"
}
trap cleanup EXIT

for command_name in codex cargo node npm; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: $command_name" >&2
    exit 1
  fi
done

if [ ! -d "$mcp_package_dir/node_modules" ]; then
  echo "missing MCP server dependencies; run: npm ci --prefix apps/mcp-server-ts" >&2
  exit 1
fi

echo "building local compute CLI..."
cargo build -p compute-cli

echo "building local MCP server..."
npm --prefix "$mcp_package_dir" run build

echo "running Codex with transient deterministic-compute-local MCP config..."
prompt='Use the deterministic-compute-local MCP server tools to run this smoke suite. Call every tool listed below exactly once with the exact JSON input shown.

1. compute_arithmetic: {"operation":"add","operands":[{"kind":"integer","value":"20"},{"kind":"integer","value":"22"}],"trace":false}
2. compute_expression: {"expression":"(2 + 3) * 4","trace":false}
3. convert_units: {"value":{"kind":"integer","value":"100"},"sourceUnit":"cm","targetUnit":"m","precision":{"decimalPlaces":2,"rounding":"exact"},"trace":false}
4. calculate_finance: {"operation":"compound-interest","principal":{"kind":"integer","value":"1000"},"periodicRate":{"kind":"decimal","value":"0.05","scale":2},"periods":2,"precision":{"decimalPlaces":2,"rounding":"exact"},"trace":false}
5. verify_result: {"expected":{"kind":"integer","value":"5"},"actual":{"kind":"integer","value":"6"},"tolerance":{"kind":"absolute","value":{"kind":"integer","value":"1"}},"trace":false}
6. generate_expected_values: {"cases":[{"id":"addition","operation":"arithmetic.add","input":{"left":{"kind":"integer","value":"20"},"right":{"kind":"integer","value":"22"}},"trace":false}],"failOnCaseError":false,"trace":false}

Return only compact JSON with this shape:
{"compute_arithmetic":"<value>","compute_expression":"<value>","convert_units":"<value>","calculate_finance":"<value>","verify_result":"<value>","generate_expected_values":"<value>"}
Use each tool response result.value.value where present. For verify_result, use result.details.passed. For generate_expected_values, use result.value.value.'

codex exec \
  -C "$repo_root" \
  --output-last-message "$result_file" \
  -c 'mcp_servers.deterministic-compute-local.command="node"' \
  -c "mcp_servers.deterministic-compute-local.args=[\"$server_entry\"]" \
  -c "mcp_servers.deterministic-compute-local.env.DETERMINISTIC_COMPUTE_CLI_COMMAND=\"$compute_cli\"" \
  -c 'mcp_servers.deterministic-compute-local.env.DETERMINISTIC_COMPUTE_CLI_ARGS_JSON="[]"' \
  -c 'mcp_servers.deterministic-compute-local.tools.compute_arithmetic.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.compute_expression.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.convert_units.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.calculate_finance.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.verify_result.approval_mode="approve"' \
  -c 'mcp_servers.deterministic-compute-local.tools.generate_expected_values.approval_mode="approve"' \
  "$prompt"

echo "validating Codex smoke output..."
normalized_result="$(tr -d '[:space:]' < "$result_file")"
for expected in \
  '"compute_arithmetic":"42"' \
  '"compute_expression":"20"' \
  '"convert_units":"1.00"' \
  '"calculate_finance":"1102.50"' \
  '"verify_result":"true"' \
  '"generate_expected_values":"1"'
do
  if [[ "$normalized_result" != *"$expected"* ]]; then
    echo "missing expected smoke output: $expected" >&2
    echo "last Codex message:" >&2
    sed 's/^/  /' "$result_file" >&2
    exit 1
  fi
done

echo "Codex MCP smoke test passed."
