import assert from "node:assert/strict";
import test from "node:test";

import {
  buildArithmeticRequest,
  invokeComputeCli,
  resolveCliCommand,
  runProcess,
  type ProcessRunner,
} from "./cli.js";
import { arithmeticToolInputSchema } from "./schemas.js";
import {
  buildExpressionToolResult,
  buildToolResult,
  type ToolPayload,
} from "./tools.js";

test("arithmetic schema accepts deterministic decimal operands", () => {
  const parsed = arithmeticToolInputSchema.parse({
    operation: "divide",
    operands: [
      { kind: "decimal", value: "10.00", scale: 2 },
      { kind: "integer", value: "4" },
    ],
    precision: { decimalPlaces: 2, rounding: "half-away-from-zero" },
    trace: true,
  });

  assert.equal(parsed.operation, "divide");
});

test("arithmetic schema rejects decimal values whose fractional digits do not match scale", () => {
  assert.throws(() =>
    arithmeticToolInputSchema.parse({
      operation: "add",
      operands: [
        { kind: "decimal", value: "1.2", scale: 2 },
        { kind: "integer", value: "4" },
      ],
    }),
  );

  const parsed = arithmeticToolInputSchema.parse({
    operation: "add",
    operands: [
      { kind: "decimal", value: "1.20", scale: 2 },
      { kind: "decimal", value: "5", scale: 0 },
    ],
  });

  assert.equal(parsed.operands[0].kind, "decimal");
});

test("buildArithmeticRequest maps MCP input to compute CLI request", () => {
  const request = buildArithmeticRequest({
    operation: "add",
    operands: [
      { kind: "integer", value: "2" },
      { kind: "integer", value: "3" },
    ],
    trace: false,
  });

  assert.deepEqual(request, {
    operation: "arithmetic.add",
    input: {
      left: { kind: "integer", value: "2" },
      right: { kind: "integer", value: "3" },
    },
    trace: false,
  });
});

test("buildToolResult returns matching structuredContent and JSON text content", () => {
  const payload = {
    tool: "compute_arithmetic" as const,
    request: {
      operation: "arithmetic.add",
      input: {
        left: { kind: "integer" as const, value: "1" },
        right: { kind: "integer" as const, value: "2" },
      },
      trace: false,
    },
    response: { ok: true, version: "0.1.0" },
  };

  const result = buildToolResult(payload);

  assert.deepEqual(result.structuredContent, payload);
  assert.deepEqual(JSON.parse(result.content[0]?.type === "text" ? result.content[0].text : ""), payload);
});

test("buildExpressionToolResult returns a structured not-implemented response", () => {
  const result = buildExpressionToolResult({
    expression: "1 + 2",
    trace: true,
  });
  const structuredContent = result.structuredContent as ToolPayload;

  assert.equal(structuredContent.tool, "compute_expression");
  assert.equal(structuredContent.response.ok, false);
  assert.equal(structuredContent.response.error?.code, "not-implemented");
});

test("invokeComputeCli sends JSON request to configured runner", async () => {
  let capturedInput = "";
  const runner: ProcessRunner = async (_command, _args, input) => {
    capturedInput = input;
    return {
      exitCode: 0,
      stdout: JSON.stringify({ ok: true, result: {}, version: "0.1.0" }),
      stderr: "",
    };
  };

  const response = await invokeComputeCli(
    {
      operation: "arithmetic.multiply",
      input: {
        left: { kind: "integer", value: "6" },
        right: { kind: "integer", value: "7" },
      },
      trace: true,
    },
    runner,
    { command: "compute-cli", args: [] },
  );

  assert.equal(response.ok, true);
  assert.match(capturedInput, /"operation":"arithmetic.multiply"/);
});

test("invokeComputeCli maps nonzero CLI exit to deterministic failure", async () => {
  const runner: ProcessRunner = async () => ({
    exitCode: 2,
    stdout: "",
    stderr: "bad request",
  });

  const response = await invokeComputeCli(
    {
      operation: "arithmetic.divide",
      input: {
        left: { kind: "integer", value: "1" },
        right: { kind: "integer", value: "0" },
      },
      trace: false,
    },
    runner,
    { command: "compute-cli", args: [] },
  );

  assert.equal(response.ok, false);
  assert.equal(response.error?.code, "cli-execution-failed");
});

test("invokeComputeCli maps invalid JSON to deterministic failure", async () => {
  const runner: ProcessRunner = async () => ({
    exitCode: 0,
    stdout: "not json",
    stderr: "",
  });

  const response = await invokeComputeCli(
    {
      operation: "arithmetic.add",
      input: {
        left: { kind: "integer", value: "1" },
        right: { kind: "integer", value: "2" },
      },
      trace: false,
    },
    runner,
    { command: "compute-cli", args: [] },
  );

  assert.equal(response.ok, false);
  assert.equal(response.error?.code, "cli-invalid-json");
});

test("invokeComputeCli maps timeout and oversize process results to deterministic failures", async () => {
  const timeoutRunner: ProcessRunner = async () => ({
    exitCode: null,
    stdout: "",
    stderr: "timed out",
    timedOut: true,
  });
  const oversizeRunner: ProcessRunner = async () => ({
    exitCode: null,
    stdout: "",
    stderr: "too much output",
    outputTooLarge: true,
  });
  const request = {
    operation: "arithmetic.add",
    input: {
      left: { kind: "integer" as const, value: "1" },
      right: { kind: "integer" as const, value: "2" },
    },
    trace: false,
  };

  const timeoutResponse = await invokeComputeCli(request, timeoutRunner, {
    command: "compute-cli",
    args: [],
  });
  const oversizeResponse = await invokeComputeCli(request, oversizeRunner, {
    command: "compute-cli",
    args: [],
  });

  assert.equal(timeoutResponse.ok, false);
  assert.equal(timeoutResponse.error?.code, "cli-timeout");
  assert.equal(oversizeResponse.ok, false);
  assert.equal(oversizeResponse.error?.code, "cli-output-too-large");
});

test("runProcess enforces timeout and max output limits", async () => {
  const timeoutResult = await runProcess(
    process.execPath,
    ["-e", "setTimeout(() => {}, 1000)"],
    "",
    { timeoutMs: 20 },
  );
  const oversizeResult = await runProcess(
    process.execPath,
    ["--version"],
    "",
    { maxOutputBytes: 0 },
  );

  assert.equal(timeoutResult.timedOut, true);
  assert.equal(oversizeResult.outputTooLarge, true);
});

test("resolveCliCommand uses configurable command and JSON args", () => {
  const command = resolveCliCommand({
    DETERMINISTIC_COMPUTE_CLI_COMMAND: "/bin/compute-cli",
    DETERMINISTIC_COMPUTE_CLI_ARGS_JSON: "[\"--flag\"]",
  });

  assert.deepEqual(command, {
    command: "/bin/compute-cli",
    args: ["--flag"],
  });
});
