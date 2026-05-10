//! bellow-rt — real-time audio primitives
//!
//! Lock-free SPSC ring buffers, atomic parameter slots, and a sample-accurate
//! clock for the audio thread. Nothing in this crate allocates on the hot path.

pub mod ringbuf;
pub mod params;
pub mod clock;
