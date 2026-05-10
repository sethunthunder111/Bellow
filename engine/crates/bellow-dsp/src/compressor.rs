use crate::node::Node;
use crate::param::ParamMap;

/// Feedforward peak compressor (VCA-style).
pub struct CompressorNode {
    sample_rate: f32,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    knee_db: f32,
    makeup_db: f32,
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl CompressorNode {
    pub fn new(
        sample_rate: u32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        knee_db: f32,
        makeup_db: f32,
    ) -> Self {
        let sr = sample_rate as f32;
        Self {
            sample_rate: sr,
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            envelope: 0.0,
            attack_coeff: (-1000.0 / (attack_ms * sr)).exp(),
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

impl Node for CompressorNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let threshold = params.get("threshold").copied().unwrap_or(self.threshold_db);
        let ratio = params.get("ratio").copied().unwrap_or(self.ratio);
        let knee = params.get("knee").copied().unwrap_or(self.knee_db);
        let makeup = params.get("makeup").copied().unwrap_or(self.makeup_db);
        let makeup_lin = Self::db_to_linear(makeup);

        for chunk in input.chunks_exact(2) {
            let l = chunk[0].abs();
            let r = chunk[1].abs();
            let peak = l.max(r);
            let peak_db = Self::linear_to_db(peak);

            let over_db = peak_db - threshold;

            let gain_reduction_db = if over_db < -knee / 2.0 {
                0.0
            } else if over_db > knee / 2.0 {
                over_db * (1.0 - 1.0 / ratio.max(1.0))
            } else {
                let t = (over_db + knee / 2.0) / knee;
                let smooth = t * t / 2.0;
                over_db * (1.0 - 1.0 / ratio.max(1.0)) * smooth
            };

            let target_env = Self::db_to_linear(-gain_reduction_db);
            if target_env < self.envelope {
                self.envelope = self.attack_coeff * (self.envelope - target_env) + target_env;
            } else {
                self.envelope = self.release_coeff * (self.envelope - target_env) + target_env;
            }

            let g = self.envelope * makeup_lin;
            output[0] = chunk[0] * g;
            output[1] = chunk[1] * g;
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "compressor"
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}
