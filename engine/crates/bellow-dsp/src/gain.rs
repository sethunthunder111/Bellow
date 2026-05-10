use crate::node::Node;
use crate::param::ParamMap;

/// Simple gain node with per-channel volume.
pub struct GainNode {
    gain_db: f32,
    channels: u16,
}

impl GainNode {
    pub fn new(gain_db: f32, channels: u16) -> Self {
        Self { gain_db, channels }
    }

    fn db_to_linear(db: f32) -> f32 {
        10f32.powf(db / 20.0)
    }
}

impl Node for GainNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let gain = params
            .get("gain")
            .copied()
            .unwrap_or(self.gain_db);
        let linear = Self::db_to_linear(gain);
        for (i, v) in input.iter().enumerate() {
            output[i] = v * linear;
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn name(&self) -> &str {
        "gain"
    }
}
