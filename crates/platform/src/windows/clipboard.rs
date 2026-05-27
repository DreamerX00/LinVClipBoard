use crate::error::PlatformError;
use crate::traits::ClipboardProvider;
use clipboard_win::{formats, get_clipboard, set_clipboard, Clipboard};
use std::sync::Mutex;

const CLIPBOARD_RETRIES: u32 = 5;

pub struct WindowsClipboardProvider {
    _inner: Mutex<()>,
}

impl WindowsClipboardProvider {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            _inner: Mutex::new(()),
        })
    }
}

impl ClipboardProvider for WindowsClipboardProvider {
    fn get_text(&self) -> Result<Option<String>, PlatformError> {
        let _lock = self._inner.lock().unwrap();
        let _clip = Clipboard::new_attempts(CLIPBOARD_RETRIES)
            .map_err(|e| PlatformError::Clipboard(format!("open clipboard: {}", e)))?;

        if clipboard_win::is_format_avail(formats::CF_UNICODETEXT) {
            let text: String = get_clipboard(formats::Unicode)
                .map_err(|e| PlatformError::Clipboard(format!("get_text: {}", e)))?;
            if text.trim().is_empty() {
                return Ok(None);
            }
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn set_text(&self, text: &str) -> Result<(), PlatformError> {
        let _lock = self._inner.lock().unwrap();
        let _clip = Clipboard::new_attempts(CLIPBOARD_RETRIES)
            .map_err(|e| PlatformError::Clipboard(format!("open clipboard: {}", e)))?;

        set_clipboard(formats::Unicode, text)
            .map_err(|e| PlatformError::Clipboard(format!("set_text: {}", e)))
    }

    fn get_image(&self) -> Result<Option<(Vec<u8>, usize, usize)>, PlatformError> {
        // CF_DIB / CF_DIBV5 bitmap reading requires manual parsing.
        // For now, return None — image support will be added in a follow-up.
        Ok(None)
    }

    fn set_image(&self, _data: &[u8], _width: usize, _height: usize) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "image clipboard set not yet implemented on Windows".to_string(),
        ))
    }

    fn get_html(&self) -> Result<Option<String>, PlatformError> {
        let _lock = self._inner.lock().unwrap();
        let _clip = Clipboard::new_attempts(CLIPBOARD_RETRIES)
            .map_err(|e| PlatformError::Clipboard(format!("open clipboard: {}", e)))?;

        let html_format = clipboard_win::raw::register_format("HTML Format")
            .map_err(|e| PlatformError::Clipboard(format!("register HTML Format: {}", e)))?;

        if clipboard_win::is_format_avail(html_format) {
            let raw: String = get_clipboard(formats::RawData(html_format))
                .map_err(|e| PlatformError::Clipboard(format!("get_html: {}", e)))?;
            Ok(Some(extract_html_fragment(&raw)))
        } else {
            Ok(None)
        }
    }

    fn get_files(&self) -> Result<Option<Vec<String>>, PlatformError> {
        Ok(None)
    }
}

/// Parse CF_HTML format and extract the HTML fragment.
/// CF_HTML header format:
///   Version:...
///   StartHTML:...
///   EndHTML:...
///   StartFragment:...
///   EndFragment:...
///   ...<html>...
fn extract_html_fragment(cf_html: &str) -> String {
    let start = cf_html
        .lines()
        .find_map(|line| line.strip_prefix("StartFragment:"))
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let end = cf_html
        .lines()
        .find_map(|line| line.strip_prefix("EndFragment:"))
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(cf_html.len());

    if start < end && end <= cf_html.len() {
        cf_html[start..end].to_string()
    } else {
        cf_html.to_string()
    }
}
