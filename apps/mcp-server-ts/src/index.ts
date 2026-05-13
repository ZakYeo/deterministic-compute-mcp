#!/usr/bin/env node

import { McpServer, StdioServerTransport } from "@modelcontextprotocol/server";

import {
  arithmeticToolInputSchema,
  expressionToolInputSchema,
  financeToolInputSchema,
  testGenerationToolInputSchema,
  verificationToolInputSchema,
  type ArithmeticToolInput,
  type ExpressionToolInput,
  type FinanceToolInput,
  type TestGenerationToolInput,
  type VerificationToolInput,
} from "./schemas.js";
import {
  buildArithmeticToolResult,
  buildExpressionToolResult,
  buildFinanceToolResult,
  buildTestGenerationToolResult,
  buildVerificationToolResult,
} from "./tools.js";

const server = new McpServer({
  name: "@deterministic-compute/mcp-server",
  version: "0.1.0",
});

server.registerTool(
  "compute_arithmetic",
  {
    description:
      "Run deterministic binary arithmetic through the Rust compute CLI.",
    inputSchema: arithmeticToolInputSchema,
  },
  async (input: ArithmeticToolInput) => buildArithmeticToolResult(input),
);

server.registerTool(
  "compute_expression",
  {
    description:
      "Expression evaluation placeholder. The expression engine is not registered in the accepted CLI yet.",
    inputSchema: expressionToolInputSchema,
  },
  async (input: ExpressionToolInput) => buildExpressionToolResult(input),
);

server.registerTool(
  "calculate_finance",
  {
    description:
      "Run deterministic finance and business calculators through the Rust compute CLI. CAGR supports only exact roots representable at the requested decimalPlaces.",
    inputSchema: financeToolInputSchema,
  },
  async (input: FinanceToolInput) => buildFinanceToolResult(input),
);

server.registerTool(
  "verify_result",
  {
    description:
      "Verify deterministic numeric results exactly or with absolute/relative tolerance through the Rust compute CLI.",
    inputSchema: verificationToolInputSchema,
  },
  async (input: VerificationToolInput) => buildVerificationToolResult(input),
);

server.registerTool(
  "generate_expected_values",
  {
    description:
      "Generate deterministic expected values for supported numeric compute operations through the Rust compute CLI.",
    inputSchema: testGenerationToolInputSchema,
  },
  async (input: TestGenerationToolInput) => buildTestGenerationToolResult(input),
);

const transport = new StdioServerTransport();
await server.connect(transport);
