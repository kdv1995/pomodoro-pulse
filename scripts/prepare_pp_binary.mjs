#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, chmodSync } from "node:fs";
import path from "node:path";
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

function extensionForTarget(target) {
  if (!target) {
    return process.platform === "win32" ? ".exe" : "";
  }
  return target.includes("windows") ? ".exe" : "";
}

function ensureSourceExists(filePath) {
  if (!existsSync(filePath)) {
    console.error(`Expected pp binary missing: ${filePath}`);
    process.exit(1);
  }
}

function main() {
  const args = process.argv.slice(2);
  const target = readFlagValue(args, "--target");

  const repoRoot = process.cwd();
  const manifestPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
  const binariesDir = path.join(repoRoot, "src-tauri", "binaries");
  mkdirSync(binariesDir, { recursive: true });

  if (target === "universal-apple-darwin") {
    run("cargo", [
      "build",
      "--manifest-path",
      manifestPath,
      "--bin",
      "pp",
      "--release",
      "--target",
      "aarch64-apple-darwin",
    ]);
    run("cargo", [
      "build",
      "--manifest-path",
      manifestPath,
      "--bin",
      "pp",
      "--release",
      "--target",
      "x86_64-apple-darwin",
    ]);

    const arm = path.join(
      repoRoot,
      "src-tauri",
      "target",
      "aarch64-apple-darwin",
      "release",
      "pp",
    );
    const intel = path.join(
      repoRoot,
      "src-tauri",
      "target",
      "x86_64-apple-darwin",
      "release",
      "pp",
    );
    const universalDir = path.join(
      repoRoot,
      "src-tauri",
      "target",
      "universal-apple-darwin",
      "release",
    );
    const universalOutput = path.join(universalDir, "pp");
    const bundledOutput = path.join(binariesDir, "pp");

    mkdirSync(universalDir, { recursive: true });
    ensureSourceExists(arm);
    ensureSourceExists(intel);
    run("lipo", ["-create", "-output", universalOutput, arm, intel]);
    copyFileSync(universalOutput, bundledOutput);
    chmodSync(universalOutput, 0o755);
    chmodSync(bundledOutput, 0o755);
    console.log(`Prepared universal pp binary: ${universalOutput}`);
    console.log(`Prepared bundled pp binary: ${bundledOutput}`);
    return;
  }

  const cargoArgs = ["build", "--manifest-path", manifestPath, "--bin", "pp", "--release"];
  if (target) {
    cargoArgs.push("--target", target);
  }
  run("cargo", cargoArgs);

  const ext = extensionForTarget(target);
  const source = target
    ? path.join(repoRoot, "src-tauri", "target", target, "release", `pp${ext}`)
    : path.join(repoRoot, "src-tauri", "target", "release", `pp${ext}`);
  const output = path.join(binariesDir, `pp${ext}`);

  ensureSourceExists(source);
  copyFileSync(source, output);
  if (!ext) {
    chmodSync(output, 0o755);
  }

  console.log(`Prepared bundled pp binary: ${output}`);
}

main();
