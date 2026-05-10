use crate::error::{EngineError, EngineResult};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// The main audio engine. Thread-safe via interior mutability.
pub struct Engine {
    state: Arc<Mutex<EngineState>>,
}

#[derive(Default)]
struct EngineState {
    initialized: bool,
    config: Option<EngineConfig>,
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
        Ok(())
    }

    pub fn suspend(&self) -> EngineResult<()> {
        let state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        // TODO: pause audio callback
        Ok(())
    }

    pub fn resume(&self) -> EngineResult<()> {
        let state = self.state.lock().unwrap();
        if !state.initialized {
            return Err(EngineError::NotInitialized);
        }
        // TODO: resume audio callback
        Ok(())
    }

    pub fn version(&self) -> EngineInfo {
        EngineInfo::current()
    }

    pub fn is_initialized(&self) -> bool {
        self.state.lock().unwrap().initialized
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
