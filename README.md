# Bellow

A high-performance audio engine for TypeScript, powered by Rust.

**Bellow** is a hybrid audio engine — a TypeScript facade with a real-time Rust DSP core. It targets pro-audio use cases: music playback, DJ mixing, live performance, spatial audio, and mastering-quality processing.

## Features

- **File playback** — Decode and play mp3, wav, flac, ogg, aac, m4a, opus (via Symphonia)
- **Device I/O** — WASAPI shared/exclusive, Core Audio, ALSA, JACK, PipeWire (via CPAL)
- **Mixer & FX** — Channels, buses, sends, parametric EQ, compressor, plate reverb, stereo delay, brickwall limiter
- **Meters** — Peak, RMS, LUFS-I, true-peak, spectrum analyzer
- **Spatial audio** — 3D positional, HRTF binaural, ambisonics, object-based surround (M6)
- **DJ tools** — BPM/key detection, beatgrid, decks, crossfader, harmonic mix advisor (M5)
- **Howler-style API** — `bellow.play(src)`, `sound.fade()`, `sound.on('end')`, sprites
- **Sample-accurate automation** — Parameter ramps and curves on the audio thread

## Architecture

```
TS facade (@sethunthunder111/bellow)
  ├─ Embedded transport (NAPI-RS, in-process)
  └─ Sidecar transport (TCP JSON-RPC, separate process)

Rust core (engine/crates)
  ├─ bellow-core   — Engine, graph, channels, buses
  ├─ bellow-decode — Audio file decoding (Symphonia)
  ├─ bellow-io     — Device I/O (CPAL)
  ├─ bellow-dsp    — FX nodes (EQ, compressor, reverb, delay, limiter)
  ├─ bellow-meter  — Peak, RMS, LUFS, spectrum
  ├─ bellow-resample — Sample-rate conversion (rubato)
  ├─ bellow-ipc    — Framed JSON-RPC transport
  └─ bellow-daemon — Standalone sidecar binary
```

## Quick start

```ts
import bellow from '@sethunthunder111/bellow';

await bellow.init();

const sound = await bellow.play('song.mp3', {
  volume: 0.8,
  loop: true,
});

sound.on('end', () => console.log('Done!'));
await sound.fade(0, 1, 1500);
```

## Milestones

| Milestone | Status | What's in it |
|-----------|--------|-------------|
| M0 | Done | Foundations — engine init, version, dual transport (embedded + sidecar) |
| M1 | In progress | Minimal playback — decode, CPAL output, `play()` handle, master volume, meters |
| M2 | In progress | Mixer & graph — channels, buses, FX scaffold (EQ, comp, reverb, delay, limiter) |
| M3 | In progress | Devices & I/O depth — device enum, exclusive mode, 384 kHz, resampling |
| M4 | In progress | Pro DSP catalogue — linear-phase EQ, vintage comps, convolution reverb, modulation, saturation |
| M5 | Planned | Analysis & DJ mix tools — BPM/key, beatgrid, decks, crossfader |
| M6 | Planned | Spatial & color — 3D positional, HRTF, ambisonics, bellowColor macro |
| M7 | Planned | Virtual cable + advanced routing |
| M8 | Planned | Performance & polish — SIMD, multi-thread graph, latency reporting |
| M9 | Planned | Reach goals — HOA, VST3/CLAP host, GPU convolution, WASM |

## Build

```bash
# Install dependencies
bun install

# Build Rust crates + NAPI addon
bun run build

# Run sidecar transport test
bun test-sidecar.mjs
```

Requires **Rust** (1.78+) and **Bun**.

## License

MIT
