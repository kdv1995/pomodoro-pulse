#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";

function run(command, args) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    shell: false,
  });
  if (result.error) {
    console.error(`Failed to run command '${command}': ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function readFlagValue(args, flag) {
  const index = args.indexOf(flag);
  if (index === -1) {
    return null;
  }
  return args[index + 1] ?? null;
}

const args = process.argv.slice(2);
const command = args[0];

if (command === "build") {
  const isHelp = args.includes("--help") || args.includes("-h");
  if (!isHelp) {
    const prepareArgs = [];
    const target = readFlagValue(args, "--target");
    if (target) {
      prepareArgs.push("--target", target);
    }
    run(process.execPath, ["./scripts/prepare_pp_binary.mjs", ...prepareArgs]);
  }
}

const npmExecPath = process.env.npm_execpath;
if (!npmExecPath) {
  console.error("npm_execpath is not available in environment.");
  process.exit(1);
}
run(process.execPath, [npmExecPath, "exec", "tauri", "--", ...args]);
