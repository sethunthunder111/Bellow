//! bellow-resample — high-quality async sample-rate conversion via rubato.
//!
//! Uses windowed-sinc interpolation. Supports any input/output rate pair.

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResampleError {
    #[error("Rubato error: {0}")]
    Rubato(#[from] rubato::ResampleError),
    #[error("Rubato construction error: {0}")]
    Construction(#[from] rubato::ResamplerConstructionError),
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
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
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

/// Offline (one-shot) sample-rate conversion for an entire interleaved buffer.
///
/// Handles any input length — uses `SincFixedIn` internally with a fixed chunk
/// size, padding the last partial chunk with zeros and calling `process_partial`.
///
/// Returns interleaved output samples at `to_rate`.
pub fn resample_offline(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, ResampleError> {
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }
    let ch = channels as usize;
    if ch == 0 {
        return Err(ResampleError::InvalidChannels(channels));
    }

    let chunk_size: usize = 1024;
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        to_rate as f64 / from_rate as f64,
        1.0,
        params,
        chunk_size,
        ch,
    )?;

    let total_frames = samples.len() / ch;
    let mut out_interleaved: Vec<f32> = Vec::with_capacity(
        ((total_frames as f64) * (to_rate as f64) / (from_rate as f64)) as usize * ch
            + chunk_size * ch,
    );

    // Reusable planar buffers
    let mut wave_in: Vec<Vec<f32>> = vec![vec![0.0f32; chunk_size]; ch];
    let mut frame_pos = 0usize;

    while frame_pos < total_frames {
        let remaining = total_frames - frame_pos;
        let take = remaining.min(chunk_size);

        // Fill planar input
        for c in 0..ch {
            for f in 0..take {
                wave_in[c][f] = samples[(frame_pos + f) * ch + c];
            }
            // Zero-pad if last chunk
            for f in take..chunk_size {
                wave_in[c][f] = 0.0;
            }
        }

        let wave_out = if take == chunk_size {
            resampler.process(&wave_in, None)?
        } else {
            // Last partial chunk
            resampler.process_partial(Some(&wave_in), None)?
        };

        let out_frames = wave_out[0].len();
        for f in 0..out_frames {
            for c in 0..ch {
                out_interleaved.push(wave_out[c][f]);
            }
        }

        frame_pos += take;
    }

    // Drain any remaining frames in the resampler buffer
    let none_in: Option<&[Vec<f32>]> = None;
    if let Ok(wave_out) = resampler.process_partial(none_in, None) {
        let out_frames = wave_out.first().map(|v| v.len()).unwrap_or(0);
        for f in 0..out_frames {
            for c in 0..ch {
                out_interleaved.push(wave_out[c][f]);
            }
        }
    }

    Ok(out_interleaved)
}

/// Simple linear-interpolation offline resample.
///
/// Much simpler than `SincFixedIn` — no chunk boundaries, no state,
/// no partial-frame flushing. Quality is good enough for one-shot
/// file conversion (full song loads). For real-time resampling use
/// `SamplerateConverter` instead.
pub fn resample_linear(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, ResampleError> {
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }
    let ch = channels as usize;
    if ch == 0 {
        return Err(ResampleError::InvalidChannels(channels));
    }

    let in_frames = samples.len() / ch;
    let ratio = from_rate as f64 / to_rate as f64;
    let out_frames = ((in_frames as f64) / ratio).ceil() as usize;

    let mut out = vec![0.0f32; out_frames * ch];

    for f in 0..out_frames {
        let in_pos = f as f64 * ratio;
        let i0 = in_pos as usize;
        let frac = (in_pos - i0 as f64) as f32;
        let i1 = (i0 + 1).min(in_frames.saturating_sub(1));

        for c in 0..ch {
            let s0 = samples[i0 * ch + c];
            let s1 = samples[i1 * ch + c];
            out[f * ch + c] = s0 + (s1 - s0) * frac;
        }
    }

    Ok(out)
}
