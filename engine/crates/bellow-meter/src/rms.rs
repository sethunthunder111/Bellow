//! RMS meter — sliding window mean-square.

#[derive(Debug, Clone)]
pub struct RmsMeter {
    channels: u16,
    window_size: usize,
    /// Circular buffer of squared samples per channel.
    buffer: Vec<f32>,
    idx: usize,
    sums: Vec<f32>,
}

impl RmsMeter {
    pub fn new(channels: u16, window_ms: f32, sample_rate: u32) -> Self {
        let window_size = ((window_ms / 1000.0) * sample_rate as f32) as usize;
        let ch = channels as usize;
        Self {
            channels,
            window_size,
            buffer: vec![0.0; window_size * ch],
            idx: 0,
            sums: vec![0.0; ch],
        }
    }

    pub fn process(&mut self, samples: &[f32]) {
        let ch = self.channels as usize;
        for frame in samples.chunks_exact(ch) {
            for (c, &s) in frame.iter().enumerate() {
                let sq = s * s;
                let old_idx = self.idx * ch + c;
                let old_sq = self.buffer[old_idx];
                self.sums[c] += sq - old_sq;
                self.buffer[old_idx] = sq;
            }
            self.idx = (self.idx + 1) % self.window_size;
        }
    }

    /// RMS per channel (linear).
    pub fn rms(&self) -> Vec<f32> {
        self.sums
            .iter()
            .map(|&s| (s / self.window_size as f32).sqrt())
            .collect()
    }

    /// RMS per channel in dBFS.
    pub fn rms_db(&self) -> Vec<f32> {
        self.rms()
            .iter()
            .map(|&r| if r > 0.0 { 20.0 * r.log10() } else { f32::NEG_INFINITY })
            .collect()
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.sums.fill(0.0);
        self.idx = 0;
    }
}
