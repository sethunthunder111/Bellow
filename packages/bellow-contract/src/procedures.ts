import { oc } from '@orpc/contract';
import { z } from 'zod';
import {
  EngineConfigSchema, EngineVersionSchema, ResultSchema,
  SoundHandleSchema, MasterStateSchema, DeviceListSchema,
} from './types.js';

export const EngineProcedures = oc.router({
  init: oc
    .input(EngineConfigSchema)
    .output(ResultSchema)
    .meta({ description: 'Initialize the audio engine with the given configuration' }),

  shutdown: oc
    .input(z.object({}).optional())
    .output(ResultSchema)
    .meta({ description: 'Shut down the audio engine' }),

  suspend: oc
    .input(z.object({}).optional())
    .output(ResultSchema)
    .meta({ description: 'Suspend audio processing' }),

  resume: oc
    .input(z.object({}).optional())
    .output(ResultSchema)
    .meta({ description: 'Resume audio processing' }),

  version: oc
    .input(z.object({}).optional())
    .output(EngineVersionSchema)
    .meta({ description: 'Get engine version and capability info' }),
});

export const SoundProcedures = oc.router({
  load: oc
    .input(z.object({ src: z.string() }))
    .output(SoundHandleSchema)
    .meta({ description: 'Load an audio file and return a sound handle' }),

  play: oc
    .input(z.object({ id: z.string() }))
    .output(ResultSchema)
    .meta({ description: 'Start or resume playback' }),

  pause: oc
    .input(z.object({ id: z.string() }))
    .output(ResultSchema)
    .meta({ description: 'Pause playback' }),

  stop: oc
    .input(z.object({ id: z.string() }))
    .output(ResultSchema)
    .meta({ description: 'Stop playback and reset position' }),

  seek: oc
    .input(z.object({ id: z.string(), positionMs: z.number().int() }))
    .output(ResultSchema)
    .meta({ description: 'Seek to a position in milliseconds' }),

  setVolume: oc
    .input(z.object({ id: z.string(), volume: z.number().min(0).max(1) }))
    .output(ResultSchema)
    .meta({ description: 'Set sound volume (0–1)' }),

  setRate: oc
    .input(z.object({ id: z.string(), rate: z.number().min(0.01) }))
    .output(ResultSchema)
    .meta({ description: 'Set playback rate' }),

  setLoop: oc
    .input(z.object({ id: z.string(), loop: z.boolean() }))
    .output(ResultSchema)
    .meta({ description: 'Enable/disable loop' }),

  dispose: oc
    .input(z.object({ id: z.string() }))
    .output(ResultSchema)
    .meta({ description: 'Dispose sound and free memory' }),

  list: oc
    .input(z.object({}).optional())
    .output(z.array(SoundHandleSchema))
    .meta({ description: 'List all active sounds' }),
});

export const MasterProcedures = oc.router({
  setVolume: oc
    .input(z.object({ volumeDb: z.number() }))
    .output(ResultSchema)
    .meta({ description: 'Set master volume in dB' }),

  get: oc
    .input(z.object({}).optional())
    .output(MasterStateSchema)
    .meta({ description: 'Get current master state' }),
});

export const DeviceProcedures = oc.router({
  list: oc
    .input(z.object({ refresh: z.boolean().optional() }).optional())
    .output(DeviceListSchema)
    .meta({ description: 'List available audio devices' }),
});

export const BellowContract = oc.router({
  engine: EngineProcedures,
  sound: SoundProcedures,
  master: MasterProcedures,
  devices: DeviceProcedures,
});

export type BellowContract = typeof BellowContract;
