use crate::error::PlatformError;
use crate::traits::ClipboardMonitor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct UnixClipboardMonitor {
    shutdown_flag: Arc<AtomicBool>,
    last_checksum: Option<String>,
}

impl Default for UnixClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnixClipboardMonitor {
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            last_checksum: None,
        }
    }

    fn compute_checksum(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

impl ClipboardMonitor for UnixClipboardMonitor {
    fn wait_for_change(&mut self) -> Result<(), PlatformError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| PlatformError::Clipboard(format!("monitor init: {}", e)))?;

        loop {
            if self.shutdown_flag.load(Ordering::Relaxed) {
                return Err(PlatformError::Clipboard("monitor shut down".to_string()));
            }

            if let Ok(text) = clipboard.get_text() {
                let checksum = Self::compute_checksum(text.as_bytes());
                if self.last_checksum.as_ref() != Some(&checksum) {
                    self.last_checksum = Some(checksum);
                    return Ok(());
                }
            }

            if let Ok(img) = clipboard.get_image() {
                let checksum = Self::compute_checksum(&img.bytes);
                if self.last_checksum.as_ref() != Some(&checksum) {
                    self.last_checksum = Some(checksum);
                    return Ok(());
                }
            }

            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn shutdown(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }
}
