use crate::node::Node;
use crate::param::ParamMap;
use std::f32::consts::PI;

/// Biquad IIR filter coefficients and state.
#[derive(Debug, Clone)]
struct Biquad {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    z1: f32, z2: f32,
}

impl Biquad {
    fn process_sample(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.b1 * self.z1 + self.b2 * self.z2
            - self.a1 * self.z1 - self.a2 * self.z2;
        self.z2 = self.z1;
        self.z1 = out;
        out
    }

    /// Peak / bell EQ.
    fn peak_eq(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            z1: 0.0, z2: 0.0,
        }
    }

    /// Low-pass.
    fn lowpass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            z1: 0.0, z2: 0.0,
        }
    }

    /// High-pass.
    fn highpass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            z1: 0.0, z2: 0.0,
        }
    }

    /// Low-shelf.
    fn lowshelf(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let sqrt_2a = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + sqrt_2a);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - sqrt_2a);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + sqrt_2a;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - sqrt_2a;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            z1: 0.0, z2: 0.0,
        }
    }

    /// High-shelf.
    fn highshelf(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let sqrt_2a = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + sqrt_2a);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - sqrt_2a);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + sqrt_2a;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - sqrt_2a;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            z1: 0.0, z2: 0.0,
        }
    }
}

/// A single EQ band definition.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub kind: EqBandKind,
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqBandKind {
    Peak,
    LowPass,
    HighPass,
    LowShelf,
    HighShelf,
    Notch,
    BandPass,
}

/// Parametric EQ with up to 32 biquad bands.
pub struct ParametricEqNode {
    sample_rate: u32,
    bands: Vec<EqBand>,
    filters: Vec<Biquad>,
}

impl ParametricEqNode {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            bands: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn set_bands(&mut self, bands: Vec<EqBand>) {
        self.filters.clear();
        for band in &bands {
            let f = match band.kind {
                EqBandKind::Peak => Biquad::peak_eq(self.sample_rate as f32, band.freq, band.gain_db, band.q),
                EqBandKind::LowPass => Biquad::lowpass(self.sample_rate as f32, band.freq, band.q),
                EqBandKind::HighPass => Biquad::highpass(self.sample_rate as f32, band.freq, band.q),
                EqBandKind::LowShelf => Biquad::lowshelf(self.sample_rate as f32, band.freq, band.gain_db, band.q),
                EqBandKind::HighShelf => Biquad::highshelf(self.sample_rate as f32, band.freq, band.gain_db, band.q),
                EqBandKind::Notch => {
                    // Approximate notch as very narrow peak with -inf gain
                    Biquad::peak_eq(self.sample_rate as f32, band.freq, -60.0, band.q.max(10.0))
                }
                EqBandKind::BandPass => {
                    // Use LP + HP combo (simplified: just peak with 0 dB gain)
                    Biquad::peak_eq(self.sample_rate as f32, band.freq, 0.0, band.q)
                }
            };
            self.filters.push(f);
        }
        self.bands = bands;
    }
}

impl Node for ParametricEqNode {
    fn process(&mut self, input: &[f32], output: &mut [f32], _params: &ParamMap) {
        if self.filters.is_empty() {
            output.copy_from_slice(input);
            return;
        }

        // Process per-channel (stereo assumed)
        let channels = 2usize;
        let frames = input.len() / channels;

        for ch in 0..channels {
            for f in 0..frames {
                let idx = f * channels + ch;
                let mut s = input[idx];
                for filter in self.filters.iter_mut() {
                    s = filter.process_sample(s);
                }
                output[idx] = s;
            }
        }
    }

    fn channels(&self) -> u16 {
        2
    }

    fn name(&self) -> &str {
        "parametric_eq"
    }

    fn reset(&mut self) {
        for f in self.filters.iter_mut() {
            f.z1 = 0.0;
            f.z2 = 0.0;
        }
    }
}
