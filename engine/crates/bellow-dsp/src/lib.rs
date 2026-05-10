//! bellow-dsp — DSP FX node library
//!
//! Every node implements the `Node` trait:
//!   fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap);
//!
//! All processing is real-time safe (no alloc, no lock, no syscall inside `process()`).
//! Parameter smoothing is handled per-node via `ParamRamp`.

pub mod error;
pub mod node;
pub mod gain;
pub mod panner;
pub mod eq;
pub mod compressor;
pub mod reverb;
pub mod delay;
pub mod limiter;
pub mod param;

pub use error::DspError;
pub use node::Node;
pub use param::{ParamMap, ParamRamp};
