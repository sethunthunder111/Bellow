use crate::node::Node;
use crate::param::ParamMap;

/// Brick-wall peak limiter with lookahead.
pub struct BrickwallLimiterNode {
    sample_rate: f32,
    ceiling_db: f32,
    lookahead_ms: f32,
    release_ms: f32,
    /// Delay line for lookahead
    delay_buffer: Vec<f32>,
    delay_idx: usize,
    delay_len: usize,
    /// Envelope follower
    envelope: f32,
    release_coeff: f32,
}

impl BrickwallLimiterNode {
    pub fn new(sample_rate: u32, ceiling_db: f32, lookahead_ms: f32, release_ms: f32) -> Self {
        let sr = sample_rate as f32;
        let delay_len = ((lookahead_ms / 1000.0) * sr) as usize + 1;
        Self {
            sample_rate: sr,
            ceiling_db,
            lookahead_ms,
            release_ms,
            delay_buffer: vec![0.0; delay_len],
            delay_idx: 0,
            delay_len,
            envelope: 1.0,
            release_coeff: (-1000.0 / (release_ms * sr)).exp(),
        }
    }

    fn db_to_linear(db: f32) -> f32 {
        10f32.powf(db / 20.0)
    }

    fn linear_to_db(linear: f32) -> f32 {
        20.0 * linear.max(1e-10).log10()
    }
}

impl Node for BrickwallLimiterNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let ceiling = params.get("ceiling").copied().unwrap_or(self.ceiling_db);
        let ceiling_lin = Self::db_to_linear(ceiling);

        for chunk in input.chunks_exact(2) {
            let l = chunk[0].abs();
            let r = chunk[1].abs();
            let peak = l.max(r);

            let target_gain = if peak > ceiling_lin {
                ceiling_lin / peak
            } else {
                1.0
            };

            if target_gain < self.envelope {
                self.envelope = target_gain; // instant attack (lookahead handles it)
            } else {
                self.envelope = self.release_coeff * (self.envelope - target_gain) + target_gain;
            }

            let g = self.envelope;

            // Lookahead delay: read delayed sample, apply gain
            let delayed = [
                self.delay_buffer[self.delay_idx * 2],
                self.delay_buffer[self.delay_idx * 2 + 1],
            ];

            // Write current sample into delay line
            self.delay_buffer[self.delay_idx * 2] = chunk[0];
            self.delay_buffer[self.delay_idx * 2 + 1] = chunk[1];
            self.delay_idx = (self.delay_idx + 1) % self.delay_len;

            output[0] = delayed[0] * g;
            output[1] = delayed[1] * g;
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "brickwall_limiter"
    }

    fn reset(&mut self) {
        self.delay_buffer.fill(0.0);
        self.delay_idx = 0;
        self.envelope = 1.0;
    }
}
