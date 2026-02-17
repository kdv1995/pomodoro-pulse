import fs from "node:fs";
import path from "node:path";

function die(msg) {
  console.error(msg);
  process.exit(1);
}

const version = process.argv[2];
if (!version) {
  die('Usage: node scripts/set_version.mjs <version>  (example: 0.2.0 or 0.2.0-beta.1)');
}

// Accept semver-ish versions. We keep this permissive (supports prerelease/build metadata).
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  die(`Invalid version: "${version}" (expected semver like 0.2.0 or 0.2.0-beta.1)`);
}

const root = process.cwd();

// 1) package.json
{
  const p = path.join(root, "package.json");
  const pkg = JSON.parse(fs.readFileSync(p, "utf8"));
  pkg.version = version;
  fs.writeFileSync(p, JSON.stringify(pkg, null, 2) + "\n");
  console.log(`Updated package.json -> ${version}`);
}

// 2) src-tauri/tauri.conf.json
{
  const p = path.join(root, "src-tauri", "tauri.conf.json");
  const conf = JSON.parse(fs.readFileSync(p, "utf8"));
  conf.version = version;
  fs.writeFileSync(p, JSON.stringify(conf, null, 2) + "\n");
  console.log(`Updated src-tauri/tauri.conf.json -> ${version}`);
}

// 3) src-tauri/Cargo.toml ([package] version)
{
  const p = path.join(root, "src-tauri", "Cargo.toml");
  const s = fs.readFileSync(p, "utf8");

  const re = /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/m;
  const m = s.match(re);
  if (!m) die("Failed to find [package] version in src-tauri/Cargo.toml");

  const out = s.replace(re, `$1${version}$3`);
  fs.writeFileSync(p, out);
  console.log(`Updated src-tauri/Cargo.toml -> ${version}`);
}

