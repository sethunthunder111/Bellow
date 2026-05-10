import type { EngineConfig, EngineVersion } from '@sethunthunder111/bellow-contract';
import type { Transport } from './client.js';
import { spawn } from 'child_process';
import { join } from 'path';
import { createConnection, type Socket } from 'net';

interface RpcMessage {
  id: number;
  method: string;
  params: unknown;
}

interface RpcResponse {
  id: number;
  result?: unknown;
  error?: string;
}

export class SidecarTransport implements Transport {
  private socket: Socket | null = null;
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private buffer = Buffer.alloc(0);

  private async ensureConnected(): Promise<Socket> {
    if (this.socket) return this.socket;

    // Try to find and spawn the daemon binary
    const daemonPath = this.findDaemonPath();
    if (!daemonPath) {
      throw new Error('bellow-daemon binary not found. Run postinstall build.');
    }

    // Spawn daemon with IPC port argument
    const port = await this.findFreePort();
    const proc = spawn(daemonPath, ['--port', String(port)], {
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    proc.stderr?.on('data', (d) => console.error('[bellow-daemon]', d.toString()));

    // Wait a moment for daemon to start listening
    await new Promise((r) => setTimeout(r, 200));

    const socket = createConnection({ port, host: '127.0.0.1' });
    this.socket = socket;

    socket.on('data', (data) => this.onData(data));
    socket.on('error', (err) => this.rejectAll(err));
    socket.on('close', () => {
      this.socket = null;
      this.rejectAll(new Error('Sidecar connection closed'));
    });

    await new Promise<void>((resolve, reject) => {
      socket.once('connect', () => resolve());
      socket.once('error', reject);
    });

    return socket;
  }

  private findDaemonPath(): string | null {
    try {
      const { existsSync } = require('fs');
      const { join } = require('path');
      const candidates = [
        join(__dirname, '..', '..', 'native', 'bellow-daemon'),
        join(__dirname, '..', '..', 'native', 'bellow-daemon.exe'),
        join(process.cwd(), 'engine', 'target', 'release', 'bellow-daemon'),
        join(process.cwd(), 'engine', 'target', 'release', 'bellow-daemon.exe'),
      ];
      for (const p of candidates) {
        if (existsSync(p)) return p;
      }
    } catch {
      // ignore
    }
    return null;
  }

  private async findFreePort(): Promise<number> {
    return new Promise((resolve, reject) => {
      const srv = require('net').createServer();
      srv.listen(0, '127.0.0.1', () => {
        const port = srv.address()?.port;
        srv.close(() => resolve(port));
      });
      srv.on('error', reject);
    });
  }

  private onData(data: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, data]);
    while (true) {
      if (this.buffer.length < 4) break;
      const len = this.buffer.readUInt32BE(0);
      if (this.buffer.length < 4 + len) break;
      const payload = this.buffer.subarray(4, 4 + len);
      this.buffer = this.buffer.subarray(4 + len);
      try {
        const msg = JSON.parse(payload.toString('utf-8')) as RpcResponse;
        const pending = this.pending.get(msg.id);
        if (pending) {
          this.pending.delete(msg.id);
          if (msg.error) pending.reject(new Error(msg.error));
          else pending.resolve(msg.result);
        }
      } catch {
        // ignore malformed
      }
    }
  }

  private rejectAll(err: Error): void {
    for (const [, p] of this.pending) p.reject(err);
    this.pending.clear();
  }

  private async call(method: string, params: unknown): Promise<unknown> {
    const socket = await this.ensureConnected();
    const id = this.nextId++;
    const msg: RpcMessage = { id, method, params };
    const payload = Buffer.from(JSON.stringify(msg), 'utf-8');
    const header = Buffer.alloc(4);
    header.writeUInt32BE(payload.length, 0);
    socket.write(Buffer.concat([header, payload]));

    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }

  async init(config: EngineConfig): Promise<{ ok: boolean; error?: string }> {
    return this.call('engine.init', config) as Promise<{ ok: boolean; error?: string }>;
  }

  async shutdown(): Promise<{ ok: boolean; error?: string }> {
    return this.call('engine.shutdown', {}) as Promise<{ ok: boolean; error?: string }>;
  }

  async suspend(): Promise<{ ok: boolean; error?: string }> {
    return this.call('engine.suspend', {}) as Promise<{ ok: boolean; error?: string }>;
  }

  async resume(): Promise<{ ok: boolean; error?: string }> {
    return this.call('engine.resume', {}) as Promise<{ ok: boolean; error?: string }>;
  }

  async version(): Promise<EngineVersion> {
    return this.call('engine.version', {}) as Promise<EngineVersion>;
  }

  async dispose(): Promise<void> {
    if (this.socket) {
      this.socket.end();
      this.socket = null;
    }
    this.rejectAll(new Error('Transport disposed'));
  }
}
