use crate::error::PlatformError;
use crate::traits::{IpcListener, IpcStream, IpcTransport as IpcTransportTrait};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};

const PIPE_NAME: &str = r"\\.\pipe\LinVClipBoard";
const MAX_RETRIES: u32 = 10;
const RETRY_DELAY_MS: u64 = 50;

pub struct WindowsIpcTransport {
    pipe_name: String,
}

impl WindowsIpcTransport {
    pub fn new(_path: std::path::PathBuf) -> Self {
        Self {
            pipe_name: PIPE_NAME.to_string(),
        }
    }

    pub fn new_from_str(_path: &str) -> Self {
        Self {
            pipe_name: PIPE_NAME.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl IpcTransportTrait for WindowsIpcTransport {
    async fn connect(&self) -> Result<Box<dyn IpcStream>, PlatformError> {
        let client = connect_with_retry(&self.pipe_name).await?;
        Ok(Box::new(NamedPipeClientStream { client }))
    }

    async fn bind(&self) -> Result<Box<dyn IpcListener>, PlatformError> {
        let server = ServerOptions::new()
            .create(&self.pipe_name)
            .map_err(|e| PlatformError::Ipc(format!("Failed to create pipe: {}", e)))?;
        Ok(Box::new(NamedPipeListener {
            server: Some(server),
            pipe_name: self.pipe_name.clone(),
        }))
    }

    fn path(&self) -> String {
        self.pipe_name.clone()
    }
}

pub struct NamedPipeClientStream {
    client: NamedPipeClient,
}

#[async_trait::async_trait]
impl IpcStream for NamedPipeClientStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError> {
        self.client
            .read(buf)
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError> {
        self.client
            .write_all(buf)
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }

    async fn flush(&mut self) -> Result<(), PlatformError> {
        self.client
            .flush()
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }
}

struct NamedPipeServerStream {
    inner: NamedPipeServer,
}

#[async_trait::async_trait]
impl IpcStream for NamedPipeServerStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError> {
        self.inner
            .read(buf)
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError> {
        self.inner
            .write_all(buf)
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }

    async fn flush(&mut self) -> Result<(), PlatformError> {
        self.inner
            .flush()
            .await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }
}

pub struct NamedPipeListener {
    server: Option<NamedPipeServer>,
    pipe_name: String,
}

#[async_trait::async_trait]
impl IpcListener for NamedPipeListener {
    async fn accept(&mut self) -> Result<(Box<dyn IpcStream>, String), PlatformError> {
        let server = self
            .server
            .as_ref()
            .ok_or_else(|| PlatformError::Ipc("No pipe server instance".to_string()))?;
        server
            .connect()
            .await
            .map_err(|e| PlatformError::Ipc(format!("Pipe connect: {}", e)))?;

        let connected = self.server.take().unwrap();

        let next = ServerOptions::new()
            .create(&self.pipe_name)
            .map_err(|e| PlatformError::Ipc(format!("Create next pipe: {}", e)))?;
        self.server = Some(next);

        Ok((
            Box::new(NamedPipeServerStream { inner: connected }),
            String::new(),
        ))
    }
}

async fn connect_with_retry(pipe_name: &str) -> Result<NamedPipeClient, PlatformError> {
    let mut last_error = None;
    for attempt in 1..=MAX_RETRIES {
        match ClientOptions::new().open(pipe_name) {
            Ok(client) => return Ok(client),
            Err(e) => {
                let is_busy = e
                    .raw_os_error()
                    .map(|code| code as u32 == 231) // ERROR_PIPE_BUSY = 231
                    .unwrap_or(false);
                if !is_busy {
                    return Err(PlatformError::Ipc(format!("Pipe open: {}", e)));
                }
                last_error = Some(e);
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }
        }
    }
    Err(PlatformError::Ipc(format!(
        "Pipe busy after {} retries: {}",
        MAX_RETRIES,
        last_error.map_or_else(|| "unknown".to_string(), |e| e.to_string())
    )))
}
