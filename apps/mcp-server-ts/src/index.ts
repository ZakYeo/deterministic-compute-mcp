#!/usr/bin/env node

import { McpServer, StdioServerTransport } from "@modelcontextprotocol/server";

import {
  arithmeticToolInputSchema,
  expressionToolInputSchema,
  financeToolInputSchema,
  type ArithmeticToolInput,
  type ExpressionToolInput,
  type FinanceToolInput,
} from "./schemas.js";
import {
  buildArithmeticToolResult,
  buildExpressionToolResult,
  buildFinanceToolResult,
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

const transport = new StdioServerTransport();
await server.connect(transport);
