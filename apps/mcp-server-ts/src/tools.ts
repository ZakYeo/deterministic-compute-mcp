import type { CallToolResult } from "@modelcontextprotocol/server";

import {
  buildArithmeticRequest,
  buildExpressionRequest,
  buildFinanceRequest,
  buildTestGenerationRequest,
  buildUnitConversionRequest,
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
  UnitConversionToolInput,
  VerificationToolInput,
} from "./schemas.js";

export type ToolPayload = {
  tool:
    | "compute_arithmetic"
    | "compute_expression"
    | "convert_units"
    | "calculate_finance"
    | "generate_expected_values"
    | "verify_result";
  request: ComputeRequest;
  response: CliResult;
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
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildArithmeticRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

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

export async function buildExpressionToolResult(
  input: ExpressionToolInput,
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildExpressionRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

  return buildToolResult({
    tool: "compute_expression",
    request,
    response,
  });
}

export async function buildUnitConversionToolResult(
  input: UnitConversionToolInput,
  runner?: ProcessRunner,
  commandConfig?: CliCommand,
): Promise<CallToolResult> {
  const request = buildUnitConversionRequest(input);
  const response = await invokeComputeCli(request, runner, commandConfig);

  return buildToolResult({
    tool: "convert_units",
    request,
    response,
  });
}
