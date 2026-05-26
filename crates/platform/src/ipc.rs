use crate::error::PlatformError;
use crate::traits::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

const FRAME_HEADER_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// Send a serializable message over an IPC stream with length-prefix framing.
pub async fn send_message<T: Serialize>(
    stream: &mut dyn IpcStream,
    message: &T,
) -> Result<(), PlatformError> {
    let data = serde_json::to_vec(message)
        .map_err(|e| PlatformError::Ipc(format!("serialization error: {}", e)))?;
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

/// Receive a deserializable message from an IPC stream with length-prefix framing.
pub async fn recv_message<T: DeserializeOwned>(
    stream: &mut dyn IpcStream,
) -> Result<T, PlatformError> {
    let mut len_buf = [0u8; FRAME_HEADER_SIZE];
    stream.read(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(PlatformError::Ipc(format!(
            "message too large: {} bytes",
            len
        )));
    }
    let mut data = vec![0u8; len];
    stream.read(&mut data).await?;
    serde_json::from_slice(&data)
        .map_err(|e| PlatformError::Ipc(format!("deserialization error: {}", e)))
}

/// Connect to the daemon via the given transport and send a request.
pub async fn send_request<R: DeserializeOwned>(
    transport: &dyn IpcTransport,
    request: &impl Serialize,
) -> Result<R, PlatformError> {
    let mut stream = transport.connect().await?;
    send_message(&mut *stream, request).await?;
    recv_message(&mut *stream).await
}
