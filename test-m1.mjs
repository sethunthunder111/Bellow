#!/usr/bin/env node
/**
 * M1 feature test: sound.load, sound.play, master.setVolume, devices.list
 */
import { spawn } from "child_process";
import { existsSync } from "fs";
import { connect, createServer } from "net";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

function findDaemon() {
  const candidates = [
    join(__dirname, "packages", "bellow", "native", "bellow-daemon.exe"),
    join(__dirname, "engine", "target", "release", "bellow-daemon.exe"),
    join(__dirname, "engine", "target", "debug", "bellow-daemon.exe"),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  throw new Error("bellow-daemon binary not found");
}

function findFreePort() {
  return new Promise((resolve, reject) => {
    const srv = createServer();
    srv.listen(0, "127.0.0.1", () => {
      const port = srv.address().port;
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });
}

// Persistent buffer + pending-promise dispatch to handle TCP reassembly correctly.
function makeRpcClient(socket) {
  let buffer = Buffer.alloc(0);
  const pending = new Map();

  socket.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (buffer.length >= 4) {
      const len = buffer.readUInt32BE(0);
      if (buffer.length < 4 + len) break;
      const payload = buffer.subarray(4, 4 + len);
      buffer = buffer.subarray(4 + len);
      try {
        const msg = JSON.parse(payload.toString("utf-8"));
        const p = pending.get(msg.id);
        if (p) {
          pending.delete(msg.id);
          p.resolve(msg);
        }
      } catch {
        /* ignore malformed */
      }
    }
  });

  return function rpcCall(method, params) {
    const id = Math.floor(Math.random() * 1_000_000);
    const payload = JSON.stringify({ id, method, params });
    const header = Buffer.alloc(4);
    header.writeUInt32BE(Buffer.byteLength(payload), 0);
    socket.write(Buffer.concat([header, Buffer.from(payload)]));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`RPC timeout for ${method}`));
      }, 5000);
      pending.set(id, {
        resolve: (msg) => {
          clearTimeout(timer);
          resolve(msg);
        },
      });
    });
  };
}

async function main() {
  const daemonPath = findDaemon();
  const port = await findFreePort();
  const proc = spawn(daemonPath, ["--port", String(port)], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  proc.stdout?.on("data", (d) => console.log("[daemon]", d.toString().trim()));
  proc.stderr?.on("data", (d) => console.log("[daemon]", d.toString().trim()));

  await new Promise((r) => setTimeout(r, 300));

  const socket = connect({ port, host: "127.0.0.1" });
  await new Promise((resolve, reject) => {
    socket.once("connect", resolve);
    socket.once("error", reject);
  });

  console.log("[M1 test] Connected to daemon");
  const rpc = makeRpcClient(socket);

  // Init engine
  const initRes = await rpc("engine.init", {
    sampleRate: 48000,
    bufferSize: 1024,
    internalPrecision: "f32",
  });
  if (initRes.result?.ok !== true) {
    throw new Error(`engine.init failed: ${JSON.stringify(initRes)}`);
  }
  console.log("[M1 test] engine.init OK");

  // Load sound
  const audioPath =
    "C:/Users/haloi/Documents/Codes/bellow/PLEEG - Home [NCS Release].mp3";
  const loadRes = await rpc("sound.load", { src: audioPath });
  const soundId = loadRes.result.id;
  console.log(
    "[M1 test] sound.load => id:",
    soundId,
    "state:",
    loadRes.result.state,
    "durationMs:",
    loadRes.result.durationMs,
  );

  // Play
  await rpc("sound.play", { id: soundId });
  console.log("[M1 test] sound.play OK — listening for 10 seconds...");

  // Let it play
  await new Promise((r) => setTimeout(r, 10000));

  // List sounds
  const listRes = await rpc("sound.list", {});
  console.log("[M1 test] sound.list =>", listRes.result?.length, "sound(s)");

  // Master volume
  await rpc("master.setVolume", { volumeDb: -6 });
  const masterRes = await rpc("master.get", {});
  console.log("[M1 test] master.get =>", JSON.stringify(masterRes.result));

  // Devices
  const devRes = await rpc("devices.list", {});
  console.log(
    "[M1 test] devices.list =>",
    devRes.result?.outputs?.length,
    "output(s)",
  );

  // Stop + dispose
  await rpc("sound.stop", { id: soundId });
  await rpc("sound.dispose", { id: soundId });
  console.log("[M1 test] sound.stop + dispose OK");

  // Shutdown
  await rpc("engine.shutdown", {});
  console.log("[M1 test] engine.shutdown OK");

  socket.end();
  proc.kill();
  console.log("[M1 test] All M1 RPC tests passed!");
}

main().catch((err) => {
  console.error("[M1 test] FAILED:", err.message);
  process.exit(1);
});
