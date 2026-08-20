use base64::Engine;
use serde::{Deserialize, Serialize};
use shared::config::AppConfig;
use shared::ipc::send_request;
use shared::models::{ClipboardItem, IpcRequest, IpcResponse, Snippet};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_updater::UpdaterExt;

// Obfuscated KLIPY API key generated at build time (XOR-scrambled).
include!(concat!(env!("OUT_DIR"), "/klipy_key.rs"));

#[derive(Serialize, Deserialize)]
pub struct ItemsResult {
    pub items: Vec<ClipboardItem>,
    pub total: u64,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResult {
    pub uptime_secs: u64,
    pub total_items: u64,
    pub db_size_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GifItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub preview_url: String,
    pub gif_url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct GifResult {
    pub items: Vec<GifItem>,
    pub page: u32,
    pub has_next: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GifCategory {
    pub category: String,
    pub query: String,
    pub preview_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: String,
    pub download_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

/// Get clipboard items from the daemon.
#[tauri::command]
async fn get_items(offset: u32, limit: u32) -> Result<ItemsResult, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::List { offset, limit };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Items { items, total }) => Ok(ItemsResult { items, total }),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}. Is clipd running?", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Search clipboard items.
#[tauri::command]
async fn search_items(query: String, limit: u32, offset: u32) -> Result<ItemsResult, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::Search {
        query,
        limit,
        offset,
    };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Items { items, total }) => Ok(ItemsResult { items, total }),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Search clipboard items using regex.
#[tauri::command]
async fn search_items_regex(
    pattern: String,
    limit: u32,
    offset: u32,
) -> Result<ItemsResult, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::SearchRegex {
        pattern,
        limit,
        offset,
    };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Items { items, total }) => Ok(ItemsResult { items, total }),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Paste an item back to clipboard.
#[tauri::command]
async fn paste_item(id: String) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::Paste { id };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Write arbitrary text (emoji/symbol) directly to the system clipboard.
#[tauri::command]
async fn paste_raw_text(text: String) -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;
    Ok("ok".to_string())
}

/// Type text into the previously focused application.
///
/// 1. Hides the window so focus returns to the previous app.
/// 2. Types the text using platform input simulation (enigo/SendInput on Windows,
///    wtype/xdotool/ydotool on Linux).
/// 3. Falls back to clipboard copy if typing is unavailable.
///
/// Returns `"typed"` on success, `"copied"` when falling back to clipboard.
#[tauri::command]
async fn type_text(text: String, app: tauri::AppHandle) -> Result<String, String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    #[cfg(windows)]
    {
        let sim = platform::WindowsInputSimulator::new();
        let _ = sim.type_text(&text);
        return Ok("typed".to_string());
    }

    #[cfg(unix)]
    {
        if try_type_direct(&text) {
            return Ok("typed".to_string());
        }

        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        clipboard
            .set_text(&text)
            .map_err(|e| format!("Failed to set clipboard: {}", e))?;

        if try_paste_shortcut() {
            return Ok("typed".to_string());
        }

        Ok("copied".to_string())
    }
}

/// Try to type text directly into the currently focused window.
fn try_type_direct(text: &str) -> bool {
    // wtype — works on wlroots-based Wayland compositors (Sway, Hyprland, etc.)
    if run_silent("wtype", &["--", text]) {
        return true;
    }
    // xdotool — works on X11 and XWayland
    if run_silent("xdotool", &["type", "--clearmodifiers", "--", text]) {
        return true;
    }
    // ydotool — works on both X11 and Wayland (needs ydotoold)
    if run_silent("ydotool", &["type", "--", text]) {
        return true;
    }
    false
}

/// Try to simulate Ctrl+V in the currently focused window.
fn try_paste_shortcut() -> bool {
    if run_silent("wtype", &["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"]) {
        return true;
    }
    if run_silent("xdotool", &["key", "--clearmodifiers", "ctrl+v"]) {
        return true;
    }
    false
}

/// Run a command silently, returning true if it exits successfully.
fn run_silent(program: &str, args: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Toggle pin on an item.
#[tauri::command]
async fn pin_item(id: String) -> Result<ClipboardItem, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::TogglePin { id };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Item(item)) => Ok(item),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Bulk delete items.
#[tauri::command]
async fn bulk_delete(ids: Vec<String>) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::BulkDelete { ids };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Bulk pin/unpin items.
#[tauri::command]
async fn bulk_pin(ids: Vec<String>, pinned: bool) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::BulkPin { ids, pinned };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Paste an HTML item as plain text only.
#[tauri::command]
async fn paste_as_plain_text(id: String) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    // First get the item to extract plain text from HTML
    let get_req = IpcRequest::Get { id };
    match send_request(&socket, &get_req).await {
        Ok(IpcResponse::Item(item)) => {
            let plain = if item.content_type == shared::models::ContentType::Html {
                html2text::from_read(item.content.as_bytes(), 200).unwrap_or_default()
            } else {
                item.content
            };
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
            clipboard
                .set_text(&plain)
                .map_err(|e| format!("Clipboard set failed: {}", e))?;
            Ok("Pasted as plain text".to_string())
        }
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

// ─── Snippet Commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn list_snippets(folder: Option<String>) -> Result<Vec<Snippet>, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::ListSnippets { folder };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Snippets(snippets)) => Ok(snippets),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn search_snippets(query: String) -> Result<Vec<Snippet>, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::SearchSnippets { query };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Snippets(snippets)) => Ok(snippets),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn create_snippet(
    name: String,
    content: String,
    folder: String,
    abbreviation: String,
    variables: String,
) -> Result<Snippet, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::CreateSnippet {
        name,
        content,
        folder,
        abbreviation,
        variables,
    };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Snippet(s)) => Ok(s),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn update_snippet(
    id: String,
    name: String,
    content: String,
    folder: String,
    abbreviation: String,
    variables: String,
) -> Result<Snippet, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::UpdateSnippet {
        id,
        name,
        content,
        folder,
        abbreviation,
        variables,
    };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Snippet(s)) => Ok(s),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn delete_snippet(id: String) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::DeleteSnippet { id };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
async fn use_snippet(
    id: String,
    variables: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::UseSnippet { id, variables };
    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Delete an item.
#[tauri::command]
async fn delete_item(id: String) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::Delete { id };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Get daemon status.
#[tauri::command]
async fn get_status() -> Result<StatusResult, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::Status;

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Status {
            uptime_secs,
            total_items,
            db_size_bytes,
        }) => Ok(StatusResult {
            uptime_secs,
            total_items,
            db_size_bytes,
        }),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Clear all non-pinned items.
#[tauri::command]
async fn clear_all() -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::Clear;

    let result = match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    };

    // Also purge the GIF cache
    if let Ok(dir) = gif_cache_dir() {
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
        }
    }

    result
}

/// Get the current configuration.
#[tauri::command]
async fn get_config() -> Result<AppConfig, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::GetConfig;

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Config(config)) => Ok(config),
        Ok(IpcResponse::Error { message }) => Err(message),
        // Fallback: load directly from file when daemon doesn't support GetConfig
        Err(_) => Ok(AppConfig::load()),
        _ => Ok(AppConfig::load()),
    }
}

/// Save configuration.
#[tauri::command]
async fn save_config(config: AppConfig) -> Result<String, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::SaveConfig { config };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Ok { message }) => Ok(message),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Get a base64-encoded image for preview (#38).
#[tauri::command]
async fn get_image_base64(path: String) -> Result<String, String> {
    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("Image file not found".to_string());
    }
    let bytes = std::fs::read(file_path).map_err(|e| format!("Read failed: {}", e))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", encoded))
}

/// Extract text from an image using Tesseract OCR (CLI).
/// Gracefully returns a helpful error if tesseract is not installed.
#[tauri::command]
async fn extract_text_from_image(
    image_path: String,
    lang: Option<String>,
) -> Result<String, String> {
    let path = std::path::Path::new(&image_path);
    if !path.exists() {
        return Err("Image file not found".to_string());
    }

    let language = lang.unwrap_or_else(|| "eng".to_string());

    #[cfg(unix)]
    {
        let output = tokio::process::Command::new("tesseract")
            .args([&image_path, "stdout", "-l", &language])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "Tesseract OCR is not installed. Install it with: sudo apt install tesseract-ocr tesseract-ocr-eng".to_string()
                } else {
                    format!("OCR failed: {}", e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("OCR failed: {}", stderr.trim()));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err("No text found in image".to_string());
        }

        Ok(text)
    }

    #[cfg(windows)]
    {
        Err("OCR via Tesseract is not available on Windows. Use the Windows Snipping Tool or a third-party OCR application.".to_string())
    }
}

/// Update the preview_text of an item via the daemon (makes OCR text searchable).
#[tauri::command]
async fn update_preview_text(id: String, preview_text: String) -> Result<ClipboardItem, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::UpdatePreviewText { id, preview_text };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Item(item)) => Ok(item),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Return the compile-time application version.
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the path to the clipd executable (same directory as this GUI).
#[cfg_attr(unix, allow(dead_code))]
fn get_clipd_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap_or_default();
    path.set_file_name("clipd");
    path.set_extension("exe");
    path
}

/// Set whether clipd launches at user login (Windows: HKCU\...\Run).
#[tauri::command]
async fn set_launch_at_startup(enabled: bool) -> Result<(), String> {
    set_launch_at_startup_impl(enabled)
}

#[cfg(windows)]
fn set_launch_at_startup_impl(enabled: bool) -> Result<(), String> {
    let mgr = platform::WindowsServiceManager::new();
    let clipd_path = get_clipd_path().to_string_lossy().to_string();
    if enabled {
        mgr.register_autostart(&clipd_path)
    } else {
        mgr.unregister_autostart()
    }
    .map_err(|e| e.to_string())
}

#[cfg(unix)]
fn set_launch_at_startup_impl(_enabled: bool) -> Result<(), String> {
    Err("Autostart management not available on this platform. Use systemctl --user.".to_string())
}

/// Check whether clipd launches at user login.
#[tauri::command]
async fn is_launch_at_startup_enabled() -> Result<bool, String> {
    is_launch_at_startup_enabled_impl()
}

#[cfg(windows)]
fn is_launch_at_startup_enabled_impl() -> Result<bool, String> {
    let mgr = platform::WindowsServiceManager::new();
    mgr.is_autostart_enabled().map_err(|e| e.to_string())
}

#[cfg(unix)]
fn is_launch_at_startup_enabled_impl() -> Result<bool, String> {
    Ok(false)
}

/// Check GitHub releases for a newer version.
#[tauri::command]
async fn check_for_updates() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION");

    let client = reqwest::Client::builder()
        .user_agent("LinVClipBoard")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get("https://api.github.com/repos/DreamerX00/LinVClipBoard/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let tag = body["tag_name"].as_str().ok_or("No tag_name in response")?;
    let latest = tag.trim_start_matches('v');
    let html_url = body["html_url"]
        .as_str()
        .unwrap_or("https://github.com/DreamerX00/LinVClipBoard/releases");

    let release_notes = body["body"].as_str().unwrap_or("").to_string();

    // Find the right asset download URL for this platform
    let asset_suffix = if cfg!(unix) { ".deb" } else { ".exe" };
    let download_url = body["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find_map(|a| {
                let name = a["name"].as_str().unwrap_or("");
                if name.ends_with(asset_suffix) {
                    a["browser_download_url"].as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    // Simple semver comparison: split by '.' and compare numerically
    let parse_ver = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
    let cur_parts = parse_ver(current);
    let lat_parts = parse_ver(latest);
    let has_update = lat_parts > cur_parts;

    Ok(UpdateInfo {
        has_update,
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        release_url: html_url.to_string(),
        release_notes,
        download_url,
    })
}

/// Download a .deb update from GitHub releases, emitting progress events.
/// The file is saved to ~/Downloads/linvclipboard_<version>.deb.
#[tauri::command]
async fn download_update(
    url: String,
    version: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let download_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .ok_or("Cannot determine Downloads directory")?;
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Cannot create download dir: {}", e))?;

    let ext = if cfg!(unix) { "deb" } else { "exe" };
    let filename = format!("linvclipboard_{}_x86_64.{}", version, ext);
    let dest = download_dir.join(&filename);

    let client = reqwest::Client::builder()
        .user_agent("LinVClipBoard")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download error: HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file =
        std::fs::File::create(&dest).map_err(|e| format!("Cannot create file: {}", e))?;

    use std::io::Write;
    let mut stream = resp.bytes_stream();
    use tokio_stream::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
            },
        );
    }

    Ok(dest.to_string_lossy().to_string())
}

/// Install a downloaded update.
///
/// On Linux, uses pkexec + dpkg to install a .deb.
/// On Windows, returns an info message (handled by the Tauri updater plugin).
#[tauri::command]
async fn install_update(path: String) -> Result<String, String> {
    install_update_impl(&path).await
}

#[cfg(unix)]
async fn install_update_impl(path: &str) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("File not found".to_string());
    }
    if !path.ends_with(".deb") {
        return Err("Not a .deb file".to_string());
    }

    // Capture current user environment for the restart script.
    let username = std::env::var("USER").unwrap_or_default();
    let display = std::env::var("DISPLAY").unwrap_or_default();
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();
    let dbus_addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
    let xauthority = std::env::var("XAUTHORITY").unwrap_or_default();
    let pid = std::process::id();

    // Sentinel file: the install script touches this after dpkg finishes so
    // the restart script knows it is safe to proceed.
    let sentinel = format!("/tmp/linvclip-dpkg-done-{pid}");

    // ── restart script (runs as the original user, detached) ──────────
    // Launched by the install script *before* dpkg.  It waits for the
    // sentinel, then brings up the new version of the app.
    let restart_script_path = format!("/tmp/linvclip-restart-{pid}.sh");
    let restart_script = format!(
        r#"#!/bin/bash
trap '' PIPE
exec >> /tmp/linvclip-restart.log 2>&1
echo "=== restart script started at $(date) ==="

export DBUS_SESSION_BUS_ADDRESS="{dbus_addr}"
export XDG_RUNTIME_DIR="{xdg_runtime}"
export HOME="{home}"
export DISPLAY="{display}"
export WAYLAND_DISPLAY="{wayland}"
export XAUTHORITY="{xauthority}"

# Wait for the install script to signal that dpkg is done (up to 120 s).
SENTINEL="{sentinel}"
echo "waiting for sentinel $SENTINEL ..."
for _i in $(seq 1 120); do
    [ -f "$SENTINEL" ] && break
    sleep 1
done
rm -f "$SENTINEL"
echo "sentinel received (or timeout) — continuing"

# Belt-and-suspenders: also wait for the dpkg lock to be released.
for _i in $(seq 1 30); do
    if ! fuser /var/lib/dpkg/lock-frontend >/dev/null 2>&1 && \
       ! fuser /var/lib/dpkg/lock >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
sleep 1

# Sync user-local binaries (manual install.sh puts them in ~/.local/bin)
LOCAL_BIN="{home}/.local/bin"
if [ -d "$LOCAL_BIN" ]; then
    for bin in clipd clipctl linvclip-ui; do
        [ -f "/usr/bin/$bin" ] && cp "/usr/bin/$bin" "$LOCAL_BIN/" 2>/dev/null || true
    done
    echo "synced local binaries"
fi

# Reload and restart the daemon
systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable clipd.service 2>/dev/null || true
systemctl --user restart clipd.service 2>/dev/null || true
echo "clipd restarted"
sleep 1

# Kill any stale UI and launch the new one
pkill -x linvclip-ui 2>/dev/null || true
sleep 0.5

UI_BIN="/usr/bin/linvclip-ui"
[ -f "{home}/.local/bin/linvclip-ui" ] && UI_BIN="{home}/.local/bin/linvclip-ui"
echo "launching $UI_BIN"
nohup setsid "$UI_BIN" >/dev/null 2>&1 &

echo "=== restart script finished at $(date) ==="
rm -f "{restart_script_path}"
"#
    );
    std::fs::write(&restart_script_path, &restart_script).map_err(|e| e.to_string())?;

    // ── install script (runs as root via pkexec) ──────────────────────
    //
    // CRITICAL: the very first action is `exec > logfile 2>&1` which
    // redirects stdout/stderr to a file.  Without this, dpkg inherits
    // pkexec's pipes that point back to the Tauri UI process.  When the
    // old prerm kills the UI (`pkill -x linvclip-ui`), those pipes
    // break and any subsequent write delivers SIGPIPE — silently killing
    // dpkg mid-install, leaving a half-installed package and no restart.
    //
    // The restart script is launched *before* dpkg so that even if
    // something unexpected kills the install script, the restart process
    // is already running and will bring the app back up after a timeout.
    let install_script = format!(
        r#"#!/bin/bash
trap '' PIPE
exec > /tmp/linvclip-update.log 2>&1

echo "=== install script started at $(date) ==="
echo "package: {path}"

# 1. Launch the restart script as the original user in a fully detached
#    process (setsid = new session, nohup = ignore SIGHUP).  It will
#    sleep until the sentinel file appears.
su "{username}" -c "nohup setsid bash '{restart_script_path}' >/dev/null 2>&1 &"
echo "restart script launched"

# 2. Remove the old Tauri-generated package if it conflicts.
if dpkg -s lin-v-clip-board >/dev/null 2>&1; then
    echo "removing old lin-v-clip-board package..."
    dpkg --remove --force-depends lin-v-clip-board || true
fi

# 3. Install the new .deb.  dpkg's prerm may kill the UI — that is fine
#    because we already redirected output away from the pipes.
echo "running dpkg ..."
DEBIAN_FRONTEND=noninteractive dpkg -i --force-overwrite "{path}" || true

# 3. Fix any unmet dependencies that dpkg could not resolve alone.
DEBIAN_FRONTEND=noninteractive apt-get install -f -y 2>&1 || true

# 4. Kill any UI that the postinst may have launched (we restart cleanly
#    via the restart script).
pkill -x linvclip-ui 2>/dev/null || true

# 5. Signal the restart script that dpkg is done.
touch "{sentinel}"
echo "sentinel touched — restart script will proceed"

echo "=== install script finished at $(date) ==="
exit 0
"#
    );
    let install_script_path = format!("/tmp/linvclip-update-{pid}.sh");
    std::fs::write(&install_script_path, &install_script).map_err(|e| e.to_string())?;

    // Run the install script as root.  If the old prerm kills the UI,
    // this .output() future will be abandoned (the process is dead) — but
    // that is fine: the restart script is already running inside a
    // detached session and will bring the new version up.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new("pkexec")
            .args(["bash", &install_script_path])
            .output(),
    )
    .await;

    let _ = std::fs::remove_file(&install_script_path);

    match result {
        Ok(Ok(output)) if output.status.success() => {
            // pkexec returned 0 and the UI is still alive (new prerm
            // doesn't kill the UI on upgrade).  Exit gracefully — the
            // restart script is already running and will bring up the
            // new version.
            std::process::exit(0);
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("dismissed")
                || stderr.contains("Not authorized")
                || output.status.code() == Some(126)
            {
                // User cancelled the auth dialog — kill the restart
                // script (it was never launched because the install
                // script didn't run) and clean up files.
                let _ = std::fs::remove_file(&restart_script_path);
                let _ = std::fs::remove_file(&sentinel);
                Err("auth_cancelled".to_string())
            } else {
                Err(format!("Install failed: {}", stderr.trim()))
            }
        }
        Ok(Err(_)) | Err(_) => {
            // Process died (old prerm killed us) or timeout.
            // The restart script is already running and will handle it.
            Ok("installed".to_string())
        }
    }
}

#[cfg(windows)]
async fn install_update_impl(_path: &str) -> Result<String, String> {
    Err("Not used on Windows — the app updates itself via the built-in updater. If this persists, download the latest installer from GitHub releases.".to_string())
}

/// Check for updates using the Tauri updater plugin (Windows only).
#[tauri::command]
async fn check_for_updates_via_plugin(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let checked = match app.updater() {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let update = match checked {
        Ok(u) => u,
        // ponytail: no update manifest published yet (or plugin misconfigured) —
        // fall back to the GitHub API check so Windows users still learn of updates
        Err(_) => return check_for_updates().await,
    };

    let current = env!("CARGO_PKG_VERSION").to_string();
    let mut info = UpdateInfo {
        has_update: false,
        current_version: current.clone(),
        latest_version: current,
        release_url: String::new(),
        release_notes: String::new(),
        download_url: String::new(),
    };
    if let Some(up) = update {
        info.has_update = true;
        info.latest_version = up.version;
        info.release_url = "https://github.com/DreamerX00/LinVClipBoard/releases/latest".to_string();
        info.release_notes = up.body.unwrap_or_default();
    }
    Ok(info)
}

/// Install update using the Tauri updater plugin (Windows only).
#[tauri::command]
async fn install_update_via_plugin(app: tauri::AppHandle) -> Result<String, String> {
    if !cfg!(windows) {
        // Guard: on a Linux .deb install the plugin would fall through to its
        // AppImage path and try to overwrite the running executable.
        return Err("Updater plugin install is Windows-only".to_string());
    }

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No update available".to_string())?;

    // Reuse the existing download-progress event so UpdateModal shows a live bar.
    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let total = total.unwrap_or(0);
                let percent = if total > 0 {
                    downloaded as f64 / total as f64 * 100.0
                } else {
                    0.0
                };
                let _ = progress_app.emit(
                    "download-progress",
                    DownloadProgress {
                        downloaded,
                        total,
                        percent,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    // Note: on success the plugin exits the app to run the installer, so this
    // return is normally only reached on the error path above.
    Ok("installed".to_string())
}

/// Decode the embedded KLIPY app key (XOR-descrambled at runtime).
fn get_gif_api_key() -> Result<String, String> {
    if KLIPY_KEY_BYTES.is_empty() {
        return Err("gif_api_key_missing".to_string());
    }
    let decoded: String = KLIPY_KEY_BYTES
        .iter()
        .enumerate()
        .map(|(i, &b)| (b ^ KLIPY_KEY_XOR_PAD[i % KLIPY_KEY_XOR_PAD.len()]) as char)
        .collect();
    Ok(decoded)
}

/// Helper: parse a KLIPY v1 GIF object from JSON.
fn parse_gif_item(r: &serde_json::Value) -> Option<GifItem> {
    let id = r["id"]
        .as_i64()
        .or_else(|| r["id"].as_u64().map(|v| v as i64))?;
    let slug = r["slug"].as_str().unwrap_or("").to_string();
    let title = r["title"].as_str().unwrap_or("").to_string();

    // Prefer sm.webp (fast, small) → sm.gif → xs.gif for preview
    // Use hd.gif for the URL users copy
    let file = &r["file"];
    let preview_url = file["sm"]["webp"]["url"]
        .as_str()
        .or_else(|| file["sm"]["gif"]["url"].as_str())
        .or_else(|| file["xs"]["gif"]["url"].as_str())?
        .to_string();
    let gif_url = file["hd"]["gif"]["url"]
        .as_str()
        .or_else(|| file["md"]["gif"]["url"].as_str())
        .unwrap_or(preview_url.as_str())
        .to_string();
    let width = file["sm"]["webp"]["width"]
        .as_u64()
        .or_else(|| file["sm"]["gif"]["width"].as_u64())
        .unwrap_or(220) as u32;
    let height = file["sm"]["webp"]["height"]
        .as_u64()
        .or_else(|| file["sm"]["gif"]["height"].as_u64())
        .unwrap_or(220) as u32;

    Some(GifItem {
        id: id.to_string(),
        slug,
        title,
        preview_url,
        gif_url,
        width,
        height,
    })
}

/// Fetch GIFs from the KLIPY v1 API.
///
/// If `query` is empty, fetches trending GIFs.
#[tauri::command]
async fn fetch_gifs(query: String, page: u32, per_page: u32) -> Result<GifResult, String> {
    let app_key = get_gif_api_key()?;

    let client = reqwest::Client::new();
    let (url, is_search) = if query.trim().is_empty() {
        (
            format!("https://api.klipy.com/api/v1/{}/gifs/trending", app_key),
            false,
        )
    } else {
        (
            format!("https://api.klipy.com/api/v1/{}/gifs/search", app_key),
            true,
        )
    };

    let mut params: Vec<(&str, String)> = vec![
        ("page", page.to_string()),
        ("per_page", per_page.to_string()),
        ("customer_id", "linvclipboard_user".to_string()),
        ("content_filter", "medium".to_string()),
        ("format_filter", "gif,webp,jpg".to_string()),
    ];
    if is_search {
        params.push(("q", query));
    }

    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API error: {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if body["result"].as_bool() != Some(true) {
        return Err("API returned error".to_string());
    }

    let data = &body["data"];
    let has_next = data["has_next"].as_bool().unwrap_or(false);
    let current_page = data["current_page"].as_u64().unwrap_or(page as u64) as u32;
    let results = data["data"].as_array().ok_or("No data array in response")?;

    let items: Vec<GifItem> = results.iter().filter_map(parse_gif_item).collect();

    Ok(GifResult {
        items,
        page: current_page,
        has_next,
    })
}

/// Fetch GIF categories from the KLIPY v1 API.
#[tauri::command]
async fn fetch_gif_categories() -> Result<Vec<GifCategory>, String> {
    let app_key = get_gif_api_key()?;

    let client = reqwest::Client::new();
    let url = format!("https://api.klipy.com/api/v1/{}/gifs/categories", app_key);

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API error: {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if body["result"].as_bool() != Some(true) {
        return Err("API returned error".to_string());
    }

    let cats = body["data"]["categories"]
        .as_array()
        .ok_or("No categories in response")?;

    let categories: Vec<GifCategory> = cats
        .iter()
        .filter_map(|c| {
            let category = c["category"].as_str()?.to_string();
            let query = c["query"].as_str()?.to_string();
            let preview_url = c["preview_url"].as_str()?.to_string();
            Some(GifCategory {
                category,
                query,
                preview_url,
            })
        })
        .collect();

    Ok(categories)
}

/// Register a GIF share event with KLIPY v1 API (POST).
#[tauri::command]
async fn register_gif_share(slug: String, query: String) -> Result<String, String> {
    let app_key = get_gif_api_key()?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.klipy.com/api/v1/{}/gifs/share/{}",
        app_key, slug
    );

    let mut body_map = serde_json::Map::new();
    body_map.insert(
        "customer_id".to_string(),
        serde_json::Value::String("linvclipboard_user".to_string()),
    );
    if !query.is_empty() {
        body_map.insert("q".to_string(), serde_json::Value::String(query));
    }

    let _ = client
        .post(&url)
        .json(&serde_json::Value::Object(body_map))
        .send()
        .await;

    Ok("ok".to_string())
}

/// Return the GIF cache directory, creating it if needed.
fn gif_cache_dir() -> Result<std::path::PathBuf, String> {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("linvclip")
        .join("gifs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;
    Ok(dir)
}

/// Copy a GIF URL to the system clipboard.
///
/// Worldwide, GIF keyboards on desktop work by copying the direct GIF URL
/// as plain text.  Chat apps (Discord, Telegram, Slack, etc.) auto-embed
/// direct `.gif` links, displaying them as animated images.
///
/// The `image/gif` MIME type is NOT supported by most paste targets (Electron
/// apps read `image/png` or `text/plain`), and `wl-clipboard` cannot offer
/// multiple MIME types simultaneously, so URL-based sharing is the standard.
#[tauri::command]
async fn copy_gif(url: String) -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&url)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;
    Ok("ok".to_string())
}

/// Purge all cached GIF files.
#[tauri::command]
async fn clear_gif_cache() -> Result<String, String> {
    if let Ok(dir) = gif_cache_dir() {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to clear GIF cache: {}", e))?;
            // Re-create empty dir
            std::fs::create_dir_all(&dir).ok();
        }
    }
    Ok("ok".to_string())
}

/// Purge GIF cache files older than the configured expiry days.
/// Called on app startup.
fn cleanup_expired_gif_cache() {
    let expiry_days = {
        let cfg = AppConfig::load();
        cfg.storage.expiry_days
    };
    let max_age = std::time::Duration::from_secs(expiry_days as u64 * 86400);

    let dir = match gif_cache_dir() {
        Ok(d) => d,
        Err(_) => return,
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            let age = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok());
            if let Some(age) = age {
                if age > max_age {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// Add a tag to an item.
#[tauri::command]
async fn add_tag(id: String, tag: String) -> Result<ClipboardItem, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::AddTag { id, tag };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Item(item)) => Ok(item),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Remove a tag from an item.
#[tauri::command]
async fn remove_tag(id: String, tag: String) -> Result<ClipboardItem, String> {
    let socket = AppConfig::socket_path();
    let request = IpcRequest::RemoveTag { id, tag };

    match send_request(&socket, &request).await {
        Ok(IpcResponse::Item(item)) => Ok(item),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(e) => Err(format!("Connection failed: {}", e)),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Generate a QR code PNG for the given text, returned as a base64 data-URL.
#[tauri::command]
fn generate_qr_code(text: String) -> Result<String, String> {
    use image::Luma;
    use qrcode::QrCode;
    use std::io::Cursor;

    let code = QrCode::new(text.as_bytes()).map_err(|e| format!("QR encode error: {e}"))?;
    let img = code.render::<Luma<u8>>().quiet_zone(true).build();

    let mut buf: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {e}"))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Convert JSON string to YAML.
#[tauri::command]
fn convert_json_to_yaml(json_text: String) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|e| format!("Invalid JSON: {}", e))?;
    yaml_serde::to_string(&value).map_err(|e| e.to_string())
}

/// Convert YAML string to JSON.
#[tauri::command]
fn convert_yaml_to_json(yaml_text: String) -> Result<String, String> {
    let value: yaml_serde::Value =
        yaml_serde::from_str(&yaml_text).map_err(|e| format!("Invalid YAML: {}", e))?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

/// Convert JSON string to TOML.
#[tauri::command]
fn convert_json_to_toml(json_text: String) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|e| format!("Invalid JSON: {}", e))?;
    toml::to_string(&value).map_err(|e| e.to_string())
}

/// Convert TOML string to JSON.
#[tauri::command]
fn convert_toml_to_json(toml_text: String) -> Result<String, String> {
    let value: toml::Value =
        toml::from_str(&toml_text).map_err(|e| format!("Invalid TOML: {}", e))?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

/// Syntax-highlight a code string, returning HTML with inline styles.
#[tauri::command]
fn highlight_code(code: String, language: Option<String>) -> Result<String, String> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::{Color, ThemeSet};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];

    // Find syntax by name or try to guess from first line
    let syntax = language
        .as_deref()
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .or_else(|| ss.find_syntax_by_first_line(&code))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut hl = HighlightLines::new(syntax, theme);
    let mut html = String::with_capacity(code.len() * 2);
    html.push_str("<pre class=\"sh-code\">");

    for line in LinesWithEndings::from(&code) {
        let ranges = hl.highlight_line(line, &ss).map_err(|e| e.to_string())?;
        for (style, text) in ranges {
            let Color { r, g, b, .. } = style.foreground;
            let escaped = text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            html.push_str(&format!(
                "<span style=\"color:rgb({r},{g},{b})\">{escaped}</span>"
            ));
        }
    }
    html.push_str("</pre>");
    Ok(html)
}

/// Detect the likely programming language for a code snippet.
#[tauri::command]
fn detect_language(code: String) -> String {
    use syntect::parsing::SyntaxSet;
    let ss = SyntaxSet::load_defaults_newlines();
    ss.find_syntax_by_first_line(&code)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "Plain Text".to_string())
}

/// Fetch Open-Graph / meta-tag preview data for a URL.
#[derive(Serialize, Deserialize, Clone)]
pub struct LinkPreview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub favicon: Option<String>,
}

#[tauri::command]
async fn fetch_link_preview(url: String) -> Result<LinkPreview, String> {
    use scraper::{Html, Selector};

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header("User-Agent", "LinVClipBoard/1.9 link-preview")
        .send()
        .await
        .map_err(|e| format!("Fetch failed: {e}"))?;

    // Only read up to 1MB of HTML
    let body = resp.text().await.map_err(|e| format!("Read body: {e}"))?;
    let body = if body.len() > 1_048_576 {
        body[..1_048_576].to_string()
    } else {
        body
    };

    let doc = Html::parse_document(&body);

    let og = |prop: &str| -> Option<String> {
        let sel = Selector::parse(&format!("meta[property=\"og:{prop}\"]")).ok()?;
        doc.select(&sel)
            .next()?
            .value()
            .attr("content")
            .map(|s| s.to_string())
    };

    let meta_name = |name: &str| -> Option<String> {
        let sel = Selector::parse(&format!("meta[name=\"{name}\"]")).ok()?;
        doc.select(&sel)
            .next()?
            .value()
            .attr("content")
            .map(|s| s.to_string())
    };

    let title = og("title").or_else(|| {
        let sel = Selector::parse("title").ok()?;
        doc.select(&sel)
            .next()
            .map(|el| el.text().collect::<String>())
    });

    let description = og("description").or_else(|| meta_name("description"));
    let image = og("image");
    let site_name = og("site_name");

    let favicon = {
        let sel = Selector::parse("link[rel~=\"icon\"]").ok();
        sel.and_then(|s| {
            let href = doc.select(&s).next()?.value().attr("href")?;
            if href.starts_with("http") {
                Some(href.to_string())
            } else if href.starts_with("//") {
                Some(format!("https:{href}"))
            } else {
                // Build origin from the URL
                let origin = url.split('/').take(3).collect::<Vec<_>>().join("/");
                let sep = if href.starts_with('/') { "" } else { "/" };
                Some(format!("{origin}{sep}{href}"))
            }
        })
    };

    Ok(LinkPreview {
        title,
        description,
        image,
        site_name,
        favicon,
    })
}

/// Position the window based on the configured mode.
///
/// - `"mouse"` → spawn near cursor on the active monitor, clamped to screen edges
/// - `"fixed"` → center on the monitor that contains the cursor (or primary fallback)
fn position_window(window: &tauri::WebviewWindow) {
    let config = AppConfig::load();
    let w = config.ui.window_width as i32;
    let h = config.ui.window_height as i32;
    let mode = config.ui.window_position.as_str();

    if let Ok(cursor) = window.cursor_position() {
        if let Ok(monitors) = window.available_monitors() {
            for mon in monitors {
                let pos = mon.position();
                let size = mon.size();
                let right = pos.x + size.width as i32;
                let bottom = pos.y + size.height as i32;

                if (cursor.x as i32) >= pos.x
                    && (cursor.x as i32) < right
                    && (cursor.y as i32) >= pos.y
                    && (cursor.y as i32) < bottom
                {
                    let (x, y) = if mode == "mouse" {
                        // Place near cursor, offset slightly, clamp to screen edges
                        let raw_x = cursor.x as i32 - w / 2;
                        let raw_y = cursor.y as i32 + 20; // 20px below cursor
                        let cx = raw_x.max(pos.x + 8).min(right - w - 8);
                        let cy = raw_y.max(pos.y + 8).min(bottom - h - 8);
                        (cx, cy)
                    } else {
                        // "fixed" — center on the monitor
                        let cx = pos.x + (size.width as i32 - w) / 2;
                        let cy = pos.y + (size.height as i32 - h) / 2;
                        (cx, cy)
                    };

                    let _ = window
                        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                    return;
                }
            }
        }
    }

    // Fallback: center on current monitor
    let _ = window.center();
}

/// Refresh the tray menu with the latest clipboard items (count from config).
async fn refresh_tray_menu(app: &tauri::AppHandle) {
    use shared::models::IpcResponse;
    use tauri::menu::{MenuBuilder, MenuItemBuilder};

    let tray_limit = AppConfig::load().ui.tray_items.clamp(3, 15);
    let socket = AppConfig::socket_path();
    let request = IpcRequest::List {
        offset: 0,
        limit: tray_limit,
    };

    let items = match send_request(&socket, &request).await {
        Ok(IpcResponse::Items { items, .. }) => items,
        _ => return,
    };

    // Build the menu
    let Ok(show_item) = MenuItemBuilder::with_id("toggle", "📋 Show / Hide").build(app) else {
        return;
    };
    let Ok(quit_item) = MenuItemBuilder::with_id("quit", "❌ Quit").build(app) else {
        return;
    };

    let mut builder = MenuBuilder::new(app).item(&show_item).separator();

    for item in &items {
        let type_icon = match item.content_type.as_str() {
            "plain_text" => "📝",
            "html" => "🌐",
            "image" => "🖼️",
            _ => "📋",
        };

        // Truncate preview for menu
        let preview: String = item
            .preview_text
            .replace('\n', " ↵ ")
            .chars()
            .take(45)
            .collect();
        let label = format!("{} {}", type_icon, preview.trim());
        let menu_id = format!("paste_{}", item.id);

        if let Ok(mi) = MenuItemBuilder::with_id(menu_id, label).build(app) {
            builder = builder.item(&mi);
        }
    }

    let Ok(menu) = builder.separator().item(&quit_item).build() else {
        return;
    };

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_menu(Some(menu));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_items,
            search_items,
            search_items_regex,
            paste_item,
            paste_as_plain_text,
            paste_raw_text,
            type_text,
            pin_item,
            delete_item,
            bulk_delete,
            bulk_pin,
            list_snippets,
            search_snippets,
            create_snippet,
            update_snippet,
            delete_snippet,
            use_snippet,
            get_status,
            clear_all,
            get_config,
            save_config,
            get_image_base64,
            add_tag,
            remove_tag,
            fetch_gifs,
            fetch_gif_categories,
            register_gif_share,
            copy_gif,
            clear_gif_cache,
            get_app_version,
            check_for_updates,
            check_for_updates_via_plugin,
            install_update_via_plugin,
            download_update,
            install_update,
            generate_qr_code,
            convert_json_to_yaml,
            convert_yaml_to_json,
            convert_json_to_toml,
            convert_toml_to_json,
            highlight_code,
            detect_language,
            fetch_link_preview,
            extract_text_from_image,
            update_preview_text,
            set_launch_at_startup,
            is_launch_at_startup_enabled,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Clean up expired GIF cache files on startup
            std::thread::spawn(cleanup_expired_gif_cache);

            // On Windows, spawn clipd if not already running
            #[cfg(windows)]
            {
                let clipd_path = get_clipd_path();
                if clipd_path.exists() {
                    let pipe_check = std::path::Path::new(r"\\.\pipe\LinVClipBoard");
                    if !pipe_check.exists() {
                        match std::process::Command::new(&clipd_path).spawn() {
                            Ok(child) => {
                                std::mem::forget(child);
                                tracing::info!("Started clipd from GUI");
                            }
                            Err(e) => {
                                tracing::warn!("Failed to start clipd: {e}");
                            }
                        }
                    }
                } else {
                    tracing::warn!("clipd.exe not found at {:?}", clipd_path);
                }
            }

            // --- System Tray ---
            // NOTE: On Linux with AppIndicator, a menu is REQUIRED for the icon to appear.
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::TrayIconBuilder;

            // Build initial menu (will be refreshed with actual items shortly)
            let show_item = MenuItemBuilder::with_id("toggle", "📋 Show / Hide").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "❌ Quit").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
                tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
                    .expect("Failed to load tray icon")
            });

            let tray_window = window.clone();
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .menu(&tray_menu)
                .tooltip("LinVClipBoard")
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    let id = event.id().as_ref().to_string();
                    match id.as_str() {
                        "toggle" => {
                            if let Some(win) = app.get_webview_window("main") {
                                if let Ok(visible) = win.is_visible() {
                                    if visible {
                                        let _ = win.hide();
                                    } else {
                                        let _ = win.show();
                                        let _ = win.set_focus();
                                    }
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ if id.starts_with("paste_") => {
                            let item_id = id.strip_prefix("paste_").unwrap().to_string();
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let socket = AppConfig::socket_path();
                                let request = IpcRequest::Paste { id: item_id };
                                let _ = send_request(&socket, &request).await;
                                // Show window briefly for feedback, then hide
                                if let Some(win) = app_handle.get_webview_window("main") {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                                    let _ = win.hide();
                                }
                            });
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    // Fallback for non-AppIndicator DEs (e.g., KDE, Sway)
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { .. } = event {
                        if let Ok(visible) = tray_window.is_visible() {
                            if visible {
                                let _ = tray_window.hide();
                            } else {
                                let _ = tray_window.show();
                                let _ = tray_window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // --- Background task: refresh tray menu with latest 5 items ---
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    refresh_tray_menu(&app_handle).await;
                }
            });

            // --- Intercept window close → hide to tray instead of quitting ---
            let close_window = app.get_webview_window("main").unwrap();
            let close_win_ref = close_window.clone();
            close_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = close_win_ref.hide();
                }
            });

            // --- Global Shortcut: configurable (defaults to Ctrl+/) (#21) ---
            use tauri_plugin_global_shortcut::ShortcutState;

            let config = AppConfig::load();
            let shortcut_str = config.ui.shortcut.clone();

            let sc_win = app.get_webview_window("main").unwrap();
            if let Err(e) = app.global_shortcut().on_shortcut(
                shortcut_str.as_str(),
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Ok(visible) = sc_win.is_visible() {
                            if visible {
                                let _ = sc_win.hide();
                            } else {
                                position_window(&sc_win);
                                let _ = sc_win.show();
                                let _ = sc_win.set_focus();
                            }
                        }
                    }
                },
            ) {
                eprintln!(
                    "Note: Could not register shortcut '{}': {}",
                    shortcut_str, e
                );
            }

            // --- Background task: push clipboard-updated events to the UI (#13) ---
            let event_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_total: u64 = 0;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let socket = AppConfig::socket_path();
                    let req = IpcRequest::Status;
                    if let Ok(IpcResponse::Status { total_items, .. }) =
                        send_request(&socket, &req).await
                    {
                        if total_items != last_total {
                            last_total = total_items;
                            let _ = event_handle.emit("clipboard-updated", total_items);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LinVClipBoard");
}
