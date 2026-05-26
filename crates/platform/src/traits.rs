use crate::error::PlatformError;

/// Platform-specific IPC transport (Unix sockets vs Named Pipes).
#[async_trait::async_trait]
pub trait IpcTransport: Send + Sync {
    async fn connect(&self) -> Result<Box<dyn IpcStream>, PlatformError>;
    async fn bind(&self) -> Result<Box<dyn IpcListener>, PlatformError>;
    fn path(&self) -> String;
}

#[async_trait::async_trait]
pub trait IpcStream: Send {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError>;
    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError>;
    async fn flush(&mut self) -> Result<(), PlatformError>;
}

#[async_trait::async_trait]
pub trait IpcListener: Send {
    async fn accept(&mut self) -> Result<(Box<dyn IpcStream>, String), PlatformError>;
}

/// Clipboard get/set operations.
pub trait ClipboardProvider: Send + Sync {
    fn get_text(&self) -> Result<Option<String>, PlatformError>;
    fn set_text(&self, text: &str) -> Result<(), PlatformError>;
    fn get_image(&self) -> Result<Option<(Vec<u8>, usize, usize)>, PlatformError>;
    fn set_image(&self, data: &[u8], width: usize, height: usize) -> Result<(), PlatformError>;
    fn get_html(&self) -> Result<Option<String>, PlatformError>;
    fn get_files(&self) -> Result<Option<Vec<String>>, PlatformError>;
}

/// Blocking clipboard change monitor.
pub trait ClipboardMonitor: Send {
    fn wait_for_change(&mut self) -> Result<(), PlatformError>;
    fn shutdown(&mut self);
}

/// Keyboard/mouse input simulation for pasting.
pub trait InputSimulator: Send + Sync {
    fn paste_text(&self, text: &str) -> Result<(), PlatformError>;
    fn type_text(&self, text: &str) -> Result<(), PlatformError>;
    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError>;
}

/// Autostart service management.
pub trait ServiceManager: Send + Sync {
    fn register_autostart(&self, app_path: &str) -> Result<(), PlatformError>;
    fn unregister_autostart(&self) -> Result<(), PlatformError>;
    fn is_autostart_enabled(&self) -> Result<bool, PlatformError>;
}
