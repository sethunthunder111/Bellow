use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("No default output device available")]
    NoDefaultOutputDevice,
    #[error("Device config error: {0}")]
    DeviceConfig(String),
    #[error("Stream build error: {0}")]
    StreamBuild(String),
    #[error("Stream play error: {0}")]
    StreamPlay(String),
    #[error("Playback queue full")]
    QueueFull,
    #[error("CPAL error: {0}")]
    Cpal(#[from] cpal::BuildStreamError),
}
