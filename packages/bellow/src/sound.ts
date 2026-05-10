import type { SoundHandle } from '@sethunthunder111/bellow-contract';
import type { BellowEngine } from './engine/client.js';

type SoundEvent = 'load' | 'play' | 'pause' | 'stop' | 'end' | 'loaderror';

export class BellowSound {
  private handle: SoundHandle;
  private engine: BellowEngine;
  private listeners: Map<SoundEvent, Set<() => void>> = new Map();

  constructor(handle: SoundHandle, engine: BellowEngine) {
    this.handle = handle;
    this.engine = engine;
  }

  get id(): string { return this.handle.id; }
  get src(): string { return this.handle.src; }
  get state(): string { return this.handle.state; }
  get positionMs(): number { return this.handle.positionMs; }
  get durationMs(): number { return this.handle.durationMs; }
  get volume(): number { return this.handle.volume; }
  get rate(): number { return this.handle.rate; }
  get loop(): boolean { return this.handle.loopPlayback; }

  async play(): Promise<void> {
    await this.engine.soundPlay(this.handle.id);
    this.emit('play');
  }

  async pause(): Promise<void> {
    await this.engine.soundPause(this.handle.id);
    this.emit('pause');
  }

  async stop(): Promise<void> {
    await this.engine.soundStop(this.handle.id);
    this.emit('stop');
  }

  async seek(positionMs: number): Promise<void> {
    await this.engine.soundSeek(this.handle.id, positionMs);
  }

  async fade(from: number, to: number, durationMs: number): Promise<void> {
    const steps = 30;
    const interval = durationMs / steps;
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const vol = from + (to - from) * t;
      await this.engine.soundSetVolume(this.handle.id, vol);
      await new Promise(r => setTimeout(r, interval));
    }
  }

  async setVolume(volume: number): Promise<void> {
    await this.engine.soundSetVolume(this.handle.id, volume);
  }

  async setRate(rate: number): Promise<void> {
    await this.engine.soundSetRate(this.handle.id, rate);
  }

  async setLoop(loop: boolean): Promise<void> {
    await this.engine.soundSetLoop(this.handle.id, loop);
  }

  async dispose(): Promise<void> {
    await this.engine.soundDispose(this.handle.id);
  }

  on(event: SoundEvent, cb: () => void): () => void {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(cb);
    return () => set!.delete(cb);
  }

  off(event: SoundEvent, cb: () => void): void {
    this.listeners.get(event)?.delete(cb);
  }

  private emit(event: SoundEvent): void {
    this.listeners.get(event)?.forEach(cb => {
      try { cb(); } catch { /* ignore */ }
    });
  }
}
