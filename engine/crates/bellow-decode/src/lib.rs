//! bellow-decode — audio file decoder using Symphonia
//!
//! Supports mp3, ogg/vorbis, flac, wav, aac, m4a, opus, alac, aiff.
//! Decodes to planar f32 PCM frames suitable for the real-time audio graph.

use bellow_rt::SampleClock;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub mod error;
pub use error::DecodeError;

/// Decoded audio buffer + metadata.
pub struct DecodedAudio {
    /// Interleaved f32 samples (channels × frames).
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_samples: u64,
}

/// Decode an audio file to f32 PCM.
pub fn decode_file<P: AsRef<Path>>(path: P) -> Result<DecodedAudio, DecodeError> {
    let file = File::open(path.as_ref())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| DecodeError::Probe(e.to_string()))?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(DecodeError::NoAudioTrack)?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
    let channels = track.codec_params.channels.unwrap_or_default().count() as u16;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    let track_id = track.id;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut samples: Vec<f32> = Vec::new();
    let mut total_samples: u64 = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(e) => return Err(DecodeError::Decode(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| DecodeError::Decode(e.to_string()))?;

        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::new(duration, spec));
        }

        if let Some(ref mut buf) = sample_buf {
            buf.copy_from_planar_ref(decoded);
            samples.extend_from_slice(buf.samples());
            total_samples += buf.samples().len() as u64;
        }
    }

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
        duration_samples: total_samples / channels as u64,
    })
}

/// A streaming decoder that yields blocks of f32 frames.
pub struct StreamingDecoder {
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    format: Box<dyn symphonia::core::formats::FormatReader>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    sample_buf: Option<SampleBuffer<f32>>,
}

impl StreamingDecoder {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DecodeError> {
        let file = File::open(path.as_ref())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let hint = Hint::new();
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| DecodeError::Probe(e.to_string()))?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(DecodeError::NoAudioTrack)?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
        let channels = track.codec_params.channels.unwrap_or_default().count() as u16;
        let track_id = track.id;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| DecodeError::Codec(e.to_string()))?;

        Ok(Self {
            decoder,
            format,
            track_id,
            sample_rate,
            channels,
            sample_buf: None,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Decode up to `max_frames` frames, returning interleaved f32 samples.
    /// Returns `None` when EOF is reached.
    pub fn next_block(&mut self, max_frames: usize) -> Result<Option<Vec<f32>>, DecodeError> {
        let mut out = Vec::with_capacity(max_frames * self.channels as usize);
        let mut frames_read: usize = 0;

        while frames_read < max_frames {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(_)) => break,
                Err(e) => return Err(DecodeError::Decode(e.to_string())),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = self
                .decoder
                .decode(&packet)
                .map_err(|e| DecodeError::Decode(e.to_string()))?;

            if self.sample_buf.is_none() {
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                self.sample_buf = Some(SampleBuffer::new(duration, spec));
            }

            if let Some(ref mut buf) = self.sample_buf {
                buf.copy_from_planar_ref(decoded);
                let chunk = buf.samples();
                let needed = ((max_frames - frames_read) * self.channels as usize).min(chunk.len());
                out.extend_from_slice(&chunk[..needed]);
                frames_read += needed / self.channels as usize;
            }
        }

        if out.is_empty() {
            Ok(None)
        } else {
            Ok(Some(out))
        }
    }
}
