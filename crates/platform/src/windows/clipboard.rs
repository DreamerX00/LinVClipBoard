use crate::error::PlatformError;
use crate::traits::ClipboardProvider;

/// Windows clipboard provider (stub — real impl in Phase 2).
pub struct WindowsClipboardProvider;

impl Default for WindowsClipboardProvider {
    fn default() -> Self {
        Self
    }
}

impl WindowsClipboardProvider {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

impl ClipboardProvider for WindowsClipboardProvider {
    fn get_text(&self) -> Result<Option<String>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn set_text(&self, _text: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn get_image(&self) -> Result<Option<(Vec<u8>, usize, usize)>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn set_image(&self, _data: &[u8], _width: usize, _height: usize) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn get_html(&self) -> Result<Option<String>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn get_files(&self) -> Result<Option<Vec<String>>, PlatformError> {
        Err(PlatformError::Unsupported(
            "Windows clipboard not yet implemented (Phase 2)".to_string(),
        ))
    }
}
