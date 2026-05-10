#!/usr/bin/env node
/**
 * Postinstall build script for @sethunthunder111/bellow
 *
 * Compiles the Rust NAPI addon and the bellow-daemon binary.
 * Detects the host platform and available toolchains.
 * Caches outputs under native/ for reuse.
 */

import { spawn } from "child_process";
import { existsSync, mkdirSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "..", "..");
const nativeDir = join(rootDir, "native");

function log(msg) {
  console.log(`[bellow:postinstall] ${msg}`);
}

function exec(cmd, args, opts) {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      stdio: "inherit",
      ...opts,
    });
    child.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${cmd} ${args.join(" ")} exited with ${code}`));
    });
    child.on("error", reject);
  });
}

async function ensureRust() {
  try {
    await exec("cargo", ["--version"], { stdio: "pipe" });
  } catch {
    log("ERROR: Rust / Cargo not found.");
    log("Please install Rust: https://rustup.rs");
    log("After installation, re-run: npm install");
    process.exit(1);
  }
}

async function ensureNapiCli() {
  try {
    await exec("npx", ["@napi-rs/cli", "--version"], { stdio: "pipe" });
  } catch {
    log("WARNING: @napi-rs/cli not found.");
    log("Attempting to install with bun...");
    try {
      await exec("bun", ["install", "@napi-rs/cli", "--save-dev"], {
        cwd: rootDir,
      });
    } catch {
      log("WARNING: Could not install @napi-rs/cli. Skipping NAPI build.");
      return false;
    }
  }
  return true;
}

async function buildNapi() {
  const engineDir = join(rootDir, "..", "..", "engine");
  if (!existsSync(engineDir)) {
    log(
      "Engine directory not found. Skipping native build (development mode).",
    );
    return;
  }

  log("Building NAPI addon...");
  await exec("npx", ["@napi-rs/cli", "build", "--release", "--platform"], {
    cwd: engineDir,
    env: { ...process.env, NAPI_RS_NATIVE_DIR: nativeDir },
  });
  log("NAPI addon built.");
}

async function buildDaemon() {
  const engineDir = join(rootDir, "..", "..", "engine");
  if (!existsSync(engineDir)) {
    log(
      "Engine directory not found. Skipping daemon build (development mode).",
    );
    return;
  }

  log("Building bellow-daemon...");
  await exec("cargo", ["build", "--release", "-p", "bellow-daemon"], {
    cwd: engineDir,
  });

  const { platform } = process;
  const ext = platform === "win32" ? ".exe" : "";
  const src = join(engineDir, "target", "release", `bellow-daemon${ext}`);
  const dst = join(nativeDir, `bellow-daemon${ext}`);

  if (existsSync(src)) {
    const { copyFileSync } = await import("fs");
    copyFileSync(src, dst);
    log(`Daemon copied to ${dst}`);
  } else {
    log("WARNING: Daemon binary not found after build.");
  }
}

async function main() {
  log("Starting native build...");
  mkdirSync(nativeDir, { recursive: true });

  try {
    await ensureRust();
  } catch {
    log("Rust not found. Skipping native build.");
    return;
  }

  const hasNapiCli = await ensureNapiCli();
  if (hasNapiCli) {
    try {
      await buildNapi();
    } catch (e) {
      log(`NAPI build failed: ${e.message}`);
    }
  }

  try {
    await buildDaemon();
  } catch (e) {
    log(`Daemon build failed: ${e.message}`);
  }

  log("Native build complete.");
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
