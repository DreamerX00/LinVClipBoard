use crate::error::PlatformError;
use crate::traits::{IpcListener, IpcStream, IpcTransport as IpcTransportTrait};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener as TokioUnixListener, UnixStream as TokioUnixStream};

/// Unix domain socket IPC transport.
pub struct UnixIpcTransport {
    path: PathBuf,
}

impl UnixIpcTransport {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn new_from_str(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
        }
    }
}

#[async_trait::async_trait]
impl IpcTransportTrait for UnixIpcTransport {
    async fn connect(&self) -> Result<Box<dyn IpcStream>, PlatformError> {
        let stream = TokioUnixStream::connect(&self.path).await?;
        Ok(Box::new(UnixStream(stream)))
    }

    async fn bind(&self) -> Result<Box<dyn IpcListener>, PlatformError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        let listener = TokioUnixListener::bind(&self.path)?;
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o700))?;
        Ok(Box::new(UnixListener(listener, self.path.clone())))
    }

    fn path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

pub struct UnixStream(TokioUnixStream);

#[async_trait::async_trait]
impl IpcStream for UnixStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError> {
        self.0.read_exact(buf).await.map_err(PlatformError::from)
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError> {
        self.0.write_all(buf).await.map_err(PlatformError::from)
    }

    async fn flush(&mut self) -> Result<(), PlatformError> {
        self.0.flush().await.map_err(PlatformError::from)
    }
}

pub struct UnixListener(TokioUnixListener, PathBuf);

#[async_trait::async_trait]
impl IpcListener for UnixListener {
    async fn accept(&mut self) -> Result<(Box<dyn IpcStream>, String), PlatformError> {
        let (stream, addr) = self.0.accept().await?;
        let addr_str = addr
            .as_pathname()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((Box::new(UnixStream(stream)), addr_str))
    }
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.1);
    }
}
