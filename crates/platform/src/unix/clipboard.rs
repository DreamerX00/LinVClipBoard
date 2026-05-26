use crate::error::PlatformError;
use crate::traits::ClipboardProvider;
use arboard::Clipboard;
use std::sync::Mutex;

pub struct UnixClipboardProvider {
    inner: Mutex<Clipboard>,
}

impl UnixClipboardProvider {
    pub fn new() -> Result<Self, PlatformError> {
        let clipboard =
            Clipboard::new().map_err(|e| PlatformError::Clipboard(format!("init: {}", e)))?;
        Ok(Self {
            inner: Mutex::new(clipboard),
        })
    }
}

impl ClipboardProvider for UnixClipboardProvider {
    fn get_text(&self) -> Result<Option<String>, PlatformError> {
        let mut clip = self.inner.lock().unwrap();
        match clip.get_text() {
            Ok(t) if t.trim().is_empty() => Ok(None),
            Ok(t) => Ok(Some(t)),
            Err(e) => Err(PlatformError::Clipboard(format!("get_text: {}", e))),
        }
    }

    fn set_text(&self, text: &str) -> Result<(), PlatformError> {
        let mut clip = self.inner.lock().unwrap();
        clip.set_text(text)
            .map_err(|e| PlatformError::Clipboard(format!("set_text: {}", e)))
    }

    fn get_image(&self) -> Result<Option<(Vec<u8>, usize, usize)>, PlatformError> {
        let mut clip = self.inner.lock().unwrap();
        match clip.get_image() {
            Ok(img) => {
                let bytes = img.bytes.to_vec();
                if bytes.is_empty() {
                    return Ok(None);
                }
                Ok(Some((bytes, img.width, img.height)))
            }
            Err(_) => Ok(None),
        }
    }

    fn set_image(&self, data: &[u8], width: usize, height: usize) -> Result<(), PlatformError> {
        let mut clip = self.inner.lock().unwrap();
        let img_data = arboard::ImageData {
            width,
            height,
            bytes: std::borrow::Cow::Borrowed(data),
        };
        clip.set_image(img_data)
            .map_err(|e| PlatformError::Clipboard(format!("set_image: {}", e)))
    }

    fn get_html(&self) -> Result<Option<String>, PlatformError> {
        Ok(None)
    }

    fn get_files(&self) -> Result<Option<Vec<String>>, PlatformError> {
        Ok(None)
    }
}
