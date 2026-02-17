import fs from "node:fs";
import path from "node:path";

function die(msg) {
  console.error(msg);
  process.exit(1);
}

const expected = process.argv[2]; // optional
const root = process.cwd();

const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const conf = JSON.parse(
  fs.readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const cargo = fs.readFileSync(path.join(root, "src-tauri", "Cargo.toml"), "utf8");

const cargoMatch = cargo.match(/(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/m);
if (!cargoMatch) die("Failed to read version from src-tauri/Cargo.toml");

const versions = {
  "package.json": pkg.version,
  "src-tauri/tauri.conf.json": conf.version,
  "src-tauri/Cargo.toml": cargoMatch[2],
};

const unique = new Set(Object.values(versions));
if (unique.size !== 1) {
  console.error("Version mismatch detected:");
  for (const [k, v] of Object.entries(versions)) console.error(`- ${k}: ${v}`);
  process.exit(1);
}

const v = [...unique][0];
if (expected && v !== expected) {
  console.error(`Version mismatch vs expected: repo=${v}, expected=${expected}`);
  process.exit(1);
}

console.log(`OK: version=${v}`);

