//! bellow-resample — high-quality async sample-rate conversion via rubato.
//!
//! Uses windowed-sinc interpolation. Supports any input/output rate pair.

use rubato::{
    InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResampleError {
    #[error("Rubato error: {0}")]
    Rubato(#[from] rubato::ResampleError),
    #[error("Invalid channel count: {0}")]
    InvalidChannels(u16),
}

/// A resampler that converts between any two sample rates.
pub struct SamplerateConverter {
    resampler: SincFixedIn<f32>,
    channels: usize,
}

impl SamplerateConverter {
    /// Create a converter from `from_rate` to `to_rate`.
    /// `chunk_size` = input frames per process call.
    pub fn new(
        from_rate: u32,
        to_rate: u32,
        channels: u16,
        chunk_size: usize,
    ) -> Result<Self, ResampleError> {
        let ch = channels as usize;
        let params = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        let resampler = SincFixedIn::new(
            to_rate as f64 / from_rate as f64,
            1.0,
            params,
            chunk_size,
            ch,
        )?;

        Ok(Self {
            resampler,
            channels: ch,
        })
    }

    /// Process a block of interleaved f32 samples.
    /// Returns interleaved output samples.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResampleError> {
        // Convert interleaved to planar (rubato expects planar)
        let frames = input.len() / self.channels;
        let mut wave_in = vec![vec![0.0f32; frames]; self.channels];
        for (f, chunk) in input.chunks_exact(self.channels).enumerate() {
            for (c, &s) in chunk.iter().enumerate() {
                wave_in[c][f] = s;
            }
        }

        let wave_out = self.resampler.process(&wave_in, None)?;

        // Convert planar back to interleaved
        let out_frames = wave_out[0].len();
        let mut interleaved = vec![0.0f32; out_frames * self.channels];
        for c in 0..self.channels {
            for f in 0..out_frames {
                interleaved[f * self.channels + c] = wave_out[c][f];
            }
        }

        Ok(interleaved)
    }

    pub fn reset(&mut self) {
        self.resampler.reset();
    }
}
