use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResampleError {
    #[error("Rubato error: {0}")]
    Rubato(#[from] rubato::ResampleError),
    #[error("Invalid channel count: {0}")]
    InvalidChannels(u16),
}
