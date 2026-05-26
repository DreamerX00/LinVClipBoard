use arboard::Clipboard;
use sha2::{Digest, Sha256};
use shared::config::AppConfig;
use shared::db::Database;
use shared::models::{ClipboardItem, ContentType};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wl_clipboard_rs::paste::{
    get_contents as wl_get_contents, ClipboardType, MimeType as WlMimeType, Seat,
};

/// Detect the currently focused application name.
///
/// Tries X11 (`xdotool`) first, then common Wayland compositors
/// (`swaymsg`, `hyprctl`), and finally falls back to reading
/// `/proc/<pid>/comm` via `xdotool getactivewindow getwindowpid`.
fn get_active_app_name() -> Option<String> {
    // 1. X11 — xdotool window-class
    if let Ok(out) = Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
    {
        if out.status.success() {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    // 2. Sway / wlroots
    if let Ok(out) = Command::new("swaymsg").args(["-t", "get_tree"]).output() {
        if out.status.success() {
            // Parse the focused node's app_id from the JSON tree
            if let Ok(tree) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(name) = find_focused_app(&tree) {
                    return Some(name.to_lowercase());
                }
            }
        }
    }

    // 3. Hyprland
    if let Ok(out) = Command::new("hyprctl").arg("activewindow").output() {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(class) = trimmed.strip_prefix("class:") {
                    let name = class.trim().to_lowercase();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
    }

    None
}

/// Recursively find the `app_id` (Wayland) or `window_properties.class`
/// (XWayland) of the focused node in a Sway tree JSON blob.
fn find_focused_app(node: &serde_json::Value) -> Option<String> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        // Try Wayland-native app_id first
        if let Some(app_id) = node.get("app_id").and_then(|v| v.as_str()) {
            if !app_id.is_empty() {
                return Some(app_id.to_string());
            }
        }
        // XWayland windows have window_properties → class
        if let Some(class) = node
            .get("window_properties")
            .and_then(|wp| wp.get("class"))
            .and_then(|v| v.as_str())
        {
            if !class.is_empty() {
                return Some(class.to_string());
            }
        }
    }

    // Recurse into child nodes
    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(name) = find_focused_app(child) {
                return Some(name);
            }
        }
    }
    if let Some(nodes) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(name) = find_focused_app(child) {
                return Some(name);
            }
        }
    }

    None
}

/// Check if the given app name matches any entry in the blacklist
/// (case-insensitive substring match).
fn is_blacklisted(app_name: &str, blacklist: &[String]) -> bool {
    let lower = app_name.to_lowercase();
    blacklist.iter().any(|b| lower.contains(&b.to_lowercase()))
}

/// Run the clipboard monitor loop.
pub async fn run(
    db: Arc<Database>,
    config: Arc<AppConfig>,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let base_poll = Duration::from_millis(config.daemon.poll_interval_ms);
    let mut last_text_checksum = String::new();
    let mut last_image_checksum = String::new();

    // Track limit enforcement — run every 100 polls
    let mut poll_count: u64 = 0;

    // Adaptive polling: speed up when activity detected, slow down when idle
    let mut idle_streak: u64 = 0;

    // Consecutive failure counter for clipboard reconnect
    let mut consecutive_failures: u32 = 0;
    let mut backoff_secs: u64 = 1;
    const MAX_BACKOFF_SECS: u64 = 60;
    const RECONNECT_THRESHOLD: u32 = 3;

    // Create clipboard instance ONCE — avoids Wayland fallback warning spam.
    let mut clipboard = Clipboard::new()?;
    tracing::info!(
        "📋 Clipboard monitor started (polling every {}ms)",
        config.daemon.poll_interval_ms
    );

    loop {
        // Check for cancellation before each poll
        if cancel.is_cancelled() {
            tracing::info!("Monitor received cancellation, stopping");
            break;
        }

        let mut had_activity = false;

        if !config.security.incognito {
            // Check blacklist: skip capturing if focused app is blacklisted
            let active_app = get_active_app_name();
            if let Some(ref app) = active_app {
                if is_blacklisted(app, &config.security.blacklisted_apps) {
                    tracing::debug!("Skipping capture — app '{}' is blacklisted", app);
                    // Still sleep and continue the loop
                    let poll_interval = base_poll;
                    tokio::select! {
                        _ = tokio::time::sleep(poll_interval) => {}
                        _ = cancel.cancelled() => {
                            tracing::info!("Monitor cancelled during sleep");
                            break;
                        }
                    }
                    continue;
                }
            }

            let mut text_failed = false;
            let mut image_failed = false;

            // Try to capture text
            match capture_text(&mut clipboard, &last_text_checksum) {
                Ok(Some((mut item, checksum))) => {
                    item.app_source = active_app.clone();
                    match db.insert(&item) {
                        Ok(true) => {
                            last_text_checksum = checksum;
                            tracing::debug!(
                                "Captured text: {} chars (from {:?})",
                                item.size_bytes,
                                &item.app_source
                            );
                            had_activity = true;
                        }
                        Ok(false) => {
                            last_text_checksum = checksum;
                            tracing::debug!("Duplicate text skipped");
                        }
                        Err(e) => tracing::error!("DB insert error: {}", e),
                    }
                }
                Ok(None) => {} // No change
                Err(_) => {
                    text_failed = true;
                }
            }

            // Try to capture image
            match capture_image(&mut clipboard, &last_image_checksum, &config) {
                Ok(Some((mut item, checksum))) => {
                    item.app_source = active_app.clone();
                    match db.insert(&item) {
                        Ok(true) => {
                            last_image_checksum = checksum;
                            tracing::debug!(
                                "Captured image: {} bytes (from {:?})",
                                item.size_bytes,
                                &item.app_source
                            );
                            had_activity = true;
                        }
                        Ok(false) => {
                            last_image_checksum = checksum;
                        }
                        Err(e) => tracing::error!("DB insert error: {}", e),
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    image_failed = true;
                }
            }

            // Only count as failure if BOTH text and image capture failed
            // (one always "fails" when clipboard holds the other type).
            let had_failure = text_failed && image_failed;

            // Reconnect clipboard on consecutive failures
            if had_failure {
                consecutive_failures += 1;
                if consecutive_failures >= RECONNECT_THRESHOLD {
                    tracing::warn!(
                        "Clipboard failed {} times, reconnecting (backoff {}s)...",
                        consecutive_failures,
                        backoff_secs
                    );
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    match Clipboard::new() {
                        Ok(new_clip) => {
                            clipboard = new_clip;
                            consecutive_failures = 0;
                            backoff_secs = 1;
                            tracing::info!("Clipboard reconnected successfully");
                        }
                        Err(e) => {
                            tracing::error!("Clipboard reconnect failed: {}", e);
                            backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                        }
                    }
                }
            } else {
                consecutive_failures = 0;
                backoff_secs = 1;
            }
        }

        // Enforce storage limits periodically
        poll_count += 1;
        if poll_count.is_multiple_of(100) {
            if let Err(e) = db.enforce_limits(&config.storage) {
                tracing::error!("Limit enforcement error: {}", e);
            }
            // Clean up orphan blobs
            let blob_dir = AppConfig::blob_dir();
            if let Err(e) = db.cleanup_orphan_blobs(&blob_dir) {
                tracing::error!("Orphan blob cleanup error: {}", e);
            }
        }

        // Adaptive polling: use base interval when active, gradually slow down when idle
        if had_activity {
            idle_streak = 0;
        } else {
            idle_streak += 1;
        }
        let poll_interval = if idle_streak > 40 {
            // After ~10 seconds of inactivity, slow to 2x base interval
            base_poll * 2
        } else {
            base_poll
        };

        tokio::select! {
            _ = tokio::time::sleep(poll_interval) => {}
            _ = cancel.cancelled() => {
                tracing::info!("Monitor cancelled during sleep");
                break;
            }
        }
    }

    Ok(())
}

/// Try to get HTML content from clipboard via Wayland's wl-clipboard-rs.
/// Returns Some(html_string) if text/html MIME is available.
fn try_get_html_wayland() -> Option<String> {
    match wl_get_contents(
        ClipboardType::Regular,
        Seat::Unspecified,
        WlMimeType::Specific("text/html"),
    ) {
        Ok((mut reader, _)) => {
            let mut buf = String::new();
            if std::io::Read::read_to_string(&mut reader, &mut buf).is_ok()
                && !buf.trim().is_empty()
            {
                Some(buf)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Try to get file list from clipboard (gnome-copied-files / text/uri-list).
fn try_get_files_wayland() -> Option<Vec<String>> {
    // Try gnome-copied-files first (Nautilus uses this)
    let raw = match wl_get_contents(
        ClipboardType::Regular,
        Seat::Unspecified,
        WlMimeType::Specific("x-special/gnome-copied-files"),
    ) {
        Ok((mut reader, _)) => {
            let mut buf = String::new();
            if std::io::Read::read_to_string(&mut reader, &mut buf).is_ok() {
                Some(buf)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // Fallback to text/uri-list
    let raw = raw.or_else(|| {
        match wl_get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            WlMimeType::Specific("text/uri-list"),
        ) {
            Ok((mut reader, _)) => {
                let mut buf = String::new();
                if std::io::Read::read_to_string(&mut reader, &mut buf).is_ok() {
                    Some(buf)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    let raw = raw?;
    let paths: Vec<String> = raw
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        // Skip the "copy\n" / "cut\n" header from gnome-copied-files
        .filter(|l| *l != "copy" && *l != "cut")
        .map(|l| {
            // Convert file:// URIs to paths
            if let Some(path) = l.strip_prefix("file://") {
                // Decode percent-encoded characters
                urlish_decode(path)
            } else {
                l.to_string()
            }
        })
        .collect();

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

/// Simple percent-decode for file paths (decodes %XX sequences).
fn urlish_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

/// Attempt to capture text from clipboard. Returns Some if content changed.
/// Now detects rich content types: HTML > Files > URI > PlainText.
fn capture_text(
    clipboard: &mut Clipboard,
    last_checksum: &str,
) -> Result<Option<(ClipboardItem, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let text = clipboard.get_text()?;

    if text.trim().is_empty() {
        return Ok(None);
    }

    // Normalize text to prevent encoding-caused duplicates
    let normalized = text.replace("\r\n", "\n").trim().to_string();

    // Compute checksum on normalized text
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let checksum = hex::encode(hasher.finalize());

    if checksum == last_checksum {
        return Ok(None);
    }

    // Priority detection: HTML > Files > URI > PlainText
    // 1. Try HTML via Wayland
    if let Some(html) = try_get_html_wayland() {
        let preview = html2text::from_read(html.as_bytes(), 200)
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>()
            .trim()
            .to_string();
        let preview = if preview.is_empty() {
            normalized.chars().take(200).collect::<String>()
        } else {
            preview
        };
        let size = html.len() as u64;
        let item = ClipboardItem::new(ContentType::Html, html, preview, checksum.clone(), size);
        return Ok(Some((item, checksum)));
    }

    // 2. Try file list via Wayland
    if let Some(files) = try_get_files_wayland() {
        let content = serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string());
        let preview = if files.len() <= 3 {
            files
                .iter()
                .map(|f| {
                    std::path::Path::new(f)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| f.clone())
                })
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let first3: Vec<_> = files[..3]
                .iter()
                .map(|f| {
                    std::path::Path::new(f)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| f.clone())
                })
                .collect();
            format!("{} +{} more", first3.join(", "), files.len() - 3)
        };
        let size = content.len() as u64;
        let item = ClipboardItem::new(ContentType::Files, content, preview, checksum.clone(), size);
        return Ok(Some((item, checksum)));
    }

    // 3. Check for single URI
    let trimmed = normalized.trim();
    if (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        && !trimmed.contains('\n')
        && trimmed.len() < 2048
    {
        // Extract domain from URL for preview
        let preview = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .and_then(|s| s.split('/').next())
            .unwrap_or(trimmed)
            .to_string();
        let size = normalized.len() as u64;
        let item = ClipboardItem::new(
            ContentType::Uri,
            normalized,
            preview,
            checksum.clone(),
            size,
        );
        return Ok(Some((item, checksum)));
    }

    // 4. Plain text fallback
    let preview = normalized.chars().take(200).collect::<String>();
    let size = normalized.len() as u64;

    let item = ClipboardItem::new(
        ContentType::PlainText,
        normalized,
        preview,
        checksum.clone(),
        size,
    );

    Ok(Some((item, checksum)))
}

/// Attempt to capture an image from clipboard. Returns Some if content changed.
fn capture_image(
    clipboard: &mut Clipboard,
    last_checksum: &str,
    config: &AppConfig,
) -> Result<Option<(ClipboardItem, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let img_data = clipboard.get_image()?;

    let raw_bytes = &img_data.bytes;
    if raw_bytes.is_empty() {
        return Ok(None);
    }

    // Compute checksum of raw image bytes
    let mut hasher = Sha256::new();
    hasher.update(raw_bytes.as_ref());
    let checksum = hex::encode(hasher.finalize());

    if checksum == last_checksum {
        return Ok(None);
    }

    // Check size limit
    let size = raw_bytes.len() as u64;
    if size > config.storage.max_item_size_bytes {
        tracing::warn!(
            "Image too large: {} bytes (limit: {})",
            size,
            config.storage.max_item_size_bytes
        );
        return Ok(None);
    }

    // Save as PNG file in blob directory
    let blob_dir = AppConfig::blob_dir();
    std::fs::create_dir_all(&blob_dir)?;
    let filename = format!("{}.png", Uuid::new_v4());
    let blob_path = blob_dir.join(&filename);

    // Create image buffer from raw RGBA data
    let img_buf = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        img_data.width as u32,
        img_data.height as u32,
        raw_bytes.to_vec(),
    );

    if let Some(buf) = img_buf {
        buf.save(&blob_path)?;
    } else {
        // Fallback: save raw bytes
        std::fs::write(&blob_path, raw_bytes.as_ref())?;
    }

    let preview = format!("Image {}x{}", img_data.width, img_data.height);
    let content = blob_path.to_string_lossy().to_string();

    let item = ClipboardItem::new(ContentType::Image, content, preview, checksum.clone(), size);

    Ok(Some((item, checksum)))
}
