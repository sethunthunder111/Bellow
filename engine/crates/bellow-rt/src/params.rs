//! Atomic parameter slots for sample-accurate parameter changes.
//!
//! The control thread writes a new value + sample offset into a shared slot.
//! The audio thread reads the value at the start of each block.

use std::sync::atomic::{AtomicU32, Ordering};

/// An f32 parameter that can be updated from the control thread and read
/// from the audio thread without locks.
pub struct AtomicF32 {
    bits: AtomicU32,
}

impl AtomicF32 {
    pub const fn new(value: f32) -> Self {
        Self {
            bits: AtomicU32::new(value.to_bits()),
        }
    }

    pub fn store(&self, value: f32) {
        self.bits.store(value.to_bits(), Ordering::Release);
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Acquire))
    }
}

/// A parameter message with a target sample time.
#[derive(Clone, Copy, Debug)]
pub struct ParamMsg {
    pub sample_time: u64,
    pub value: f32,
}
