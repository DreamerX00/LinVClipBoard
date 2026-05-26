use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("IPC error: {0}")]
    Ipc(String),
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    #[error("Input simulation error: {0}")]
    Input(String),
    #[error("Service error: {0}")]
    Service(String),
    #[error("Not supported on this platform: {0}")]
    Unsupported(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<String> for PlatformError {
    fn from(s: String) -> Self {
        PlatformError::Ipc(s)
    }
}
