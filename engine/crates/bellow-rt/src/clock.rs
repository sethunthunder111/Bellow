//! Sample-accurate clock for the audio thread.

/// A monotonic sample counter. Increments by block_size every callback.
#[derive(Clone, Copy, Debug, Default)]
pub struct SampleClock {
    sample_rate: u32,
    samples: u64,
}

impl SampleClock {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: 0,
        }
    }

    pub fn advance(&mut self, block_size: usize) {
        self.samples += block_size as u64;
    }

    pub fn samples(&self) -> u64 {
        self.samples
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn seconds(&self) -> f64 {
        self.samples as f64 / self.sample_rate as f64
    }
}
