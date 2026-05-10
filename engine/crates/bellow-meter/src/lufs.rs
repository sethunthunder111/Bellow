//! LUFS meter — ITU-R BS.1770-4 implementation.
//!
//! Simplified: K-weighted pre-filtering + 400ms sliding window.

use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct LufsMeter {
    sample_rate: f32,
    /// Pre-filter coefficients (2nd-order high-shelf)
    pre_b0: f32, pre_b1: f32, pre_b2: f32,
    pre_a1: f32, pre_a2: f32,
    pre_z1_l: f32, pre_z2_l: f32,
    pre_z1_r: f32, pre_z2_r: f32,

    /// RLB filter coefficients (2nd-order high-pass)
    rlb_b0: f32, rlb_b1: f32, rlb_b2: f32,
    rlb_a1: f32, rlb_a2: f32,
    rlb_z1_l: f32, rlb_z2_l: f32,
    rlb_z1_r: f32, rlb_z2_r: f32,

    /// 400ms window
    window_size: usize,
    loudness_sum: f32,
    loudness_buffer: Vec<f32>,
    loudness_idx: usize,
}

impl LufsMeter {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;

        // Pre-filter: high-shelf at ~1.5 kHz, +4 dB gain
        let f0 = 1500.0;
        let q = 0.5;
        let g = 10f32.powf(4.0 / 20.0);
        let w0 = 2.0 * PI * f0 / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let sqrt_2a = 2.0 * g.sqrt() * alpha;
        let a0_pre = (g + 1.0) - (g - 1.0) * cos_w0 + sqrt_2a;
        let pre_b0 = g * ((g + 1.0) + (g - 1.0) * cos_w0 + sqrt_2a) / a0_pre;
        let pre_b1 = -2.0 * g * ((g - 1.0) + (g + 1.0) * cos_w0) / a0_pre;
        let pre_b2 = g * ((g + 1.0) + (g - 1.0) * cos_w0 - sqrt_2a) / a0_pre;
        let pre_a1 = 2.0 * ((g - 1.0) - (g + 1.0) * cos_w0) / a0_pre;
        let pre_a2 = ((g + 1.0) - (g - 1.0) * cos_w0 - sqrt_2a) / a0_pre;

        // RLB filter: 2nd-order HP at ~40 Hz
        let f0_rlb = 38.13547096;
        let w0_rlb = 2.0 * PI * f0_rlb / sr;
        let cos_rlb = w0_rlb.cos();
        let sin_rlb = w0_rlb.sin();
        let q_rlb = 0.5;
        let alpha_rlb = sin_rlb / (2.0 * q_rlb);
        let a0_rlb = 1.0 + alpha_rlb;
        let rlb_b0 = ((1.0 + cos_rlb) / 2.0) / a0_rlb;
        let rlb_b1 = (-(1.0 + cos_rlb)) / a0_rlb;
        let rlb_b2 = ((1.0 + cos_rlb) / 2.0) / a0_rlb;
        let rlb_a1 = (-2.0 * cos_rlb) / a0_rlb;
        let rlb_a2 = (1.0 - alpha_rlb) / a0_rlb;

        let window_size = ((0.4) * sr) as usize;

        Self {
            sample_rate: sr,
            pre_b0, pre_b1, pre_b2, pre_a1, pre_a2,
            pre_z1_l: 0.0, pre_z2_l: 0.0, pre_z1_r: 0.0, pre_z2_r: 0.0,
            rlb_b0, rlb_b1, rlb_b2, rlb_a1, rlb_a2,
            rlb_z1_l: 0.0, rlb_z2_l: 0.0, rlb_z1_r: 0.0, rlb_z2_r: 0.0,
            window_size,
            loudness_sum: 0.0,
            loudness_buffer: vec![0.0; window_size],
            loudness_idx: 0,
        }
    }

    fn process_sample(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Pre-filter (K-weighting high-shelf)
        let pre_l = self.pre_b0 * l + self.pre_b1 * self.pre_z1_l + self.pre_b2 * self.pre_z2_l
            - self.pre_a1 * self.pre_z1_l - self.pre_a2 * self.pre_z2_l;
        self.pre_z2_l = self.pre_z1_l;
        self.pre_z1_l = pre_l;

        let pre_r = self.pre_b0 * r + self.pre_b1 * self.pre_z1_r + self.pre_b2 * self.pre_z2_r
            - self.pre_a1 * self.pre_z1_r - self.pre_a2 * self.pre_z2_r;
        self.pre_z2_r = self.pre_z1_r;
        self.pre_z1_r = pre_r;

        // RLB filter
        let rlb_l = self.rlb_b0 * pre_l + self.rlb_b1 * self.rlb_z1_l + self.rlb_b2 * self.rlb_z2_l
            - self.rlb_a1 * self.rlb_z1_l - self.rlb_a2 * self.rlb_z2_l;
        self.rlb_z2_l = self.rlb_z1_l;
        self.rlb_z1_l = rlb_l;

        let rlb_r = self.rlb_b0 * pre_r + self.rlb_b1 * self.rlb_z1_r + self.rlb_b2 * self.rlb_z2_r
            - self.rlb_a1 * self.rlb_z1_r - self.rlb_a2 * self.rlb_z2_r;
        self.rlb_z2_r = self.rlb_z1_r;
        self.rlb_z1_r = rlb_r;

        (rlb_l, rlb_r)
    }

    pub fn process(&mut self, samples: &[f32]) {
        for chunk in samples.chunks_exact(2) {
            let (l, r) = self.process_sample(chunk[0], chunk[1]);
            let power = l * l + r * r;
            let old = self.loudness_buffer[self.loudness_idx];
            self.loudness_sum += power - old;
            self.loudness_buffer[self.loudness_idx] = power;
            self.loudness_idx = (self.loudness_idx + 1) % self.window_size;
        }
    }

    /// Integrated LUFS (momentary, 400ms window).
    pub fn integrated_lufs(&self) -> f32 {
        let mean_power = self.loudness_sum / self.window_size as f32;
        if mean_power <= 0.0 {
            f32::NEG_INFINITY
        } else {
            -0.691 + 10.0 * mean_power.log10()
        }
    }

    pub fn reset(&mut self) {
        self.pre_z1_l = 0.0; self.pre_z2_l = 0.0;
        self.pre_z1_r = 0.0; self.pre_z2_r = 0.0;
        self.rlb_z1_l = 0.0; self.rlb_z2_l = 0.0;
        self.rlb_z1_r = 0.0; self.rlb_z2_r = 0.0;
        self.loudness_sum = 0.0;
        self.loudness_buffer.fill(0.0);
        self.loudness_idx = 0;
    }
}
