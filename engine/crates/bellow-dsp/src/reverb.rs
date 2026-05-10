use crate::node::Node;
use crate::param::ParamMap;
use std::f32::consts::PI;

/// Schroeder-Moorer style plate reverb with 4 all-pass + 4 comb filters.
pub struct PlateReverbNode {
    sample_rate: f32,
    /// Wet/dry mix (0 = dry, 1 = wet)
    mix: f32,
    /// Decay time in seconds
    decay: f32,
    /// Pre-delay in ms
    predelay_ms: f32,
    /// Damping (0–1, high = darker)
    damping: f32,
    /// Modulation depth for allpass delays
    mod_depth: f32,

    // All-pass delays
    ap_delay_len: [usize; 4],
    ap_buffer: [Vec<f32>; 4],
    ap_idx: [usize; 4],

    // Comb delays
    comb_len: [usize; 4],
    comb_buffer: [Vec<f32>; 4],
    comb_idx: [usize; 4],
    comb_fb: [f32; 4],

    // Pre-delay
    predelay_buffer: Vec<f32>,
    predelay_idx: usize,
    predelay_len: usize,

    lfo_phase: f32,
}

impl PlateReverbNode {
    pub fn new(sample_rate: u32, decay: f32, predelay_ms: f32, mix: f32, damping: f32) -> Self {
        let sr = sample_rate as f32;

        let ap_lens = [
            (0.00501 * sr) as usize,
            (0.00711 * sr) as usize,
            (0.01341 * sr) as usize,
            (0.01701 * sr) as usize,
        ];

        let comb_lens = [
            (0.0297 * sr) as usize,
            (0.0371 * sr) as usize,
            (0.0411 * sr) as usize,
            (0.0437 * sr) as usize,
        ];

        let predelay_len = ((predelay_ms / 1000.0) * sr) as usize;

        Self {
            sample_rate: sr,
            mix: mix.clamp(0.0, 1.0),
            decay,
            predelay_ms,
            damping: damping.clamp(0.0, 1.0),
            mod_depth: 0.5,
            ap_delay_len: ap_lens,
            ap_buffer: [
                vec![0.0; ap_lens[0]],
                vec![0.0; ap_lens[1]],
                vec![0.0; ap_lens[2]],
                vec![0.0; ap_lens[3]],
            ],
            ap_idx: [0; 4],
            comb_len: comb_lens,
            comb_buffer: [
                vec![0.0; comb_lens[0]],
                vec![0.0; comb_lens[1]],
                vec![0.0; comb_lens[2]],
                vec![0.0; comb_lens[3]],
            ],
            comb_idx: [0; 4],
            comb_fb: [
                10f32.powf(-3.0 * comb_lens[0] as f32 / sr / decay),
                10f32.powf(-3.0 * comb_lens[1] as f32 / sr / decay),
                10f32.powf(-3.0 * comb_lens[2] as f32 / sr / decay),
                10f32.powf(-3.0 * comb_lens[3] as f32 / sr / decay),
            ],
            predelay_buffer: vec![0.0; predelay_len.max(1)],
            predelay_idx: 0,
            predelay_len,
            lfo_phase: 0.0,
        }
    }

    fn allpass(&mut self, idx: usize, input: f32) -> f32 {
        let len = self.ap_delay_len[idx];
        let i = self.ap_idx[idx];
        let delayed = self.ap_buffer[idx][i];
        let out = -input + delayed;
        self.ap_buffer[idx][i] = input + delayed * 0.5;
        self.ap_idx[idx] = (i + 1) % len;
        out
    }

    fn comb(&mut self, idx: usize, input: f32) -> f32 {
        let len = self.comb_len[idx];
        let i = self.comb_idx[idx];
        let delayed = self.comb_buffer[idx][i];
        let damped = delayed * (1.0 - self.damping) + self.comb_buffer[idx][(i + len - 1) % len] * self.damping;
        let out = damped;
        self.comb_buffer[idx][i] = input + damped * self.comb_fb[idx];
        self.comb_idx[idx] = (i + 1) % len;
        out
    }
}

impl Node for PlateReverbNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let mix = params.get("mix").copied().unwrap_or(self.mix).clamp(0.0, 1.0);
        let decay = params.get("decay").copied().unwrap_or(self.decay);

        // Update feedback coefficients if decay changed
        if (decay - self.decay).abs() > 0.01 {
            self.decay = decay;
            for i in 0..4 {
                self.comb_fb[i] = 10f32.powf(-3.0 * self.comb_len[i] as f32 / self.sample_rate / self.decay);
            }
        }

        for chunk in input.chunks_exact(2) {
            let mono = (chunk[0] + chunk[1]) * 0.5;

            // Pre-delay
            let pre = if self.predelay_len > 0 {
                let out = self.predelay_buffer[self.predelay_idx];
                self.predelay_buffer[self.predelay_idx] = mono;
                self.predelay_idx = (self.predelay_idx + 1) % self.predelay_len;
                out
            } else {
                mono
            };

            // All-pass cascade
            let mut s = pre;
            for i in 0..4 {
                s = self.allpass(i, s);
            }

            // Parallel comb filters
            let mut comb_sum = 0.0;
            for i in 0..4 {
                comb_sum += self.comb(i, s);
            }
            comb_sum *= 0.25;

            let wet_l = comb_sum;
            let wet_r = comb_sum;

            output[0] = chunk[0] * (1.0 - mix) + wet_l * mix;
            output[1] = chunk[1] * (1.0 - mix) + wet_r * mix;
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "plate_reverb"
    }

    fn reset(&mut self) {
        for buf in self.ap_buffer.iter_mut() {
            buf.fill(0.0);
        }
        for buf in self.comb_buffer.iter_mut() {
            buf.fill(0.0);
        }
        self.predelay_buffer.fill(0.0);
        self.ap_idx = [0; 4];
        self.comb_idx = [0; 4];
        self.predelay_idx = 0;
    }
}
