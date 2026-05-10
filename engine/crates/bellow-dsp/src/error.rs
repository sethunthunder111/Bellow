use thiserror::Error;

#[derive(Debug, Error)]
pub enum DspError {
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
    #[error("Unsupported sample rate: {0}")]
    UnsupportedSampleRate(u32),
    #[error("Node not found")]
    NodeNotFound,
}
