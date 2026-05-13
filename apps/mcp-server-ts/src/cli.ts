import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import type { ArithmeticToolInput } from "./schemas.js";

export type ComputeRequest = {
  operation: string;
  input: {
    left: ArithmeticToolInput["operands"][0];
    right: ArithmeticToolInput["operands"][1];
  };
  precision?: ArithmeticToolInput["precision"];
  trace: boolean;
};

export type ComputeResponse = {
  ok: boolean;
  result?: unknown;
  diagnostics?: unknown[];
  trace?: unknown[];
  error?: {
    code: string;
    message: string;
    detail?: string;
  };
  version: string;
};

export type CliFailure = {
  ok: false;
  error: {
    code:
      | "cli-execution-failed"
      | "cli-invalid-json"
      | "cli-timeout"
      | "cli-output-too-large";
    message: string;
    detail?: string;
  };
  version: "mcp-wrapper";
};

export type CliResult = ComputeResponse | CliFailure;

export type CliCommand = {
  command: string;
  args: string[];
};

export type ProcessResult = {
  exitCode: number | null;
  stdout: string;
  stderr: string;
  timedOut?: boolean;
  outputTooLarge?: boolean;
};

export type ProcessRunner = (
  command: string,
  args: string[],
  input: string,
) => Promise<ProcessResult>;

const operationMap = {
  add: "arithmetic.add",
  subtract: "arithmetic.subtract",
  multiply: "arithmetic.multiply",
  divide: "arithmetic.divide",
} as const;

export function buildArithmeticRequest(input: ArithmeticToolInput): ComputeRequest {
  const request: ComputeRequest = {
    operation: operationMap[input.operation],
    input: {
      left: input.operands[0],
      right: input.operands[1],
    },
    trace: input.trace ?? false,
  };

  if (input.precision) {
    request.precision = input.precision;
  }

  return request;
}

export function resolveCliCommand(env: NodeJS.ProcessEnv = process.env): CliCommand {
  if (env.DETERMINISTIC_COMPUTE_CLI_COMMAND) {
    return {
      command: env.DETERMINISTIC_COMPUTE_CLI_COMMAND,
      args: parseArgsJson(env.DETERMINISTIC_COMPUTE_CLI_ARGS_JSON),
    };
  }

  return {
    command: "cargo",
    args: [
      "run",
      "--quiet",
      "--manifest-path",
      path.join(repoRoot(), "crates/compute-cli/Cargo.toml"),
      "--",
    ],
  };
}

export async function invokeComputeCli(
  request: ComputeRequest,
  runner: ProcessRunner = runProcess,
  commandConfig: CliCommand = resolveCliCommand(),
): Promise<CliResult> {
  const input = `${JSON.stringify(request)}\n`;
  const processResult = await runner(commandConfig.command, commandConfig.args, input);

  if (processResult.timedOut) {
    return {
      ok: false,
      error: {
        code: "cli-timeout",
        message: "compute CLI timed out",
        detail: processResult.stderr.trim() || undefined,
      },
      version: "mcp-wrapper",
    };
  }

  if (processResult.outputTooLarge) {
    return {
      ok: false,
      error: {
        code: "cli-output-too-large",
        message: "compute CLI output exceeded the configured limit",
        detail: processResult.stderr.trim() || undefined,
      },
      version: "mcp-wrapper",
    };
  }

  if (processResult.exitCode !== 0) {
    return {
      ok: false,
      error: {
        code: "cli-execution-failed",
        message: "compute CLI exited unsuccessfully",
        detail: processResult.stderr.trim() || `exit code ${processResult.exitCode}`,
      },
      version: "mcp-wrapper",
    };
  }

  try {
    return JSON.parse(processResult.stdout) as ComputeResponse;
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "cli-invalid-json",
        message: "compute CLI returned invalid JSON",
        detail: error instanceof Error ? error.message : String(error),
      },
      version: "mcp-wrapper",
    };
  }
}

export function runProcess(
  command: string,
  args: string[],
  input: string,
  options: {
    timeoutMs?: number;
    maxOutputBytes?: number;
  } = {},
): Promise<ProcessResult> {
  const timeoutMs = options.timeoutMs ?? 10_000;
  const maxOutputBytes = options.maxOutputBytes ?? 1_048_576;

  return new Promise((resolve) => {
    const child = spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    let outputBytes = 0;
    let settled = false;
    let timedOut = false;
    let outputTooLarge = false;

    const finish = (result: ProcessResult) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeout);
      resolve(result);
    };

    const failAndKill = (reason: "timeout" | "output") => {
      if (reason === "timeout") {
        timedOut = true;
        stderr = stderr
          ? `${stderr}\nprocess exceeded ${timeoutMs}ms timeout`
          : `process exceeded ${timeoutMs}ms timeout`;
      } else {
        outputTooLarge = true;
        stderr = stderr
          ? `${stderr}\nprocess output exceeded ${maxOutputBytes} byte limit`
          : `process output exceeded ${maxOutputBytes} byte limit`;
      }

      child.kill("SIGTERM");
      setTimeout(() => {
        if (!settled) {
          child.kill("SIGKILL");
        }
      }, 100).unref();
    };

    const appendOutput = (stream: "stdout" | "stderr", chunk: string) => {
      outputBytes += Buffer.byteLength(chunk);
      if (outputBytes > maxOutputBytes && !outputTooLarge) {
        failAndKill("output");
      }

      if (stream === "stdout") {
        stdout += chunk;
      } else {
        stderr += chunk;
      }
    };

    const timeout = setTimeout(() => {
      failAndKill("timeout");
    }, timeoutMs);
    timeout.unref();

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      appendOutput("stdout", chunk);
    });
    child.stderr.on("data", (chunk: string) => {
      appendOutput("stderr", chunk);
    });
    child.on("error", (error) => {
      finish({
        exitCode: 1,
        stdout,
        stderr: stderr ? `${stderr}\n${error.message}` : error.message,
        timedOut,
        outputTooLarge,
      });
    });
    child.on("close", (exitCode) => {
      if (!outputTooLarge) {
        outputTooLarge =
          Buffer.byteLength(stdout) + Buffer.byteLength(stderr) > maxOutputBytes;
      }
      finish({ exitCode, stdout, stderr, timedOut, outputTooLarge });
    });

    child.stdin.end(input);
  });
}

function parseArgsJson(value: string | undefined): string[] {
  if (!value) {
    return [];
  }

  const parsed = JSON.parse(value) as unknown;
  if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "string")) {
    throw new Error("DETERMINISTIC_COMPUTE_CLI_ARGS_JSON must be a JSON string array");
  }
  return parsed;
}

function repoRoot(): string {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}
