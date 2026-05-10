use crate::error::{EngineError, EngineResult};
use crate::graph::AudioGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub internal_precision: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            buffer_size: 256,
            internal_precision: "f32".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub version: String,
    pub rust_version: String,
    pub supported_backends: Vec<String>,
}

impl EngineInfo {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
            supported_backends: vec![
                "wasapi".to_string(),
                "coreaudio".to_string(),
                "alsa".to_string(),
                "jack".to_string(),
                "pipewire".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundHandle {
    pub id: String,
    pub src: String,
    pub state: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub rate: f32,
    pub loop_playback: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterState {
    pub volume_db: f32,
    pub muted: bool,
}

impl Default for MasterState {
    fn default() -> Self {
        Self {
            volume_db: 0.0,
            muted: false,
        }
    }
}

/// The main audio engine. Thread-safe via interior mutability.
pub struct Engine {
    state: Arc<Mutex<EngineState>>,
}

#[derive(Default)]
struct EngineState {
    initialized: bool,
    config: Option<EngineConfig>,
    sounds: HashMap<String, SoundHandle>,
    master: MasterState,
    next_sound_id: u64,
    graph: Option<AudioGraph>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(EngineState::default())),
        }
    }

    pub fn init(&self, config: EngineConfig) -> EngineResult<()> {
        let mut state = self.state.lock().unwrap();
        if state.initialized {
            return Err(EngineError::AlreadyInitialized);
        }
        if config.sample_rate < 22050 || config.sample_rate > 384000 {
            return Err(EngineError::InvalidSampleRate(config.sample_rate));
        }
        if config.buffer_size < 16 || config.buffer_size > 4096 {
            return Err(EngineError::InvalidBufferSize(config.buffer_size));
        }
        let sample_rate = config.sample_rate;
        let buffer_size = config.buffer_size as u32;
        let graph = AudioGraph::start(sample_rate, buffer_size)
            .map_err(|e| EngineError::Other(format!("Audio graph start failed: {}", e)))?;
        state.graph = Some(graph);
        state.config = Some(config);
        state.initialized = true;
        Ok(())
    }

    pub fn shutdown(&self) -> EngineResult<()> {
        let mut state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        state.initialized = false;
        state.config = None;
        state.sounds.clear();
        state.graph = None;
        Ok(())
    }

    pub fn suspend(&self) -> EngineResult<()> {
        let state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        Ok(())
    }

    pub fn resume(&self) -> EngineResult<()> {
        let state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        Ok(())
    }

    pub fn version(&self) -> EngineInfo {
        EngineInfo::current()
    }

    pub fn is_initialized(&self) -> bool {
        self.state.lock().unwrap().initialized
    }

    pub fn sound_load(&self, src: &str) -> EngineResult<SoundHandle> {
        // Decode outside the lock (I/O bound)
        let decoded = crate::graph::decode_audio(src)
            .map_err(|e| EngineError::Other(format!("Decode failed: {}", e)))?;

        let mut state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        let id = format!("sound-{}", state.next_sound_id);
        state.next_sound_id += 1;

        let duration_ms = if decoded.sample_rate == 0 || decoded.channels == 0 {
            0
        } else {
            let frames = decoded.samples.len() / decoded.channels.max(1) as usize;
            ((frames as u64) * 1000) / decoded.sample_rate as u64
        };

        if let Some(graph) = &state.graph {
            graph.add_sound(id.clone(), decoded);
        }

        let handle = SoundHandle {
            id: id.clone(),
            src: src.to_string(),
            state: "loaded".to_string(),
            position_ms: 0,
            duration_ms,
            volume: 1.0,
            rate: 1.0,
            loop_playback: false,
        };
        state.sounds.insert(id, handle.clone());
        Ok(handle)
    }

    pub fn sound_play(&self, id: &str) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.set_playing(id, true);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.state = "playing".to_string();
        Ok(())
    }

    pub fn sound_pause(&self, id: &str) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.set_playing(id, false);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        if sound.state == "playing" {
            sound.state = "paused".to_string();
        }
        Ok(())
    }

    pub fn sound_stop(&self, id: &str) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.stop(id);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.state = "stopped".to_string();
        sound.position_ms = 0;
        Ok(())
    }

    pub fn sound_seek(&self, id: &str, position_ms: u64) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.seek(id, position_ms);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.position_ms = position_ms;
        Ok(())
    }

    pub fn sound_set_volume(&self, id: &str, volume: f32) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.set_volume(id, volume);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.volume = volume.clamp(0.0, 1.0);
        Ok(())
    }

    pub fn sound_set_rate(&self, id: &str, rate: f32) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.set_rate(id, rate);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.rate = rate.max(0.01);
        Ok(())
    }

    pub fn sound_set_loop(&self, id: &str, loop_playback: bool) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.set_loop(id, loop_playback);
            }
        }
        let mut state = self.state.lock().unwrap();
        let sound = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        sound.loop_playback = loop_playback;
        Ok(())
    }

    pub fn sound_dispose(&self, id: &str) -> EngineResult<()> {
        {
            let state = self.state.lock().unwrap();
            if let Some(graph) = &state.graph {
                graph.dispose(id);
            }
        }
        let mut state = self.state.lock().unwrap();
        state
            .sounds
            .remove(id)
            .ok_or_else(|| EngineError::SoundNotFound(id.to_string()))?;
        Ok(())
    }

    pub fn sound_list(&self) -> Vec<SoundHandle> {
        self.state
            .lock()
            .unwrap()
            .sounds
            .values()
            .cloned()
            .collect()
    }

    pub fn master_set_volume(&self, volume_db: f32) -> EngineResult<()> {
        let mut state = self.state.lock().unwrap();
        state.master.volume_db = volume_db.clamp(-120.0, 24.0);
        let lin = 10f32.powf(state.master.volume_db / 20.0);
        if let Some(graph) = &state.graph {
            graph.set_master_volume_lin(lin);
        }
        Ok(())
    }

    pub fn master_get(&self) -> MasterState {
        self.state.lock().unwrap().master.clone()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
