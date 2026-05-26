use crate::error::PlatformError;
use crate::traits::InputSimulator;

/// Windows input simulator (stub — real impl in Phase 7).
pub struct WindowsInputSimulator;

impl Default for WindowsInputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsInputSimulator {
    pub fn new() -> Self {
        Self
    }
}

impl InputSimulator for WindowsInputSimulator {
    fn paste_text(&self, _text: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows input simulation not yet implemented (Phase 7)".to_string(),
        ))
    }

    fn type_text(&self, _text: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows input simulation not yet implemented (Phase 7)".to_string(),
        ))
    }

    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows input simulation not yet implemented (Phase 7)".to_string(),
        ))
    }
}
