# Phase 3: Windows Named Pipe IPC Transport

**Version**: v3.0.0-ipc  
**Effort**: 2-3 days  
**Dependencies**: Phase 1 (PAL IpcTransport trait), Phase 2 (clipd running)

---

## Objective

Implement the Named Pipe IPC transport for Windows using Tokio's async named pipe API, replacing the Unix domain socket implementation used on Linux. Both clipd (server) and clients (clipctl, Tauri GUI) will use the same transport abstraction.

---

## Tasks

### 3.1 Implement Windows IpcTransport

**`crates/platform/src/windows/ipc.rs`:**

```rust
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;

const PIPE_NAME: &str = r"\\.\pipe\LinVClipBoard";

pub struct WindowsIpcTransport;

impl IpcTransport for WindowsIpcTransport {
    fn path(&self) -> String {
        PIPE_NAME.to_string()
    }
}

// --- Client ---
pub struct WindowsIpcStream {
    client: NamedPipeClient,
}

impl IpcStream for WindowsIpcStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError> {
        self.client.read(buf).await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError> {
        self.client.write_all(buf).await
            .map_err(|e| PlatformError::Ipc(e.to_string()))
    }
}

// --- Server ---
pub struct WindowsIpcListener {
    server: Option<NamedPipeServer>,
}

impl IpcListener for WindowsIpcListener {
    async fn accept(&mut self) -> Result<Box<dyn IpcStream>, PlatformError> {
        let server = self.server.as_mut().unwrap();
        server.connect().await
            .map_err(|e| PlatformError::Ipc(e.to_string()))?;

        // Create next server instance for subsequent connections
        let new_server = ServerOptions::new()
            .create(PIPE_NAME)
            .map_err(|e| PlatformError::Ipc(e.to_string()))?;

        let old_server = self.server.replace(new_server).unwrap();

        // The old_server is now connected — we need to handle it
        // For simplicity, return it wrapped in WindowsIpcStream
        todo!("Wrap connected NamedPipeServer as IpcStream")
    }
}
```

### 3.2 Implement IPC Client Connection with Retry

On Windows, the pipe may be busy if another client is connecting. Implement retry:

```rust
pub async fn connect_to_pipe() -> Result<NamedPipeClient, PlatformError> {
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

    for attempt in 1..=10 {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(client) => return Ok(client),
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                tokio::time::sleep(Duration::from_millis(50 * attempt)).await;
                continue;
            }
            Err(e) => return Err(PlatformError::Ipc(e.to_string())),
        }
    }
    Err(PlatformError::Ipc("Pipe busy after 10 retries".into()))
}
```

### 3.3 Implement Multi-Instance Named Pipe Server

Named pipes support multiple instances. The pattern:

1. Create first pipe instance with `ServerOptions::new().create(PIPE_NAME)`
2. Wait for connection with `server.connect()`
3. Create next instance immediately (so new clients can connect)
4. Spawn task to handle the connected client
5. Loop back to step 2

```rust
pub async fn serve_forever<F, Fut>(handler: F) -> Result<(), PlatformError>
where
    F: Fn(Box<dyn IpcStream>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), PlatformError>> + Send,
{
    // Create first instance
    let mut server = ServerOptions::new()
        .max_instances(10) // Allow up to 10 concurrent connections
        .create(PIPE_NAME)
        .map_err(|e| PlatformError::Ipc(e.to_string()))?;

    loop {
        // Wait for client connection
        server.connect().await
            .map_err(|e| PlatformError::Ipc(e.to_string()))?;

        // Create next instance for future connections
        let next_server = ServerOptions::new()
            .max_instances(10)
            .create(PIPE_NAME);

        // Spawn handler for this connection
        let stream = WindowsIpcStream { server };
        tokio::spawn(async move {
            if let Err(e) = handler(Box::new(stream)).await {
                tracing::error!("IPC handler error: {e}");
            }
        });

        // Use next instance for loop continuation
        match next_server {
            Ok(s) => server = s,
            Err(e) => {
                tracing::error!("Failed to create next pipe instance: {e}");
                break;
            }
        }
    }

    Ok(())
}
```

### 3.4 Update Shared Library IPC

**`crates/shared/src/ipc.rs`:**

Replace direct `UnixStream` usage with `IpcTransport` trait:

```rust
use platform::ipc::{IpcTransport, IpcStream, connect_to_pipe};

pub async fn connect() -> Result<Box<dyn IpcStream>, Error> {
    #[cfg(unix)]
    {
        // Existing Unix socket connection
    }
    #[cfg(windows)]
    {
        connect_to_pipe().await.map_err(|e| Error::Ipc(e.to_string()))
    }
}
```

### 3.5 Update IPC Protocol Serialization

The existing IPC protocol in `crates/shared/src/models.rs` uses JSON over the transport. This is platform-agnostic — no changes needed.

- [ ] Verify IPC message format is consistent across platforms
- [ ] Test IPC serialization roundtrip on Windows: `cargo test -p shared`

### 3.6 Security: Named Pipe ACL

- [ ] Set named pipe DACL to allow only the current user
- [ ] Use `PipeSecurityAttributes` or explicit ACL to prevent other user processes from connecting
- [ ] Mark with `PIPE_REJECT_REMOTE_CLIENTS`

### 3.7 Edge Case Handling

- [ ] Pipe name collision (another app using same name)
- [ ] Client disconnect mid-read (graceful error handling)
- [ ] Server restart without client timeout (implement connection retry)
- [ ] Long-running reads (set read timeout on pipe)
- [ ] Message mode vs byte mode (`PIPE_TYPE_MESSAGE` vs `PIPE_TYPE_BYTE`) — use byte mode for JSON framing

---

## Deliverables

1. Windows Named Pipe server (multi-instance) in clipd
2. Windows Named Pipe client in shared library
3. Retry logic for pipe-busy scenarios
4. Proper pipe cleanup on shutdown

---

## Acceptance Criteria

- [ ] clipd creates `\\.\pipe\LinVClipBoard`
- [ ] Multiple clients can connect concurrently
- [ ] IPC messages (list, search, pin, delete) work identically to Linux
- [ ] Client reconnects automatically if clipd restarts
- [ ] Remote pipe connections are rejected
- [ ] All `cargo test -p shared` pass on Windows
- [ ] Phase 3 branch committed
