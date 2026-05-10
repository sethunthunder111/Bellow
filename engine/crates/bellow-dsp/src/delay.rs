use crate::node::Node;
use crate::param::ParamMap;

/// Stereo delay with feedback and optional ping-pong.
pub struct StereoDelayNode {
    sample_rate: f32,
    delay_l_ms: f32,
    delay_r_ms: f32,
    feedback: f32,
    mix: f32,
    ping_pong: bool,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    idx_l: usize,
    idx_r: usize,
    delay_l_samples: usize,
    delay_r_samples: usize,
}

impl StereoDelayNode {
    pub fn new(
        sample_rate: u32,
        delay_l_ms: f32,
        delay_r_ms: f32,
        feedback: f32,
        mix: f32,
        ping_pong: bool,
    ) -> Self {
        let sr = sample_rate as f32;
        let max_delay = ((delay_l_ms.max(delay_r_ms) / 1000.0) * sr * 2.0) as usize + 4096;
        Self {
            sample_rate: sr,
            delay_l_ms,
            delay_r_ms,
            feedback: feedback.clamp(0.0, 0.99),
            mix: mix.clamp(0.0, 1.0),
            ping_pong,
            buffer_l: vec![0.0; max_delay],
            buffer_r: vec![0.0; max_delay],
            idx_l: 0,
            idx_r: 0,
            delay_l_samples: ((delay_l_ms / 1000.0) * sr) as usize,
            delay_r_samples: ((delay_r_ms / 1000.0) * sr) as usize,
        }
    }
}

impl Node for StereoDelayNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let feedback = params.get("feedback").copied().unwrap_or(self.feedback).clamp(0.0, 0.99);
        let mix = params.get("mix").copied().unwrap_or(self.mix).clamp(0.0, 1.0);

        for chunk in input.chunks_exact(2) {
            let in_l = chunk[0];
            let in_r = chunk[1];

            let len_l = self.buffer_l.len();
            let len_r = self.buffer_r.len();

            let read_l = (self.idx_l + len_l - self.delay_l_samples) % len_l;
            let read_r = (self.idx_r + len_r - self.delay_r_samples) % len_r;

            let delayed_l = self.buffer_l[read_l];
            let delayed_r = self.buffer_r[read_r];

            let (wet_l, wet_r) = if self.ping_pong {
                // Cross-feed for ping-pong
                (delayed_r, delayed_l)
            } else {
                (delayed_l, delayed_r)
            };

            self.buffer_l[self.idx_l] = in_l + wet_l * feedback;
            self.buffer_r[self.idx_r] = in_r + wet_r * feedback;

            self.idx_l = (self.idx_l + 1) % len_l;
            self.idx_r = (self.idx_r + 1) % len_r;

            output[0] = in_l * (1.0 - mix) + wet_l * mix;
            output[1] = in_r * (1.0 - mix) + wet_r * mix;
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "stereo_delay"
    }

    fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.idx_l = 0;
        self.idx_r = 0;
    }
}
