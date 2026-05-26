use crate::error::PlatformError;
use crate::traits::ClipboardMonitor;

/// Windows clipboard monitor (stub — real impl in Phase 2).
pub struct WindowsClipboardMonitor;

impl Default for WindowsClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsClipboardMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl ClipboardMonitor for WindowsClipboardMonitor {
    fn wait_for_change(&mut self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard monitor not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn shutdown(&mut self) {}
}
