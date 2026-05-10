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
use std::sync::{Arc, Mutex};

/// Per-sound playback state living on the audio thread.
pub struct PlayingSound {
    pub id: String,
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub frame_pos: usize,
    pub volume: f32,
    pub rate: f32,
    pub looping: bool,
    pub playing: bool,
}

impl PlayingSound {
    pub fn from_decoded(id: String, decoded: DecodedAudio) -> Self {
        Self {
            id,
            samples: decoded.samples,
            channels: decoded.channels,
            sample_rate: decoded.sample_rate,
            frame_pos: 0,
            volume: 1.0,
            rate: 1.0,
            looping: false,
            playing: false,
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
        if self.sample_rate == 0 {
            return 0;
        }
        ((self.frame_pos as u64) * 1000) / self.sample_rate as u64
    }
}

#[derive(Default)]
pub struct MixerState {
    pub sounds: Vec<PlayingSound>,
    pub master_volume_lin: f32,
    pub muted: bool,
}

impl MixerState {
    pub fn new() -> Self {
        Self {
            sounds: Vec::new(),
            master_volume_lin: 1.0,
            muted: false,
        }
    }
}

/// Owns the live CPAL stream and the shared mixer.
pub struct AudioGraph {
    pub mixer: Arc<Mutex<MixerState>>,
    _stream: OutputStream,
    pub output_sample_rate: u32,
    pub output_channels: u16,
}

impl AudioGraph {
    /// Start an output stream and return a graph handle.
    pub fn start(sample_rate: u32, buffer_size: u32) -> Result<Self, String> {
        let mixer = Arc::new(Mutex::new(MixerState::new()));
        let mixer_cb = Arc::clone(&mixer);
        let channels: u16 = 2;

        let stream = build_output_stream(sample_rate, channels, buffer_size, move |out| {
            // Zero out the buffer first
            out.fill(0.0);

            let mut state = match mixer_cb.lock() {
                Ok(s) => s,
                Err(_) => return,
            };

            if state.muted {
                return;
            }
            let master = state.master_volume_lin;

            // Mix every playing sound into `out` (interleaved stereo).
            let frames = out.len() / channels as usize;
            for sound in state.sounds.iter_mut() {
                if !sound.playing {
                    continue;
                }
                let in_ch = sound.channels.max(1) as usize;
                let total_in_frames = sound.samples.len() / in_ch;

                for f in 0..frames {
                    if sound.frame_pos >= total_in_frames {
                        if sound.looping {
                            sound.frame_pos = 0;
                        } else {
                            sound.playing = false;
                            break;
                        }
                    }

                    let src_idx = sound.frame_pos * in_ch;
                    let (l, r) = if in_ch >= 2 {
                        (sound.samples[src_idx], sound.samples[src_idx + 1])
                    } else {
                        let m = sound.samples[src_idx];
                        (m, m)
                    };

                    let g = sound.volume * master;
                    out[f * 2] += l * g;
                    out[f * 2 + 1] += r * g;

                    sound.frame_pos += 1;
                }
            }
        })
        .map_err(|e| e.to_string())?;

        Ok(Self {
            mixer,
            _stream: stream,
            output_sample_rate: sample_rate,
            output_channels: channels,
        })
    }

    /// Add a decoded sound to the mixer (initially not playing).
    pub fn add_sound(&self, id: String, decoded: DecodedAudio) -> (u64, u64) {
        let sound = PlayingSound::from_decoded(id, decoded);
        let dur = sound.duration_ms();
        let pos = sound.position_ms();
        if let Ok(mut state) = self.mixer.lock() {
            state.sounds.push(sound);
        }
        (pos, dur)
    }

    pub fn set_playing(&self, id: &str, playing: bool) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    s.playing = playing;
                }
            }
        }
    }

    pub fn stop(&self, id: &str) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    s.playing = false;
                    s.frame_pos = 0;
                }
            }
        }
    }

    pub fn seek(&self, id: &str, position_ms: u64) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    let target_frames = (position_ms as u64 * s.sample_rate as u64 / 1000) as usize;
                    s.frame_pos = target_frames;
                }
            }
        }
    }

    pub fn set_volume(&self, id: &str, volume: f32) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    s.volume = volume.clamp(0.0, 1.0);
                }
            }
        }
    }

    pub fn set_rate(&self, id: &str, rate: f32) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    s.rate = rate.max(0.01);
                }
            }
        }
    }

    pub fn set_loop(&self, id: &str, looping: bool) {
        if let Ok(mut state) = self.mixer.lock() {
            for s in state.sounds.iter_mut() {
                if s.id == id {
                    s.looping = looping;
                }
            }
        }
    }

    pub fn dispose(&self, id: &str) {
        if let Ok(mut state) = self.mixer.lock() {
            state.sounds.retain(|s| s.id != id);
        }
    }

    pub fn position_ms(&self, id: &str) -> Option<u64> {
        let state = self.mixer.lock().ok()?;
        state.sounds.iter().find(|s| s.id == id).map(|s| s.position_ms())
    }

    pub fn set_master_volume_lin(&self, lin: f32) {
        if let Ok(mut state) = self.mixer.lock() {
            state.master_volume_lin = lin;
        }
    }

    pub fn set_muted(&self, muted: bool) {
        if let Ok(mut state) = self.mixer.lock() {
            state.muted = muted;
        }
    }
}

/// Decode an audio file into a `DecodedAudio` block (control-thread).
pub fn decode_audio(path: &str) -> Result<DecodedAudio, String> {
    decode_file(path).map_err(|e| e.to_string())
}
