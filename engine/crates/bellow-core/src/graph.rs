//! AudioGraph — runtime mixer connecting decoded sounds to the audio output.
//!
//! Owns:
//! - The active CPAL output stream (`bellow-io`)
//! - A shared `Mixer` of active `PlayingSound` voices
//! - Master volume / mute state applied per-sample on the audio thread
//!
//! The control-thread side mutates state via the `Engine` API. The audio
//! thread reads via `Arc<Mutex<MixerState>>` (taken briefly per buffer; OK
//! at audio-block granularity for M1, will be replaced with lock-free
//! SPSC primitives in M8).

use bellow_decode::{decode_file, DecodedAudio};
use bellow_io::{build_output_stream, OutputStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Lock-free per-sound state shared between control thread and audio thread.
/// All fields are atomics so the audio thread never blocks.
pub struct AtomicSound {
    pub id: String,
    pub samples: Arc<Vec<f32>>,
    pub channels: u16,
    pub sample_rate: u32,
    pub frame_pos: AtomicUsize,
    pub volume: AtomicUsize, // f32 bits stored as u32
    pub looping: AtomicBool,
    pub playing: AtomicBool,
}

impl AtomicSound {
    pub fn from_decoded(id: String, decoded: DecodedAudio) -> Self {
        Self {
            id,
            samples: Arc::new(decoded.samples),
            channels: decoded.channels,
            sample_rate: decoded.sample_rate,
            frame_pos: AtomicUsize::new(0),
            volume: AtomicUsize::new(1.0f32.to_bits() as usize),
            looping: AtomicBool::new(false),
            playing: AtomicBool::new(false),
        }
    }

    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0;
        }
        let frames = self.samples.len() / self.channels.max(1) as usize;
        ((frames as u64) * 1000) / self.sample_rate as u64
    }

    pub fn position_ms(&self) -> u64 {
        let pos = self.frame_pos.load(Ordering::Relaxed);
        if self.sample_rate == 0 {
            return 0;
        }
        ((pos as u64) * 1000) / self.sample_rate as u64
    }
}

/// Mixes active atomic sounds into the output buffer.
/// Called on the audio thread — must be lock-free and real-time safe.
fn mix_sounds(sounds: &[AtomicSound], out: &mut [f32], master: f32) {
    let out_ch = 2usize; // stereo output
    let frames = out.len() / out_ch;
    for sound in sounds {
        if !sound.playing.load(Ordering::Relaxed) {
            continue;
        }
        let in_ch = sound.channels.max(1) as usize;
        let total_in_frames = sound.samples.len() / in_ch;
        let vol = f32::from_bits(sound.volume.load(Ordering::Relaxed) as u32);
        let mut pos = sound.frame_pos.load(Ordering::Relaxed);
        let g = vol * master;

        for f in 0..frames {
            if pos >= total_in_frames {
                if sound.looping.load(Ordering::Relaxed) {
                    pos = 0;
                } else {
                    sound.playing.store(false, Ordering::Relaxed);
                    break;
                }
            }
            let src_idx = pos * in_ch;
            let (l, r) = if in_ch >= 2 {
                (sound.samples[src_idx], sound.samples[src_idx + 1])
            } else {
                let m = sound.samples[src_idx];
                (m, m)
            };
            out[f * 2] += l * g;
            out[f * 2 + 1] += r * g;
            pos += 1;
        }
        sound.frame_pos.store(pos, Ordering::Relaxed);
    }
}

/// Owns the live CPAL stream and the shared mixer.
pub struct AudioGraph {
    pub sounds: Arc<Mutex<Vec<AtomicSound>>>,
    pub master_volume: Arc<AtomicUsize>, // f32 bits
    pub muted: Arc<AtomicBool>,
    _stream: OutputStream,
    pub output_sample_rate: u32,
    pub output_channels: u16,
}

impl AudioGraph {
    /// Start an output stream and return a graph handle.
    ///
    /// Uses device defaults for rate and buffer size (most stable on
    /// Windows WASAPI shared mode). The actual rate is stored in
    /// `output_sample_rate` and used to drive resampling at load.
    pub fn start(sample_rate: u32, _buffer_size: u32) -> Result<Self, String> {
        let sounds: Arc<Mutex<Vec<AtomicSound>>> = Arc::new(Mutex::new(Vec::new()));
        let sounds_cb = Arc::clone(&sounds);
        let master_vol = Arc::new(AtomicUsize::new(1.0f32.to_bits() as usize));
        let master_vol_cb = Arc::clone(&master_vol);
        let muted = Arc::new(AtomicBool::new(false));
        let muted_cb = Arc::clone(&muted);
        let channels: u16 = 2;

        let device_sample_rate_hint = 0u32;
        let device_buffer_size_hint = 0u32;
        let _ = sample_rate;

        let stream = build_output_stream(
            device_sample_rate_hint,
            channels,
            device_buffer_size_hint,
            move |out| {
                out.fill(0.0);
                if muted_cb.load(Ordering::Relaxed) {
                    return;
                }
                let master = f32::from_bits(master_vol_cb.load(Ordering::Relaxed) as u32);
                let sound_list = match sounds_cb.lock() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                mix_sounds(&*sound_list, out, master);
            },
        )
        .map_err(|e| e.to_string())?;

        let actual_sample_rate = stream.sample_rate();
        let actual_channels = stream.channels();

        Ok(Self {
            sounds,
            master_volume: master_vol,
            muted,
            _stream: stream,
            output_sample_rate: actual_sample_rate,
            output_channels: actual_channels,
        })
    }

    /// Add a decoded sound to the mixer (initially not playing).
    /// Resamples the decoded audio to the output sample rate if needed.
    pub fn add_sound(&self, id: String, mut decoded: DecodedAudio) -> (u64, u64) {
        if decoded.sample_rate != self.output_sample_rate && decoded.channels > 0 {
            match bellow_resample::resample_offline(
                &decoded.samples,
                decoded.sample_rate,
                self.output_sample_rate,
                decoded.channels,
            ) {
                Ok(resampled) => {
                    decoded.samples = resampled;
                    decoded.sample_rate = self.output_sample_rate;
                }
                Err(e) => {
                    eprintln!(
                        "[bellow-core] resample {} -> {} failed: {} (playing at native rate, may be pitch-shifted)",
                        decoded.sample_rate, self.output_sample_rate, e
                    );
                }
            }
        }

        let sound = AtomicSound::from_decoded(id, decoded);
        let dur = sound.duration_ms();
        let pos = sound.position_ms();
        if let Ok(mut state) = self.sounds.lock() {
            state.push(sound);
        }
        (pos, dur)
    }

    pub fn set_playing(&self, id: &str, playing: bool) {
        if let Ok(state) = self.sounds.lock() {
            for s in state.iter() {
                if s.id == id {
                    s.playing.store(playing, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn stop(&self, id: &str) {
        if let Ok(state) = self.sounds.lock() {
            for s in state.iter() {
                if s.id == id {
                    s.playing.store(false, Ordering::Relaxed);
                    s.frame_pos.store(0, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn seek(&self, id: &str, position_ms: u64) {
        if let Ok(state) = self.sounds.lock() {
            for s in state.iter() {
                if s.id == id {
                    let target_frames = (position_ms as u64 * s.sample_rate as u64 / 1000) as usize;
                    s.frame_pos.store(target_frames, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn set_volume(&self, id: &str, volume: f32) {
        let bits = volume.clamp(0.0, 1.0).to_bits() as usize;
        if let Ok(state) = self.sounds.lock() {
            for s in state.iter() {
                if s.id == id {
                    s.volume.store(bits, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn set_rate(&self, _id: &str, _rate: f32) {
        // TODO: implement per-sound rate in M2
    }

    pub fn set_loop(&self, id: &str, looping: bool) {
        if let Ok(state) = self.sounds.lock() {
            for s in state.iter() {
                if s.id == id {
                    s.looping.store(looping, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn dispose(&self, id: &str) {
        if let Ok(mut state) = self.sounds.lock() {
            state.retain(|s| s.id != id);
        }
    }

    pub fn position_ms(&self, id: &str) -> Option<u64> {
        let state = self.sounds.lock().ok()?;
        state.iter().find(|s| s.id == id).map(|s| s.position_ms())
    }

    pub fn set_master_volume_lin(&self, lin: f32) {
        (*self.master_volume).store(lin.to_bits() as usize, Ordering::Relaxed);
    }

    pub fn set_muted(&self, muted: bool) {
        (*self.muted).store(muted, Ordering::Relaxed);
    }
}

/// Decode an audio file into a `DecodedAudio` block (control-thread).
pub fn decode_audio(path: &str) -> Result<DecodedAudio, String> {
    decode_file(path).map_err(|e| e.to_string())
}
