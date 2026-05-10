import type {
  EngineConfig, SoundHandle, MasterState, DeviceList,
} from '@sethunthunder111/bellow-contract';
import { BellowEngine } from './engine/client.js';
import { BellowSound } from './sound.js';

let globalEngine: BellowEngine | null = null;

export interface BellowInitOptions extends EngineConfig {}

export async function init(options?: BellowInitOptions): Promise<void> {
  if (globalEngine?.isReady()) {
    return;
  }
  globalEngine = new BellowEngine(options ?? {});
  await globalEngine.init();
}

export async function version(): Promise<{
  version: string;
  rustVersion: string;
  supportedBackends: string[];
}> {
  if (!globalEngine) {
    throw new Error('Bellow not initialized. Call bellow.init() first.');
  }
  return globalEngine.version();
}

export async function shutdown(): Promise<void> {
  if (globalEngine) {
    await globalEngine.shutdown();
    globalEngine = null;
  }
}

export async function suspend(): Promise<void> {
  if (!globalEngine) throw new Error('Bellow not initialized.');
  await globalEngine.suspend();
}

export async function resume(): Promise<void> {
  if (!globalEngine) throw new Error('Bellow not initialized.');
  await globalEngine.resume();
}

// ---- M1: Playback ----

export async function play(src: string, options?: {
  volume?: number;
  loop?: boolean;
  rate?: number;
}): Promise<BellowSound> {
  if (!globalEngine) throw new Error('Bellow not initialized. Call bellow.init() first.');
  const handle = await globalEngine.soundLoad(src);
  if (options?.volume !== undefined) {
    await globalEngine.soundSetVolume(handle.id, options.volume);
  }
  if (options?.loop !== undefined) {
    await globalEngine.soundSetLoop(handle.id, options.loop);
  }
  if (options?.rate !== undefined) {
    await globalEngine.soundSetRate(handle.id, options.rate);
  }
  await globalEngine.soundPlay(handle.id);
  return new BellowSound(handle, globalEngine);
}

export async function masterVolume(volumeDb: number): Promise<void> {
  if (!globalEngine) throw new Error('Bellow not initialized.');
  await globalEngine.masterSetVolume(volumeDb);
}

export async function masterState(): Promise<MasterState> {
  if (!globalEngine) throw new Error('Bellow not initialized.');
  return globalEngine.masterGet();
}

// ---- M3: Devices ----

export async function devices(): Promise<DeviceList> {
  if (!globalEngine) throw new Error('Bellow not initialized.');
  return globalEngine.devicesList();
}

// Re-export contract types for advanced users
export type { EngineConfig, SoundHandle, MasterState, DeviceList } from '@sethunthunder111/bellow-contract';
export { BellowSound } from './sound.js';

// Default export for convenience
export default {
  init,
  version,
  shutdown,
  suspend,
  resume,
  play,
  masterVolume,
  masterState,
  devices,
};
