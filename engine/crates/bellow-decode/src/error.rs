use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Probe failed: {0}")]
    Probe(String),
    #[error("Codec error: {0}")]
    Codec(String),
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("No audio track found")]
    NoAudioTrack,
}
