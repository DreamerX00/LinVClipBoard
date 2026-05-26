use crate::error::PlatformError;
use crate::traits::ServiceManager;
use std::path::PathBuf;
use std::process::Command;

pub struct UnixServiceManager;

impl Default for UnixServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UnixServiceManager {
    pub fn new() -> Self {
        Self
    }

    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("systemd")
            .join("user")
    }
}

impl ServiceManager for UnixServiceManager {
    fn register_autostart(&self, app_path: &str) -> Result<(), PlatformError> {
        let service_content = format!(
            "[Unit]
Description=LinVClipBoard Clipboard Daemon
After=graphical-session.target

[Service]
ExecStart={}
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
",
            app_path
        );

        let service_dir = Self::config_dir();
        std::fs::create_dir_all(&service_dir)
            .map_err(|e| PlatformError::Service(format!("cannot create systemd dir: {}", e)))?;

        let service_path = service_dir.join("clipd.service");
        std::fs::write(&service_path, &service_content)
            .map_err(|e| PlatformError::Service(format!("cannot write service file: {}", e)))?;

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "clipd.service"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "start", "clipd.service"])
            .status();

        Ok(())
    }

    fn unregister_autostart(&self) -> Result<(), PlatformError> {
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "clipd.service"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "clipd.service"])
            .status();

        let service_path = Self::config_dir().join("clipd.service");
        if service_path.exists() {
            let _ = std::fs::remove_file(&service_path);
        }

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        Ok(())
    }

    fn is_autostart_enabled(&self) -> Result<bool, PlatformError> {
        let output = Command::new("systemctl")
            .args(["--user", "is-enabled", "clipd.service"])
            .output()
            .map_err(|e| PlatformError::Service(format!("systemctl: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout == "enabled")
    }
}
