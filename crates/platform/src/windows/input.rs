use crate::error::PlatformError;
use crate::traits::InputSimulator;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

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
    fn paste_text(&self, text: &str) -> Result<(), PlatformError> {
        // Set clipboard content using the Windows clipboard provider
        let provider = crate::windows::WindowsClipboardProvider::new()?;
        provider.set_text(text)?;

        std::thread::sleep(std::time::Duration::from_millis(50));

        self.simulate_paste_shortcut()
    }

    fn type_text(&self, text: &str) -> Result<(), PlatformError> {
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| PlatformError::Input(e.to_string()))?;
        release_all_modifiers(&mut enigo)?;
        enigo
            .text(text)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        Ok(())
    }

    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError> {
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| PlatformError::Input(e.to_string()))?;

        release_all_modifiers(&mut enigo)?;

        enigo
            .key(Key::Control, Press)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        std::thread::sleep(std::time::Duration::from_millis(15));
        enigo
            .key(Key::Layout('v'), Click)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        std::thread::sleep(std::time::Duration::from_millis(15));
        enigo
            .key(Key::Control, Release)
            .map_err(|e| PlatformError::Input(e.to_string()))?;

        Ok(())
    }
}

fn release_all_modifiers(enigo: &mut Enigo) -> Result<(), PlatformError> {
    let mods = [Key::Control, Key::Shift, Key::Alt, Key::Meta];
    for key in &mods {
        let _ = enigo.key(*key, Release);
    }
    Ok(())
}
