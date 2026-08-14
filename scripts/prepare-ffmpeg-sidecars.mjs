import { chmodSync, copyFileSync, existsSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";

const require = createRequire(import.meta.url);
const checkOnly = process.argv.includes("--check");
const target = currentTarget();
const binaries = [
  { name: "ffmpeg", source: require("ffmpeg-static") },
  { name: "ffprobe", source: require("ffprobe-static").path },
];

for (const binary of binaries) {
  if (typeof binary.source !== "string" || !existsSync(binary.source)) {
    throw new Error(`${binary.name} npm artifact is unavailable; run npm install`);
  }
  const destination = resolve(
    dirname(new URL(import.meta.url).pathname),
    "../src-tauri/binaries",
    `${binary.name}-${target}`,
  );
  if (!checkOnly) {
    copyFileSync(binary.source, destination);
    chmodSync(destination, 0o755);
  }
  if (!existsSync(destination) || !statSync(destination).isFile()) {
    throw new Error(`missing generated sidecar: ${destination}`);
  }
  const version = execFileSync(destination, ["-version"], { encoding: "utf8" })
    .split("\n", 1)[0];
  console.log(`${binary.name}-${target}: ${version}`);
}

function currentTarget() {
  const expected = process.env.ENCORE_SIDECAR_TARGET;
  const detected = targetFor(process.platform, process.arch);
  if (expected && expected !== detected) {
    throw new Error(
      `cross-target sidecars are not available from host npm artifacts (${detected} -> ${expected})`,
    );
  }
  return expected ?? detected;
}

function targetFor(platform, arch) {
  if (platform !== "darwin") {
    throw new Error(`Encore sidecars currently support macOS hosts, received ${platform}`);
  }
  if (arch === "arm64") return "aarch64-apple-darwin";
  if (arch === "x64") return "x86_64-apple-darwin";
  throw new Error(`unsupported macOS architecture for sidecars: ${arch}`);
}
