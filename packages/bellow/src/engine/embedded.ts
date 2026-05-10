import type {
  EngineConfig, EngineVersion,
  SoundHandle, MasterState, DeviceList,
} from '@sethunthunder111/bellow-contract';
import type { Transport } from './client.js';

// In-process NAPI transport: calls Rust functions directly via the native addon.
// The addon is loaded lazily so the package works even before postinstall completes.

let nativeModule: Record<string, (...args: unknown[]) => unknown> | null = null;

function loadNative(): Record<string, (...args: unknown[]) => unknown> {
  if (nativeModule) return nativeModule;
  try {
    // napi-rs produces a .node file whose name includes the platform triple.
    // We'll search for it in the native/ directory.
    const { existsSync, readdirSync } = require('fs');
    const { join } = require('path');
    const nativeDir = join(__dirname, '..', '..', 'native');
    if (!existsSync(nativeDir)) {
      throw new Error('Native module directory not found. Run postinstall build.');
    }
    const files = readdirSync(nativeDir);
    const nodeFile = files.find((f: string) => f.endsWith('.node'));
    if (!nodeFile) {
      throw new Error('No .node native addon found in native/. Run postinstall build.');
    }
    const mod = require(join(nativeDir, nodeFile));
    nativeModule = mod;
    return mod;
  } catch (err) {
    throw new Error(`Failed to load native addon: ${(err as Error).message}`);
  }
}

export class EmbeddedTransport implements Transport {
  async init(config: EngineConfig): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    const res = native.engine_init(config);
    return res as { ok: boolean; error?: string };
  }

  async shutdown(): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.engine_shutdown() as { ok: boolean; error?: string };
  }

  async suspend(): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.engine_suspend() as { ok: boolean; error?: string };
  }

  async resume(): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.engine_resume() as { ok: boolean; error?: string };
  }

  async version(): Promise<EngineVersion> {
    const native = loadNative();
    return native.engine_version() as EngineVersion;
  }

  async dispose(): Promise<void> {
    // nothing extra for embedded; shutdown handles it
  }
}
