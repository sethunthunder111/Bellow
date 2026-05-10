use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Engine already initialized")]
    AlreadyInitialized,
    #[error("Engine not initialized")]
    NotInitialized,
    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(u32),
    #[error("Invalid buffer size: {0}")]
    InvalidBufferSize(usize),
    #[error("Sound not found: {0}")]
    SoundNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type EngineResult<T> = Result<T, EngineError>;
