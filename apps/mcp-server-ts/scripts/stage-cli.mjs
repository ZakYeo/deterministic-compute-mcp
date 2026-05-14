#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(packageRoot, "../..");
const platform = process.platform;
const arch = process.arch;
const extension = platform === "win32" ? ".exe" : "";
const binaryName = `compute-cli-${platform}-${arch}${extension}`;
const source = path.join(repoRoot, "target/release", `compute-cli${extension}`);
const destination = path.join(packageRoot, "bin", binaryName);

const build = spawnSync("cargo", ["build", "--release", "-p", "compute-cli"], {
  cwd: repoRoot,
  stdio: "inherit",
});

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

mkdirSync(path.dirname(destination), { recursive: true });
copyFileSync(source, destination);

if (platform !== "win32") {
  chmodSync(destination, 0o755);
}

console.log(`staged ${destination}`);
