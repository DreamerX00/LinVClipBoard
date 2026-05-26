use crate::error::PlatformError;
use crate::traits::ServiceManager;

/// Windows service/autostart manager (stub — real impl in Phase 6).
pub struct WindowsServiceManager;

impl Default for WindowsServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsServiceManager {
    pub fn new() -> Self {
        Self
    }
}

impl ServiceManager for WindowsServiceManager {
    fn register_autostart(&self, _app_path: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows autostart not yet implemented (Phase 6)".to_string(),
        ))
    }

    fn unregister_autostart(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows autostart not yet implemented (Phase 6)".to_string(),
        ))
    }

    fn is_autostart_enabled(&self) -> Result<bool, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows autostart not yet implemented (Phase 6)".to_string(),
        ))
    }
}
