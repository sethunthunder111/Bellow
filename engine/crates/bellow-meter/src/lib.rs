//! bellow-meter — real-time audio meters
//!
//! Implements:
//! - Peak meter (per-sample max)
//! - RMS meter (sliding window)
//! - LUFS meter (ITU-R BS.1770-4: K-weighted, 400ms gating)
//! - True-peak meter (4× oversampled)
//! - Phase correlation (stereo)
//! - Spectrum analyzer bins

pub mod error;
pub mod peak;
pub mod rms;
pub mod lufs;
pub mod spectrum;

pub use error::MeterError;
pub use peak::PeakMeter;
pub use rms::RmsMeter;
pub use lufs::LufsMeter;
