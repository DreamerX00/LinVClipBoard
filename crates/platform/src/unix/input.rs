use crate::error::PlatformError;
use crate::traits::InputSimulator;
use std::process::Command;

pub struct UnixInputSimulator;

impl Default for UnixInputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl UnixInputSimulator {
    pub fn new() -> Self {
        Self
    }

    fn run_silent(program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

impl InputSimulator for UnixInputSimulator {
    fn paste_text(&self, text: &str) -> Result<(), PlatformError> {
        let mut clip = arboard::Clipboard::new()
            .map_err(|e| PlatformError::Input(format!("clipboard init: {}", e)))?;
        clip.set_text(text)
            .map_err(|e| PlatformError::Input(format!("set_text: {}", e)))?;
        drop(clip);

        if Self::run_silent("wtype", &["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"]) {
            return Ok(());
        }
        if Self::run_silent("xdotool", &["key", "--clearmodifiers", "ctrl+v"]) {
            return Ok(());
        }
        if Self::run_silent("ydotool", &["key", "29", "47", "47", "29"]) {
            return Ok(());
        }
        Err(PlatformError::Input(
            "no paste tool found (tried wtype, xdotool, ydotool)".to_string(),
        ))
    }

    fn type_text(&self, text: &str) -> Result<(), PlatformError> {
        if Self::run_silent("wtype", &["--", text]) {
            return Ok(());
        }
        if Self::run_silent("xdotool", &["type", "--clearmodifiers", "--", text]) {
            return Ok(());
        }
        if Self::run_silent("ydotool", &["type", "--", text]) {
            return Ok(());
        }
        Err(PlatformError::Input(
            "no type tool found (tried wtype, xdotool, ydotool)".to_string(),
        ))
    }

    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError> {
        if Self::run_silent("wtype", &["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"]) {
            return Ok(());
        }
        if Self::run_silent("xdotool", &["key", "--clearmodifiers", "ctrl+v"]) {
            return Ok(());
        }
        Err(PlatformError::Input(
            "no paste shortcut tool found".to_string(),
        ))
    }
}
