use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub daemon: DaemonConfig,
    pub security: SecurityConfig,
    pub ui: UiConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Clipboard poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Log level: "trace", "debug", "info", "warn", "error".
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Apps whose clipboard content should never be stored.
    pub blacklisted_apps: Vec<String>,
    /// If true, don't store any clipboard content.
    pub incognito: bool,
    /// Auto-delete sensitive items after N minutes (0 = disabled).
    #[serde(default)]
    pub sensitive_expiry_minutes: u32,
    /// Clear clipboard after first paste.
    #[serde(default)]
    pub clear_after_paste: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Auto-OCR images on clipboard capture (off by default — resource intensive).
    #[serde(default)]
    pub auto_ocr: bool,
    /// Show smart paste detection badges.
    #[serde(default = "default_true")]
    pub smart_paste: bool,
    /// Auto-detect and redact sensitive content.
    #[serde(default = "default_true")]
    pub redact_sensitive: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            auto_ocr: false,
            smart_paste: true,
            redact_sensitive: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme: "auto", "dark", or "light".
    pub theme: String,
    pub window_width: u32,
    pub window_height: u32,
    /// Global shortcut to toggle the overlay (e.g. "Ctrl+/").
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    /// UI language code: "en", "pt", etc.
    #[serde(default = "default_language")]
    pub language: String,
    /// Zoom level as a percentage (50–200). Default 100.
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    /// Window positioning mode: "fixed" or "mouse".
    #[serde(default = "default_window_position")]
    pub window_position: String,
    /// Accent color override: "auto" (use theme default) or a hex string like "#818cf8".
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    /// Number of recent items to show in the system tray menu (3–15).
    #[serde(default = "default_tray_items")]
    pub tray_items: u32,
}

fn default_shortcut() -> String {
    "Ctrl+/".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_zoom() -> u32 {
    100
}

fn default_window_position() -> String {
    "mouse".to_string()
}

fn default_accent_color() -> String {
    "auto".to_string()
}

fn default_tray_items() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum number of items to keep.
    pub max_items: u64,
    /// Maximum size of a single item in bytes.
    pub max_item_size_bytes: u64,
    /// Days after which non-pinned items expire.
    pub expiry_days: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                poll_interval_ms: 250,
                log_level: "info".to_string(),
            },
            security: SecurityConfig {
                blacklisted_apps: vec![
                    "keepassxc".to_string(),
                    "1password".to_string(),
                    "bitwarden".to_string(),
                ],
                incognito: false,
                sensitive_expiry_minutes: 0,
                clear_after_paste: false,
            },
            ui: UiConfig {
                theme: "auto".to_string(),
                window_width: 420,
                window_height: 520,
                shortcut: "Ctrl+/".to_string(),
                language: "en".to_string(),
                zoom: 100,
                window_position: "mouse".to_string(),
                accent_color: "auto".to_string(),
                tray_items: 5,
            },
            features: FeaturesConfig::default(),
            storage: StorageConfig {
                max_items: 10_000,
                max_item_size_bytes: 50 * 1024 * 1024, // 50MB
                expiry_days: 30,
            },
        }
    }
}

impl AppConfig {
    /// Path to the config file.
    pub fn config_path() -> PathBuf {
        #[cfg(unix)]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("linvclip")
                .join("config.toml")
        }
        #[cfg(windows)]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming"))
                .join("LinVClipBoard")
                .join("config.toml")
        }
    }

    /// Path to the data directory.
    pub fn data_dir() -> PathBuf {
        #[cfg(unix)]
        {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("~/.local/share"))
                .join("linvclip")
        }
        #[cfg(windows)]
        {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"))
                .join("LinVClipBoard")
        }
    }

    /// Path to the blob storage directory.
    pub fn blob_dir() -> PathBuf {
        Self::data_dir().join("blobs")
    }

    /// Path to the database file.
    pub fn db_path() -> PathBuf {
        Self::data_dir().join("clipboard.db")
    }

    /// Path to the IPC transport endpoint.
    /// Unix: Unix domain socket at `/run/user/<uid>/linvclip.sock` or `/tmp/linvclip-<uid>.sock`.
    /// Windows: Named pipe at `\\.\pipe\LinVClipBoard`.
    pub fn socket_path() -> PathBuf {
        #[cfg(unix)]
        {
            let uid = unsafe { libc::getuid() };
            let run_dir = PathBuf::from(format!("/run/user/{}", uid));
            if run_dir.exists() {
                run_dir.join("linvclip.sock")
            } else {
                PathBuf::from(format!("/tmp/linvclip-{}.sock", uid))
            }
        }
        #[cfg(windows)]
        {
            PathBuf::from(r"\\.\pipe\LinVClipBoard")
        }
    }

    /// Load config from file, or create with defaults if missing.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<AppConfig>(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        tracing::warn!("Failed to parse config: {}, using defaults", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}, using defaults", e);
                }
            }
        } else {
            // Create default config file
            let config = AppConfig::default();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match toml::to_string_pretty(&config) {
                Ok(content) => {
                    let _ = std::fs::write(&path, content);
                    tracing::info!("Created default config at {:?}", path);
                }
                Err(e) => {
                    tracing::warn!("Failed to serialize default config: {}", e);
                }
            }
            return config;
        }
        AppConfig::default()
    }
}
