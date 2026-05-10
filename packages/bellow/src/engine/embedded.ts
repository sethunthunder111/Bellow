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

  // ---- Sound (M1) ----
  async soundLoad(src: string): Promise<SoundHandle> {
    const native = loadNative();
    return native.sound_load({ src }) as SoundHandle;
  }
  async soundPlay(id: string): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_play({ id }) as { ok: boolean; error?: string };
  }
  async soundPause(id: string): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_pause({ id }) as { ok: boolean; error?: string };
  }
  async soundStop(id: string): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_stop({ id }) as { ok: boolean; error?: string };
  }
  async soundSeek(id: string, positionMs: number): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_seek({ id, positionMs }) as { ok: boolean; error?: string };
  }
  async soundSetVolume(id: string, volume: number): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_setVolume({ id, volume }) as { ok: boolean; error?: string };
  }
  async soundSetRate(id: string, rate: number): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_setRate({ id, rate }) as { ok: boolean; error?: string };
  }
  async soundSetLoop(id: string, loop: boolean): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_setLoop({ id, loop }) as { ok: boolean; error?: string };
  }
  async soundDispose(id: string): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.sound_dispose({ id }) as { ok: boolean; error?: string };
  }
  async soundList(): Promise<SoundHandle[]> {
    const native = loadNative();
    return native.sound_list({}) as SoundHandle[];
  }

  // ---- Master ----
  async masterSetVolume(volumeDb: number): Promise<{ ok: boolean; error?: string }> {
    const native = loadNative();
    return native.master_setVolume({ volumeDb }) as { ok: boolean; error?: string };
  }
  async masterGet(): Promise<MasterState> {
    const native = loadNative();
    return native.master_get({}) as MasterState;
  }

  // ---- Devices ----
  async devicesList(): Promise<DeviceList> {
    const native = loadNative();
    return native.devices_list({}) as DeviceList;
  }
}
