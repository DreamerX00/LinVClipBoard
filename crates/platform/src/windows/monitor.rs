use crate::error::PlatformError;
use crate::traits::ClipboardMonitor;
use clipboard_win::monitor::{Monitor, Shutdown};
use std::sync::mpsc::{channel, sync_channel, Receiver};
use std::thread::JoinHandle;

/// Event-driven clipboard change monitor for Windows.
///
/// `clipboard_win::monitor::Monitor` is backed by a message-only window and is
/// therefore not `Send`: it must be created and polled on a single thread. To
/// satisfy the `ClipboardMonitor: Send` bound, the `Monitor` is owned by a
/// dedicated worker thread that forwards change notifications over a channel.
/// Only the `Shutdown` handle (which *is* `Send`, as it merely posts a window
/// message) crosses threads — exactly the pattern clipboard-win recommends.
pub struct WindowsClipboardMonitor {
    events: Receiver<()>,
    shutdown: Option<Shutdown>,
    _thread: Option<JoinHandle<()>>,
}

impl Default for WindowsClipboardMonitor {
    fn default() -> Self {
        Self::new().expect("WindowsClipboardMonitor::default() failed")
    }
}

impl WindowsClipboardMonitor {
    pub fn new() -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = sync_channel::<Result<Shutdown, String>>(1);
        let (event_tx, event_rx) = channel::<()>();

        let thread = std::thread::Builder::new()
            .name("clipboard-monitor".to_string())
            .spawn(move || {
                let mut monitor = match Monitor::new() {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                if ready_tx.send(Ok(monitor.shutdown_channel())).is_err() {
                    return;
                }
                loop {
                    match monitor.recv() {
                        Ok(true) => {
                            if event_tx.send(()).is_err() {
                                // Receiver dropped: nobody is listening any more.
                                break;
                            }
                        }
                        // `Shutdown` handle dropped → close message received.
                        Ok(false) => break,
                        Err(e) => {
                            tracing::warn!("Clipboard monitor recv error: {}", e);
                        }
                    }
                }
            })
            .map_err(|e| PlatformError::Clipboard(format!("spawn monitor thread: {}", e)))?;

        let shutdown = ready_rx
            .recv()
            .map_err(|_| PlatformError::Clipboard("monitor thread exited early".to_string()))?
            .map_err(|e| PlatformError::Clipboard(format!("create monitor: {}", e)))?;

        Ok(Self {
            events: event_rx,
            shutdown: Some(shutdown),
            _thread: Some(thread),
        })
    }
}

impl ClipboardMonitor for WindowsClipboardMonitor {
    fn wait_for_change(&mut self) -> Result<(), PlatformError> {
        self.events
            .recv()
            .map_err(|_| PlatformError::Clipboard("monitor shut down".to_string()))
    }

    fn shutdown(&mut self) {
        // Dropping the handle posts the close message that unblocks the
        // worker's `Monitor::recv`, which then exits its loop.
        self.shutdown = None;
    }
}

/// Public helper: create a clipboard monitor together with a receiver that is
/// notified once for every clipboard change.
pub fn create_clipboard_monitor() -> Result<(WindowsClipboardMonitor, Receiver<()>), PlatformError>
{
    let mut monitor = WindowsClipboardMonitor::new()?;
    let (tx, rx) = channel::<()>();
    let (proxy_tx, proxy_rx) = channel::<()>();
    let events = std::mem::replace(&mut monitor.events, proxy_rx);
    std::thread::Builder::new()
        .name("clipboard-monitor-fanout".to_string())
        .spawn(move || {
            while events.recv().is_ok() {
                let _ = tx.send(());
                if proxy_tx.send(()).is_err() {
                    break;
                }
            }
        })
        .map_err(|e| PlatformError::Clipboard(format!("spawn fanout thread: {}", e)))?;
    Ok((monitor, rx))
}
