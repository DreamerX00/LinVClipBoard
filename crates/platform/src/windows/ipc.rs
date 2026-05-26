use crate::error::PlatformError;
use crate::traits::{IpcListener, IpcStream, IpcTransport as IpcTransportTrait};
use std::path::PathBuf;

/// Named pipe IPC transport for Windows (stub — real impl in Phase 3).
pub struct WindowsIpcTransport {
    pipe_name: String,
}

impl WindowsIpcTransport {
    pub fn new(_path: PathBuf) -> Self {
        Self {
            pipe_name: r"\\.\pipe\LinVClipBoard".to_string(),
        }
    }

    pub fn new_from_str(_path: &str) -> Self {
        Self {
            pipe_name: r"\\.\pipe\LinVClipBoard".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl IpcTransportTrait for WindowsIpcTransport {
    async fn connect(&self) -> Result<Box<dyn IpcStream>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows named pipe IPC not yet implemented (Phase 3)".to_string(),
        ))
    }

    async fn bind(&self) -> Result<Box<dyn IpcListener>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows named pipe IPC not yet implemented (Phase 3)".to_string(),
        ))
    }

    fn path(&self) -> String {
        self.pipe_name.clone()
    }
}

pub struct NamedPipeStream;

#[async_trait::async_trait]
impl IpcStream for NamedPipeStream {
    async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, PlatformError> {
        Err(PlatformError::Unsupported("stub".to_string()))
    }

    async fn write_all(&mut self, _buf: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("stub".to_string()))
    }

    async fn flush(&mut self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("stub".to_string()))
    }
}

pub struct NamedPipeListener;

#[async_trait::async_trait]
impl IpcListener for NamedPipeListener {
    async fn accept(&mut self) -> Result<(Box<dyn IpcStream>, String), PlatformError> {
        Err(PlatformError::Unsupported("stub".to_string()))
    }
}
