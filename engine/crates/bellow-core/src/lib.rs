//! bellow-core — audio engine core
//!
//! Owns the audio graph, scheduler, and device I/O controller.
//! All public methods are intended to be called from the control thread.

pub mod engine;
pub mod error;
pub mod graph;

pub use engine::{Engine, EngineConfig, EngineInfo, MasterState, SoundHandle};
pub use error::EngineError;
pub use graph::{AudioGraph, MixerState, PlayingSound};
