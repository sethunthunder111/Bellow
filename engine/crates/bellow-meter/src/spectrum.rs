//! Spectrum analyzer — sliding FFT with configurable window.

#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    fft_size: usize,
    hop_size: usize,
    window: Vec<f32>,
    /// Circular input buffer
    ring: Vec<f32>,
    ring_idx: usize,
    /// FFT input buffer
    fft_in: Vec<f32>,
    /// Smoothed magnitude bins
    bins: Vec<f32>,
    smooth: f32,
    sample_count: usize,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize, hop_size: usize, smoothing: f32) -> Self {
        let window = hann_window(fft_size);
        Self {
            fft_size,
            hop_size,
            window,
            ring: vec![0.0; fft_size],
            ring_idx: 0,
            fft_in: vec![0.0; fft_size],
            bins: vec![0.0; fft_size / 2],
            smooth: smoothing.clamp(0.0, 0.99),
            sample_count: 0,
        }
    }

    /// Process interleaved stereo samples (analyzes mono sum).
    pub fn process(&mut self, samples: &[f32]) {
        for chunk in samples.chunks_exact(2) {
            let mono = (chunk[0] + chunk[1]) * 0.5;
            self.ring[self.ring_idx] = mono;
            self.ring_idx = (self.ring_idx + 1) % self.fft_size;
            self.sample_count += 1;

            if self.sample_count >= self.hop_size {
                self.sample_count = 0;
                self.run_fft();
            }
        }
    }

    fn run_fft(&mut self) {
        // Copy ring to fft_in in order (linearize circular buffer)
        for i in 0..self.fft_size {
            let idx = (self.ring_idx + i) % self.fft_size;
            self.fft_in[i] = self.ring[idx] * self.window[i];
        }

        // Simple DFT magnitude (no external FFT lib yet — placeholder with optimized inner loop)
        let n = self.fft_size;
        let half = n / 2;
        use std::f32::consts::PI;

        for k in 0..half {
            let mut real = 0.0f32;
            let mut imag = 0.0f32;
            for (i, &sample) in self.fft_in.iter().enumerate() {
                let angle = -2.0 * PI * (k as f32) * (i as f32) / (n as f32);
                real += sample * angle.cos();
                imag += sample * angle.sin();
            }
            let mag = (real * real + imag * imag).sqrt() / (n as f32);
            self.bins[k] = self.smooth * self.bins[k] + (1.0 - self.smooth) * mag;
        }
    }

    /// Get magnitude bins (length = fft_size / 2).
    pub fn bins(&self) -> &[f32] {
        &self.bins
    }

    /// Get bins in dB (clamped to -120 dB).
    pub fn bins_db(&self) -> Vec<f32> {
        self.bins
            .iter()
            .map(|&b| {
                if b > 0.0 {
                    20.0 * b.max(1e-6).log10()
                } else {
                    -120.0
                }
            })
            .collect()
    }

    pub fn reset(&mut self) {
        self.ring.fill(0.0);
        self.ring_idx = 0;
        self.fft_in.fill(0.0);
        self.bins.fill(0.0);
        self.sample_count = 0;
    }
}

fn hann_window(size: usize) -> Vec<f32> {
    use std::f32::consts::PI;
    (0..size)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * (i as f32) / (size as f32)).cos())
        .collect()
}
