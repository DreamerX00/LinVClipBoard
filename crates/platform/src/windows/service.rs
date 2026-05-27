use crate::error::PlatformError;
use crate::traits::ServiceManager;
use winreg::enums::*;
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "LinVClipBoard";

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
    fn register_autostart(&self, app_path: &str) -> Result<(), PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        key.set_value(APP_NAME, &app_path)
            .map_err(|e| PlatformError::Service(format!("Set value: {e}")))?;
        Ok(())
    }

    fn unregister_autostart(&self) -> Result<(), PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        key.delete_value(APP_NAME)
            .map_err(|e| PlatformError::Service(format!("Delete value: {e}")))?;
        Ok(())
    }

    fn is_autostart_enabled(&self) -> Result<bool, PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_READ)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        let value: Result<String, _> = key.get_value(APP_NAME);
        Ok(value.is_ok())
    }
}
