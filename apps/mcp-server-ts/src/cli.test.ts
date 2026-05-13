import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  buildArithmeticRequest,
  buildFinanceRequest,
  buildTestGenerationRequest,
  buildVerificationRequest,
  invokeComputeCli,
  resolveCliCommand,
  runProcess,
  type ProcessRunner,
} from "./cli.js";
import {
  arithmeticToolInputSchema,
  financeToolInputSchema,
  testGenerationToolInputSchema,
  verificationToolInputSchema,
} from "./schemas.js";
import {
  buildExpressionToolResult,
  buildFinanceToolResult,
  buildTestGenerationToolResult,
  buildToolResult,
  buildVerificationToolResult,
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

test("finance schema documents exact CAGR precision contract", () => {
  const parsed = financeToolInputSchema.parse({
    operation: "cagr",
    beginningValue: { kind: "integer", value: "100" },
    endingValue: { kind: "integer", value: "121" },
    periods: 2,
    precision: { decimalPlaces: 2, rounding: "exact" },
    trace: true,
  });

  assert.equal(parsed.operation, "cagr");
  assert.throws(() =>
    financeToolInputSchema.parse({
      operation: "cagr",
      beginningValue: { kind: "integer", value: "100" },
      endingValue: { kind: "integer", value: "121" },
      periods: 2,
      precision: { rounding: "exact" },
    }),
  );
});

test("buildFinanceRequest maps MCP input to compute CLI request", () => {
  const request = buildFinanceRequest({
    operation: "loan-payment",
    principal: { kind: "integer", value: "1000" },
    periodicRate: { kind: "decimal", value: "0.01", scale: 2 },
    periods: 12,
    precision: { decimalPlaces: 2, rounding: "half-away-from-zero" },
    trace: false,
  });

  assert.deepEqual(request, {
    operation: "finance.loan-payment",
    input: {
      principal: { kind: "integer", value: "1000" },
      periodicRate: { kind: "decimal", value: "0.01", scale: 2 },
      periods: 12,
    },
    precision: { decimalPlaces: 2, rounding: "half-away-from-zero" },
    trace: false,
  });
});

test("verification schema accepts exact and tolerance comparisons", () => {
  const exact = verificationToolInputSchema.parse({
    expected: { kind: "integer", value: "42" },
    actual: { kind: "integer", value: "42" },
  });
  const tolerant = verificationToolInputSchema.parse({
    expected: { kind: "decimal", value: "100.00", scale: 2 },
    actual: { kind: "decimal", value: "100.01", scale: 2 },
    tolerance: {
      kind: "relative",
      value: { kind: "decimal", value: "0.001", scale: 3 },
    },
    trace: true,
  });

  assert.equal(exact.trace, false);
  assert.equal(tolerant.tolerance?.kind, "relative");
});

test("verification schema rejects negative tolerance values", () => {
  assert.throws(() =>
    verificationToolInputSchema.parse({
      expected: { kind: "integer", value: "42" },
      actual: { kind: "integer", value: "41" },
      tolerance: {
        kind: "absolute",
        value: { kind: "integer", value: "-1" },
      },
    }),
  );

  assert.throws(() =>
    verificationToolInputSchema.parse({
      expected: { kind: "decimal", value: "10.00", scale: 2 },
      actual: { kind: "decimal", value: "10.01", scale: 2 },
      tolerance: {
        kind: "relative",
        value: { kind: "decimal", value: "-0.01", scale: 2 },
      },
    }),
  );
});

test("public JSON schemas define verification compare contract", () => {
  const requestSchema = JSON.parse(
    fs.readFileSync(
      path.join(process.cwd(), "../../schemas/compute-request.schema.json"),
      "utf8",
    ),
  );
  const responseSchema = JSON.parse(
    fs.readFileSync(
      path.join(process.cwd(), "../../schemas/compute-response.schema.json"),
      "utf8",
    ),
  );

  assert.ok(requestSchema.$defs.NumericValue);
  assert.ok(requestSchema.$defs.VerificationCompareInput);
  assert.ok(requestSchema.$defs.VerificationTolerance);
  assert.equal(
    requestSchema.$defs.VerificationCompareInput.properties.tolerance.$ref,
    "#/$defs/VerificationTolerance",
  );
  assert.ok(responseSchema.$defs.VerificationDetails);
  assert.deepEqual(responseSchema.$defs.ComparisonStatus.enum, [
    "exact-match",
    "exact-mismatch",
    "within-tolerance",
    "outside-tolerance",
  ]);
  assert.equal(
    responseSchema.$defs.ToleranceDetails.properties.allowedDifference.$ref,
    "#/$defs/NumericValue",
  );
});

test("public JSON schemas define expected-value generation contract", () => {
  const requestSchema = JSON.parse(
    fs.readFileSync(
      path.join(process.cwd(), "../../schemas/compute-request.schema.json"),
      "utf8",
    ),
  );
  const responseSchema = JSON.parse(
    fs.readFileSync(
      path.join(process.cwd(), "../../schemas/compute-response.schema.json"),
      "utf8",
    ),
  );

  assert.ok(requestSchema.$defs.GenerateExpectedValuesInput);
  assert.ok(requestSchema.$defs.ExpectedValueCaseSpec);
  assert.equal(
    requestSchema.$defs.GenerateExpectedValuesInput.properties.cases.items.$ref,
    "#/$defs/ExpectedValueCaseSpec",
  );
  assert.equal(
    requestSchema.$defs.ExpectedValueCaseSpec.properties.id.maxLength,
    128,
  );
  assert.equal(
    requestSchema.$defs.GenerateExpectedValuesInput.properties.cases.maxItems,
    100,
  );
  assert.deepEqual(
    requestSchema.$defs.ExpectedValueCaseSpec.properties.operation.enum,
    [
      "arithmetic.add",
      "arithmetic.subtract",
      "arithmetic.multiply",
      "arithmetic.divide",
      "expression.evaluate",
      "finance.simple-interest",
      "finance.compound-interest",
      "finance.loan-payment",
      "finance.percentage-change",
      "finance.margin-markup",
      "finance.cagr",
      "verification.compare",
    ],
  );
  assert.ok(responseSchema.$defs.GeneratedExpectedValuesDetails);
  assert.equal(
    responseSchema.$defs.GeneratedExpectedValuesDetails.properties.cases.items.$ref,
    "#/$defs/GeneratedExpectedValueCase",
  );
  assert.equal(
    responseSchema.$defs.GeneratedExpectedValuesDetails.properties.caseCount.maximum,
    100,
  );
  assert.equal(
    responseSchema.$defs.TraceStep.properties.metadata.$ref,
    "#/$defs/TraceMetadata",
  );
});

test("buildVerificationRequest maps MCP input to compute CLI request", () => {
  const request = buildVerificationRequest({
    expected: { kind: "integer", value: "100" },
    actual: { kind: "integer", value: "101" },
    tolerance: {
      kind: "absolute",
      value: { kind: "integer", value: "1" },
    },
    trace: false,
  });

  assert.deepEqual(request, {
    operation: "verification.compare",
    input: {
      expected: { kind: "integer", value: "100" },
      actual: { kind: "integer", value: "101" },
      tolerance: {
        kind: "absolute",
        value: { kind: "integer", value: "1" },
      },
    },
    trace: false,
  });
});

test("test generation schema accepts bounded deterministic cases", () => {
  const parsed = testGenerationToolInputSchema.parse({
    cases: [
      {
        id: "addition",
        operation: "arithmetic.add",
        input: {
          left: { kind: "integer", value: "20" },
          right: { kind: "integer", value: "22" },
        },
      },
    ],
    maxCases: 1,
  });

  assert.equal(parsed.trace, false);
  assert.equal(parsed.failOnCaseError, false);
  assert.equal(parsed.cases[0]?.trace, false);
});

test("test generation schema rejects case counts above maxCases", () => {
  assert.throws(() =>
    testGenerationToolInputSchema.parse({
      maxCases: 1,
      cases: [
        { id: "a", operation: "arithmetic.add", input: {} },
        { id: "b", operation: "arithmetic.add", input: {} },
      ],
    }),
  );
});

test("test generation schema enforces id and serialized input bounds", () => {
  assert.throws(() =>
    testGenerationToolInputSchema.parse({
      cases: [
        {
          id: "x".repeat(129),
          operation: "arithmetic.add",
          input: {},
        },
      ],
    }),
  );

  assert.throws(() =>
    testGenerationToolInputSchema.parse({
      cases: [
        {
          id: "large-input",
          operation: "expression.evaluate",
          input: { expression: "x".repeat(16 * 1024) },
        },
      ],
    }),
  );
});

test("test generation schema rejects recursive generation cases", () => {
  assert.throws(() =>
    testGenerationToolInputSchema.parse({
      cases: [
        {
          id: "recursive",
          operation: "test-generation.generate-expected-values",
          input: { cases: [] },
        },
      ],
    }),
  );
});

test("buildTestGenerationRequest maps MCP input to compute CLI request", () => {
  const request = buildTestGenerationRequest({
    cases: [
      {
        id: "addition",
        operation: "arithmetic.add",
        input: {
          left: { kind: "integer", value: "20" },
          right: { kind: "integer", value: "22" },
        },
        trace: false,
      },
    ],
    failOnCaseError: false,
    trace: true,
  });

  assert.deepEqual(request, {
    operation: "test-generation.generate-expected-values",
    input: {
      cases: [
        {
          id: "addition",
          operation: "arithmetic.add",
          input: {
            left: { kind: "integer", value: "20" },
            right: { kind: "integer", value: "22" },
          },
          trace: false,
        },
      ],
      failOnCaseError: false,
    },
    trace: true,
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

test("buildFinanceToolResult invokes compute CLI with finance request", async () => {
  let capturedInput = "";
  const runner: ProcessRunner = async (_command, _args, input) => {
    capturedInput = input;
    return {
      exitCode: 0,
      stdout: JSON.stringify({
        ok: true,
        result: { operation: JSON.parse(input).operation },
        version: "0.1.0",
      }),
      stderr: "",
    };
  };

  const result = await buildFinanceToolResult(
    {
      operation: "simple-interest",
      principal: { kind: "integer", value: "1000" },
      periodicRate: { kind: "decimal", value: "0.05", scale: 2 },
      periods: 3,
      trace: false,
    },
    runner,
    { command: "compute-cli", args: [] },
  );
  const structuredContent = result.structuredContent as ToolPayload;

  assert.equal(structuredContent.tool, "calculate_finance");
  assert.equal(structuredContent.request.operation, "finance.simple-interest");
  assert.match(capturedInput, /"operation":"finance.simple-interest"/);
  assert.equal(structuredContent.response.ok, true);
});

test("buildVerificationToolResult invokes compute CLI with verification request", async () => {
  let capturedInput = "";
  const runner: ProcessRunner = async (_command, _args, input) => {
    capturedInput = input;
    return {
      exitCode: 0,
      stdout: JSON.stringify({
        ok: true,
        result: { operation: JSON.parse(input).operation },
        version: "0.1.0",
      }),
      stderr: "",
    };
  };

  const result = await buildVerificationToolResult(
    {
      expected: { kind: "integer", value: "5" },
      actual: { kind: "integer", value: "6" },
      tolerance: {
        kind: "absolute",
        value: { kind: "integer", value: "1" },
      },
      trace: false,
    },
    runner,
    { command: "compute-cli", args: [] },
  );
  const structuredContent = result.structuredContent as ToolPayload;

  assert.equal(structuredContent.tool, "verify_result");
  assert.equal(structuredContent.request.operation, "verification.compare");
  assert.match(capturedInput, /"operation":"verification.compare"/);
  assert.equal(structuredContent.response.ok, true);
});

test("buildTestGenerationToolResult invokes compute CLI with generation request", async () => {
  let capturedInput = "";
  const runner: ProcessRunner = async (_command, _args, input) => {
    capturedInput = input;
    return {
      exitCode: 0,
      stdout: JSON.stringify({
        ok: true,
        result: { operation: JSON.parse(input).operation },
        version: "0.1.0",
      }),
      stderr: "",
    };
  };

  const result = await buildTestGenerationToolResult(
    {
      cases: [
        {
          id: "addition",
          operation: "arithmetic.add",
          input: {
            left: { kind: "integer", value: "20" },
            right: { kind: "integer", value: "22" },
          },
          trace: false,
        },
      ],
      failOnCaseError: false,
      trace: false,
    },
    runner,
    { command: "compute-cli", args: [] },
  );
  const structuredContent = result.structuredContent as ToolPayload;

  assert.equal(structuredContent.tool, "generate_expected_values");
  assert.equal(
    structuredContent.request.operation,
    "test-generation.generate-expected-values",
  );
  assert.match(capturedInput, /"operation":"test-generation.generate-expected-values"/);
  assert.equal(structuredContent.response.ok, true);
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
