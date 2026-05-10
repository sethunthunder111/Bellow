import { z } from "zod";

// Engine configuration
export const EngineConfigSchema = z.object({
  sampleRate: z.number().int().min(22050).max(384000).optional().default(48000),
  bufferSize: z.number().int().min(16).max(4096).optional().default(256),
  internalPrecision: z.enum(["f32", "f64"]).optional().default("f32"),
  transport: z
    .enum(["embedded", "sidecar", "auto"])
    .optional()
    .default("embedded"),
  device: z
    .object({
      output: z.string().optional().default("default"),
      input: z.string().optional(),
      exclusive: z.boolean().optional().default(false),
    })
    .optional(),
});

export type EngineConfig = z.infer<typeof EngineConfigSchema>;

// Engine info / version
export const EngineVersionSchema = z.object({
  version: z.string(),
  rustVersion: z.string(),
  supportedBackends: z.array(z.string()),
});

export type EngineVersion = z.infer<typeof EngineVersionSchema>;

// Shared response wrapper
export const OkSchema = z.object({ ok: z.literal(true) });
export const ErrSchema = z.object({ ok: z.literal(false), error: z.string() });
export const ResultSchema = z.union([OkSchema, ErrSchema]);

export type Ok = z.infer<typeof OkSchema>;
export type Err = z.infer<typeof ErrSchema>;
export type Result = z.infer<typeof ResultSchema>;

// ---- Sound (M1) ----
export const SoundHandleSchema = z.object({
  id: z.string(),
  src: z.string(),
  state: z.enum(["loading", "playing", "paused", "stopped", "ended"]),
  positionMs: z.number().int().default(0),
  durationMs: z.number().int().default(0),
  volume: z.number().min(0).max(1).default(1),
  rate: z.number().min(0.01).default(1),
  loopPlayback: z.boolean().default(false),
});

export type SoundHandle = z.infer<typeof SoundHandleSchema>;

// ---- Master ----
export const MasterStateSchema = z.object({
  volumeDb: z.number().default(0),
  muted: z.boolean().default(false),
});

export type MasterState = z.infer<typeof MasterStateSchema>;

// ---- Devices ----
export const BufferSizeRangeSchema = z.object({
  min: z.number().int(),
  max: z.number().int(),
  preferred: z.array(z.number().int()),
});

export const DeviceInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  hostApi: z.string(),
  isInput: z.boolean(),
  isOutput: z.boolean(),
  channelCount: z.number().int(),
  channelLayout: z.array(z.string()),
  supportedSampleRates: z.array(z.number().int()),
  supportedBufferSizes: BufferSizeRangeSchema,
  supportedFormats: z.array(z.string()),
  isDefault: z.boolean(),
});

export const DeviceListSchema = z.object({
  inputs: z.array(DeviceInfoSchema),
  outputs: z.array(DeviceInfoSchema),
  hostApis: z.array(z.string()),
});

export type DeviceList = z.infer<typeof DeviceListSchema>;
export type DeviceInfo = z.infer<typeof DeviceInfoSchema>;

// ---- FX (M2) ----
export const EqBandSchema = z.object({
  type: z.enum([
    "peak",
    "lowpass",
    "highpass",
    "lowshelf",
    "highshelf",
    "notch",
    "bandpass",
  ]),
  freq: z.number().min(20).max(20000),
  gainDb: z.number().min(-24).max(24).optional(),
  q: z.number().min(0.1).max(20).optional().default(1.0),
});

export const FxNodeSchema = z.object({
  id: z.string(),
  type: z.enum([
    "gain",
    "panner",
    "parametricEq",
    "compressor",
    "plateReverb",
    "stereoDelay",
    "brickwallLimiter",
  ]),
  bypass: z.boolean().default(false),
  params: z.record(z.unknown()).default({}),
});

export type FxNode = z.infer<typeof FxNodeSchema>;

// ---- Channel / Bus (M2) ----
export const ChannelSchema = z.object({
  id: z.string(),
  gainDb: z.number().default(0),
  pan: z.number().min(-1).max(1).default(0),
  muted: z.boolean().default(false),
  solo: z.boolean().default(false),
  fx: z.array(FxNodeSchema).default([]),
  routeTo: z.string().optional(),
});

export type Channel = z.infer<typeof ChannelSchema>;

export const BusSchema = z.object({
  id: z.string(),
  fx: z.array(FxNodeSchema).default([]),
});

export type Bus = z.infer<typeof BusSchema>;
