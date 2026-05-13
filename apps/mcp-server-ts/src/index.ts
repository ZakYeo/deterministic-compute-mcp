#!/usr/bin/env node

import process from "node:process";

type JsonRpcRequest = {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: unknown;
};

type JsonRpcResponse = {
  jsonrpc: "2.0";
  id: string | number | null;
  result?: unknown;
  error?: {
    code: number;
    message: string;
  };
};

const serverInfo = {
  name: "deterministic-compute-mcp",
  version: "0.1.0",
};

function createResponse(request: JsonRpcRequest): JsonRpcResponse | undefined {
  const id = request.id ?? null;

  if (request.method === "initialize") {
    return {
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: "2025-03-26",
        capabilities: {
          tools: {},
        },
        serverInfo,
      },
    };
  }

  if (request.method === "tools/list") {
    return {
      jsonrpc: "2.0",
      id,
      result: {
        tools: [],
      },
    };
  }

  if (request.method?.startsWith("notifications/")) {
    return undefined;
  }

  return {
    jsonrpc: "2.0",
    id,
    error: {
      code: -32601,
      message: "Method not implemented in foundation scaffold",
    },
  };
}

function writeResponse(response: JsonRpcResponse | undefined): void {
  if (!response) {
    return;
  }

  process.stdout.write(`${JSON.stringify(response)}\n`);
}

let buffer = "";

process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buffer += chunk;

  for (;;) {
    const newlineIndex = buffer.indexOf("\n");
    if (newlineIndex === -1) {
      break;
    }

    const line = buffer.slice(0, newlineIndex).trim();
    buffer = buffer.slice(newlineIndex + 1);

    if (line.length === 0) {
      continue;
    }

    try {
      const request = JSON.parse(line) as JsonRpcRequest;
      writeResponse(createResponse(request));
    } catch {
      writeResponse({
        jsonrpc: "2.0",
        id: null,
        error: {
          code: -32700,
          message: "Parse error",
        },
      });
    }
  }
});
