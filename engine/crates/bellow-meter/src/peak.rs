//! Peak meter — tracks per-channel peak hold with optional decay.

#[derive(Debug, Clone)]
pub struct PeakMeter {
    channels: u16,
    peaks: Vec<f32>,
    holds: Vec<f32>,
    decay_per_sample: f32,
}

impl PeakMeter {
    pub fn new(channels: u16, sample_rate: u32, decay_db_per_s: f32) -> Self {
        let decay_per_sample = decay_db_per_s / (sample_rate as f32);
        Self {
            channels,
            peaks: vec![0.0; channels as usize],
            holds: vec![0.0; channels as usize],
            decay_per_sample,
        }
    }

    /// Process a block of interleaved f32 samples.
    pub fn process(&mut self, samples: &[f32]) {
        let ch = self.channels as usize;
        for frame in samples.chunks_exact(ch) {
            for (i, &s) in frame.iter().enumerate() {
                let abs = s.abs();
                if abs > self.holds[i] {
                    self.holds[i] = abs;
                }
            }
        }

        // Apply decay to peaks
        for i in 0..ch {
            self.holds[i] = (self.holds[i] - self.decay_per_sample).max(0.0);
            self.peaks[i] = self.holds[i];
        }
    }

    /// Get current peak per channel (linear).
    pub fn peaks(&self) -> &[f32] {
        &self.peaks
    }

    /// Get current peak in dBFS.
    pub fn peaks_db(&self) -> Vec<f32> {
        self.peaks
            .iter()
            .map(|&p| if p > 0.0 { 20.0 * p.log10() } else { f32::NEG_INFINITY })
            .collect()
    }

    pub fn reset(&mut self) {
        self.peaks.fill(0.0);
        self.holds.fill(0.0);
    }
}
