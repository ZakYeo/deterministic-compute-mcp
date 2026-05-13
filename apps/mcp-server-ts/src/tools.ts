import type { CallToolResult } from "@modelcontextprotocol/server";

import {
  buildArithmeticRequest,
  buildFinanceRequest,
  buildTestGenerationRequest,
  buildVerificationRequest,
  invokeComputeCli,
  type CliCommand,
  type CliResult,
  type ComputeRequest,
  type ProcessRunner,
} from "./cli.js";
import type {
  ArithmeticToolInput,
  ExpressionToolInput,
  FinanceToolInput,
  TestGenerationToolInput,
  VerificationToolInput,
} from "./schemas.js";

export type ToolPayload = {
  tool:
    | "compute_arithmetic"
    | "compute_expression"
    | "calculate_finance"
    | "generate_expected_values"
    | "verify_result";
  request: ComputeRequest | ExpressionComputeRequest;
  response: CliResult | ExpressionFailure;
};

export type ExpressionFailure = {
  ok: false;
  error: {
    code: "not-implemented";
    message: string;
  };
  version: "mcp-wrapper";
};

export type ExpressionComputeRequest = {
  operation: "expression.evaluate";
  input: {
    expression: string;
  };
  precision?: ExpressionToolInput["precision"];
  trace: boolean;
};

export function buildToolResult(payload: ToolPayload): CallToolResult {
  return {
    content: [
      {
        type: "text",
        text: JSON.stringify(payload, null, 2),
      },
    ],
    structuredContent: payload,
  };
}

export async function buildArithmeticToolResult(
  input: ArithmeticToolInput,
): Promise<CallToolResult> {
  const request = buildArithmeticRequest(input);
  const response = await invokeComputeCli(request);

  return buildToolResult({
    tool: "compute_arithmetic",
    request,
    response,
  });
}

export async function buildFinanceToolResult(
  input: FinanceToolInput,
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildFinanceRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

  return buildToolResult({
    tool: "calculate_finance",
    request,
    response,
  });
}

export async function buildVerificationToolResult(
  input: VerificationToolInput,
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildVerificationRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

  return buildToolResult({
    tool: "verify_result",
    request,
    response,
  });
}

export async function buildTestGenerationToolResult(
  input: TestGenerationToolInput,
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildTestGenerationRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

  return buildToolResult({
    tool: "generate_expected_values",
    request,
    response,
  });
}

export function buildExpressionToolResult(
  input: ExpressionToolInput,
): CallToolResult {
  const request: ExpressionComputeRequest = {
    operation: "expression.evaluate",
    input: {
      expression: input.expression,
    },
    trace: input.trace ?? false,
  };

  if (input.precision) {
    request.precision = input.precision;
  }

  return buildToolResult({
    tool: "compute_expression",
    request,
    response: {
      ok: false,
      error: {
        code: "not-implemented",
        message: "expression.evaluate is not wired through the current MCP wrapper",
      },
      version: "mcp-wrapper",
    },
  });
}
