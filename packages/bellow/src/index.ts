import type { EngineConfig } from '@sethunthunder111/bellow-contract';
import { BellowEngine } from './engine/client.js';

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

// Re-export contract types for advanced users
export type { EngineConfig } from '@sethunthunder111/bellow-contract';

// Default export for convenience
export default {
  init,
  version,
  shutdown,
  suspend,
  resume,
};
