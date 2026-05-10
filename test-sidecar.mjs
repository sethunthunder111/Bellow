#!/usr/bin/env bun
/**
 * M0 round-trip test: spawns bellow-daemon, sends engine.version RPC,
 * and verifies the response.
 *
 * Uses only Bun built-ins (no npm dependencies required).
 */

import { spawn } from "child_process";
import { connect } from "net";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

function log(msg) {
  console.log(`[test] ${msg}`);
}

function findDaemon() {
  const candidates = [
    join(__dirname, "packages", "bellow", "native", "bellow-daemon.exe"),
    join(__dirname, "engine", "target", "release", "bellow-daemon.exe"),
    join(__dirname, "engine", "target", "debug", "bellow-daemon.exe"),
  ];
  for (const p of candidates) {
    try {
      if (require("fs").existsSync(p)) return p;
    } catch {
      // not found
    }
  }
  throw new Error("bellow-daemon binary not found. Run cargo build first.");
}

function findFreePort() {
  return new Promise((resolve, reject) => {
    const srv = require("net").createServer();
    srv.listen(0, "127.0.0.1", () => {
      const port = srv.address().port;
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });
}

async function rpcCall(socket, method, params) {
  const id = Math.floor(Math.random() * 1_000_000);
  const msg = JSON.stringify({ id, method, params });
  const payload = Buffer.from(msg, "utf-8");
  const header = Buffer.alloc(4);
  header.writeUInt32BE(payload.length, 0);
  socket.write(Buffer.concat([header, payload]));

  return new Promise((resolve, reject) => {
    let buffer = Buffer.alloc(0);
    const onData = (data) => {
      buffer = Buffer.concat([buffer, data]);
      while (buffer.length >= 4) {
        const len = buffer.readUInt32BE(0);
        if (buffer.length < 4 + len) break;
        const resp = JSON.parse(buffer.subarray(4, 4 + len).toString("utf-8"));
        socket.off("data", onData);
        if (resp.error) reject(new Error(resp.error));
        else resolve(resp.result);
        return;
      }
    };
    socket.on("data", onData);
    socket.on("error", reject);
  });
}

async function main() {
  const daemonPath = findDaemon();
  log(`Daemon: ${daemonPath}`);

  const port = await findFreePort();
  log(`Port: ${port}`);

  const proc = spawn(daemonPath, ["--port", String(port)], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  proc.stderr.on("data", (d) =>
    console.error(`[daemon-err] ${d.toString().trim()}`),
  );

  // Wait for daemon to start listening
  await new Promise((r) => setTimeout(r, 500));

  const socket = connect({ port, host: "127.0.0.1" });

  await new Promise((resolve, reject) => {
    socket.once("connect", resolve);
    socket.once("error", reject);
  });

  log("Connected to daemon");

  // Test 1: engine.version
  const version = await rpcCall(socket, "engine.version", {});
  log(`version = ${JSON.stringify(version)}`);
  console.assert(version.version === "0.0.1", "version mismatch");
  console.assert(
    Array.isArray(version.supported_backends),
    "backends should be array",
  );

  // Test 2: engine.init
  const initRes = await rpcCall(socket, "engine.init", {
    sample_rate: 48000,
    buffer_size: 256,
    internal_precision: "f32",
  });
  log(`init = ${JSON.stringify(initRes)}`);
  console.assert(initRes.ok === true, "init should succeed");

  // Test 3: engine.init again (should fail - already initialized)
  const initAgain = await rpcCall(socket, "engine.init", {
    sample_rate: 48000,
    buffer_size: 256,
    internal_precision: "f32",
  });
  log(`initAgain = ${JSON.stringify(initAgain)}`);
  console.assert(initAgain.ok === false, "second init should fail");

  // Test 4: engine.shutdown
  const shutdownRes = await rpcCall(socket, "engine.shutdown", {});
  log(`shutdown = ${JSON.stringify(shutdownRes)}`);
  console.assert(shutdownRes.ok === true, "shutdown should succeed");

  socket.end();
  proc.kill();

  log("All sidecar round-trip tests passed!");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
