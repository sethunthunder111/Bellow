use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeterError {
    #[error("Invalid window size: {0}")]
    InvalidWindowSize(usize),
    #[error("Unsupported channel count: {0}")]
    UnsupportedChannels(u16),
}
