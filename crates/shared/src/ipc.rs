use crate::models::{IpcRequest, IpcResponse};
use std::path::Path;
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;

/// Length-prefix frame size (4 bytes, big-endian u32).
const FRAME_HEADER_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024; // 64MB max message

/// Send an IPC message over a stream with length-prefix framing.
#[cfg(unix)]
pub async fn send_message<T: serde::Serialize>(
    stream: &mut UnixStream,
    message: &T,
) -> std::io::Result<()> {
    let data = serde_json::to_vec(message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

/// Receive an IPC message from a stream with length-prefix framing.
#[cfg(unix)]
pub async fn recv_message<T: serde::de::DeserializeOwned>(
    stream: &mut UnixStream,
) -> std::io::Result<T> {
    let mut len_buf = [0u8; FRAME_HEADER_SIZE];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Message too large: {} bytes", len),
        ));
    }

    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;

    serde_json::from_slice(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Connect to the daemon and send a request, returning the response.
#[cfg(unix)]
pub async fn send_request(
    socket_path: &Path,
    request: &IpcRequest,
) -> std::io::Result<IpcResponse> {
    let mut stream = UnixStream::connect(socket_path).await?;
    send_message(&mut stream, request).await?;
    recv_message(&mut stream).await
}

/// Windows IPC using Named Pipes via the `platform` crate.
#[cfg(windows)]
pub async fn send_request(
    _socket_path: &Path,
    request: &IpcRequest,
) -> std::io::Result<IpcResponse> {
    use platform::IpcTransport;
    let transport = platform::WindowsIpcTransport::new_from_str("");
    let mut stream = transport
        .connect()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()))?;

    platform::ipc::send_message(&mut *stream, request)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    platform::ipc::recv_message(&mut *stream)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}
