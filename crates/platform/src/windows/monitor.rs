use crate::error::PlatformError;
use crate::traits::ClipboardMonitor;
use clipboard_win::monitor::Monitor;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

pub struct WindowsClipboardMonitor {
    monitor: Option<Monitor>,
    shutdown_tx: Option<Sender<()>>,
    _thread: Option<JoinHandle<()>>,
}

impl Default for WindowsClipboardMonitor {
    fn default() -> Self {
        Self::new().expect("WindowsClipboardMonitor::default() failed")
    }
}

impl WindowsClipboardMonitor {
    pub fn new() -> Result<Self, PlatformError> {
        let monitor = Monitor::new()
            .map_err(|e| PlatformError::Clipboard(format!("create monitor: {}", e)))?;
        Ok(Self {
            monitor: Some(monitor),
            shutdown_tx: None,
            _thread: None,
        })
    }
}

impl ClipboardMonitor for WindowsClipboardMonitor {
    fn wait_for_change(&mut self) -> Result<(), PlatformError> {
        let monitor = self
            .monitor
            .as_mut()
            .ok_or_else(|| PlatformError::Clipboard("monitor not initialized".to_string()))?;

        loop {
            match monitor.recv() {
                Ok(true) => return Ok(()),
                Ok(false) => return Err(PlatformError::Clipboard("monitor shut down".to_string())),
                Err(e) => {
                    tracing::warn!("Clipboard monitor recv error: {}", e);
                    continue;
                }
            }
        }
    }

    fn shutdown(&mut self) {
        self.monitor = None;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Public helper: create a clipboard monitor with a channel for shutdown signalling.
pub fn create_clipboard_monitor() -> Result<(WindowsClipboardMonitor, Receiver<()>), PlatformError>
{
    let monitor =
        Monitor::new().map_err(|e| PlatformError::Clipboard(format!("create monitor: {}", e)))?;
    let (tx, rx) = channel();
    Ok((
        WindowsClipboardMonitor {
            monitor: Some(monitor),
            shutdown_tx: Some(tx),
            _thread: None,
        },
        rx,
    ))
}
