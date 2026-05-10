use crate::node::Node;
use crate::param::ParamMap;

/// Equal-power stereo panner.
pub struct PannerNode {
    pan: f32, // -1 (left) to +1 (right)
}

impl PannerNode {
    pub fn new(pan: f32) -> Self {
        Self { pan: pan.clamp(-1.0, 1.0) }
    }

    fn coefficients(pan: f32) -> (f32, f32) {
        // Equal-power pan law: sin/cos on quarter circle
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        (angle.cos(), angle.sin())
    }
}

impl Node for PannerNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], params: &ParamMap) {
        let pan = params
            .get("pan")
            .copied()
            .unwrap_or(self.pan)
            .clamp(-1.0, 1.0);
        let (left_gain, right_gain) = Self::coefficients(pan);

        for chunk in input.chunks_exact(2) {
            let l = chunk[0];
            let r = chunk[1];
            // Mono-to-stereo pan: apply gains to summed mono source
            let m = (l + r) * 0.5;
            output[0] = m * left_gain;
            output[1] = m * right_gain;
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "panner"
    }
}
