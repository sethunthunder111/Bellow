//! bellow-io — audio device I/O using CPAL
//!
//! Wraps CPAL to provide:
//! - Device enumeration with full capabilities
//! - Output stream management (shared + exclusive on Windows via WASAPI)
//! - Input stream management
//! - Hot-plug detection where supported
//!
//! Currently supports WASAPI shared, Core Audio, ALSA, JACK, PipeWire, PulseAudio.
//! ASIO is behind a feature flag.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub mod devices;
pub mod error;
pub use devices::*;
pub use error::IoError;

/// A running output stream. Drop to stop.
pub struct OutputStream {
    _stream: cpal::Stream,
    running: Arc<AtomicBool>,
    config: StreamConfig,
}

impl OutputStream {
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }
    pub fn channels(&self) -> u16 {
        self.config.channels
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Build and start an output stream on the default device.
/// The `render_callback` is called with an interleaved f32 buffer to fill.
///
/// `requested_sample_rate` is a hint. The actual rate used is the device's
/// preferred rate if `requested_sample_rate == 0`, otherwise the requested
/// rate (must be supported by the device).
///
/// `requested_buffer_size == 0` means "device default" (recommended on
/// Windows WASAPI shared mode for stable playback).
pub fn build_output_stream<F>(
    requested_sample_rate: u32,
    requested_channels: u16,
    requested_buffer_size: u32,
    mut render_callback: F,
) -> Result<OutputStream, IoError>
where
    F: FnMut(&mut [f32]) + Send + 'static,
{
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(IoError::NoDefaultOutputDevice)?;

    let default_cfg = device
        .default_output_config()
        .map_err(|e| IoError::DeviceConfig(e.to_string()))?;

    let mut config: StreamConfig = default_cfg.config();

    // Honor requested rate only if non-zero, else use device default
    if requested_sample_rate != 0 {
        config.sample_rate = SampleRate(requested_sample_rate);
    }
    if requested_channels != 0 {
        config.channels = requested_channels;
    }
    if requested_buffer_size != 0 {
        config.buffer_size = BufferSize::Fixed(requested_buffer_size);
    } else {
        config.buffer_size = BufferSize::Default;
    }

    let running = Arc::new(AtomicBool::new(true));

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                render_callback(data);
            },
            move |err| eprintln!("CPAL output error: {}", err),
            None,
        )
        .map_err(|e| IoError::StreamBuild(e.to_string()))?;

    stream
        .play()
        .map_err(|e| IoError::StreamPlay(e.to_string()))?;

    Ok(OutputStream {
        _stream: stream,
        running,
        config,
    })
}

/// Sample-accurate playback queue for a single sound source.
/// Pushes interleaved f32 frames into a ringbuffer consumed by the audio callback.
pub struct PlaybackQueue {
    tx: Sender<Vec<f32>>,
    rx: Arc<Mutex<Receiver<Vec<f32>>>>,
    channels: u16,
    sample_rate: u32,
    /// Remaining samples in the current chunk (interleaved).
    current_chunk: Vec<f32>,
    chunk_offset: usize,
    pub finished: Arc<AtomicBool>,
}

impl PlaybackQueue {
    pub fn new(channels: u16, sample_rate: u32, _capacity_frames: usize) -> Self {
        let (tx, rx) = bounded::<Vec<f32>>(16);
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            channels,
            sample_rate,
            current_chunk: Vec::new(),
            chunk_offset: 0,
            finished: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn push_frames(&self, frames: Vec<f32>) -> Result<(), IoError> {
        self.tx.try_send(frames).map_err(|_| IoError::QueueFull)
    }

    /// Fill an output buffer from the queue. Call this inside the CPAL callback.
    pub fn fill_buffer(&mut self, out: &mut [f32]) {
        let mut written = 0usize;
        let _frame_size = self.channels as usize;

        while written < out.len() {
            if self.chunk_offset >= self.current_chunk.len() {
                match self.rx.lock().unwrap().try_recv() {
                    Ok(chunk) => {
                        self.current_chunk = chunk;
                        self.chunk_offset = 0;
                    }
                    Err(_) => {
                        // Underrun: zero-fill remainder
                        out[written..].fill(0.0);
                        break;
                    }
                }
            }

            let available = self.current_chunk.len() - self.chunk_offset;
            let to_write = (out.len() - written).min(available);
            out[written..written + to_write].copy_from_slice(
                &self.current_chunk[self.chunk_offset..self.chunk_offset + to_write],
            );
            self.chunk_offset += to_write;
            written += to_write;
        }

        if written < out.len() {
            out[written..].fill(0.0);
        }
    }

    pub fn finish(&self) {
        self.finished.store(true, Ordering::Relaxed);
    }
}
