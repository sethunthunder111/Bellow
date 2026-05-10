import type {
  EngineConfig, EngineVersion,
  SoundHandle, MasterState, DeviceList,
} from '@sethunthunder111/bellow-contract';
import { EmbeddedTransport } from './embedded.js';
import { SidecarTransport } from './sidecar.js';

export interface Transport {
  init(config: EngineConfig): Promise<{ ok: boolean; error?: string }>;
  shutdown(): Promise<{ ok: boolean; error?: string }>;
  suspend(): Promise<{ ok: boolean; error?: string }>;
  resume(): Promise<{ ok: boolean; error?: string }>;
  version(): Promise<EngineVersion>;
  dispose(): Promise<void>;
  // Sound
  soundLoad(src: string): Promise<SoundHandle>;
  soundPlay(id: string): Promise<{ ok: boolean; error?: string }>;
  soundPause(id: string): Promise<{ ok: boolean; error?: string }>;
  soundStop(id: string): Promise<{ ok: boolean; error?: string }>;
  soundSeek(id: string, positionMs: number): Promise<{ ok: boolean; error?: string }>;
  soundSetVolume(id: string, volume: number): Promise<{ ok: boolean; error?: string }>;
  soundSetRate(id: string, rate: number): Promise<{ ok: boolean; error?: string }>;
  soundSetLoop(id: string, loop: boolean): Promise<{ ok: boolean; error?: string }>;
  soundDispose(id: string): Promise<{ ok: boolean; error?: string }>;
  soundList(): Promise<SoundHandle[]>;
  // Master
  masterSetVolume(volumeDb: number): Promise<{ ok: boolean; error?: string }>;
  masterGet(): Promise<MasterState>;
  // Devices
  devicesList(): Promise<DeviceList>;
}

const DEFAULT_CONFIG: EngineConfig = {
  sampleRate: 48000,
  bufferSize: 256,
  internalPrecision: 'f32',
  transport: 'embedded',
  device: {
    output: 'default',
    input: undefined,
    exclusive: false,
  },
};

export class BellowEngine {
  private transport: Transport;
  private ready = false;
  private config: EngineConfig;

  constructor(config: Partial<EngineConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    const mode = this.config.transport ?? 'embedded';
    if (mode === 'sidecar') {
      this.transport = new SidecarTransport();
    } else if (mode === 'embedded') {
      this.transport = new EmbeddedTransport();
    } else {
      // auto: try embedded first, fall back to sidecar
      try {
        this.transport = new EmbeddedTransport();
      } catch {
        this.transport = new SidecarTransport();
      }
    }
  }

  isReady(): boolean {
    return this.ready;
  }

  async init(): Promise<void> {
    const res = await this.transport.init(this.config);
    if (!res.ok) {
      throw new Error(`Engine init failed: ${res.error ?? 'unknown'}`);
    }
    this.ready = true;
  }

  async shutdown(): Promise<void> {
    if (!this.ready) return;
    await this.transport.shutdown();
    this.ready = false;
  }

  async suspend(): Promise<void> {
    await this.transport.suspend();
  }

  async resume(): Promise<void> {
    await this.transport.resume();
  }

  async version(): Promise<EngineVersion> {
    return this.transport.version();
  }

  async dispose(): Promise<void> {
    await this.transport.dispose();
    this.ready = false;
  }
}
