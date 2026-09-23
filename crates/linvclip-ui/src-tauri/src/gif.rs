//! GIF search (KLIPY): provider settings, HTTP calls and the Tauri commands.
//!
//! The API key is **not** compiled into the binary. It is resolved at runtime:
//!
//! 1. The user's own key: `[gif] api_key` in `config.toml`, or the
//!    `KLIPY_API_KEY` environment variable.
//! 2. Otherwise the project's `gif-provider.json` (root of the repository, on
//!    `main`), downloaded on first use and cached on disk for [`PROVIDER_TTL`].
//!    Editing that one file rotates the key for every installed version, with
//!    no rebuild and no release.
//!
//! When KLIPY rejects the key, the file is re-downloaded and the request is
//! retried once with whatever it now contains, so a rotation takes effect on
//! the very next request instead of when the cache expires.
//!
//! Every error returned to the frontend is either one of the `GIF_*` codes
//! below (mapped to a localized message in `GifPicker.jsx`) or a short text
//! that never contains a URL — the KLIPY key is part of every request URL and
//! must never end up on screen.

use serde::{Deserialize, Serialize};
use shared::config::{AppConfig, GifConfig};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// No key configured anywhere (empty `api_key` in `gif-provider.json`).
pub const GIF_API_KEY_MISSING: &str = "gif_api_key_missing";
/// KLIPY rejected the key, and re-downloading `gif-provider.json` did not help.
pub const GIF_API_KEY_INVALID: &str = "gif_api_key_invalid";
/// Could not reach KLIPY or GitHub (DNS, connect, timeout).
pub const GIF_NETWORK_ERROR: &str = "gif_network_error";

pub const DEFAULT_BASE_URL: &str = "https://api.klipy.com/api/v1";

/// Where `gif-provider.json` is downloaded from, in order. Both are plain CDN
/// files with no API rate limit; jsDelivr is the mirror for when GitHub's raw
/// host is blocked (same fallback as `install.sh`).
pub const PROVIDER_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/DreamerX00/LinVClipBoard/main/gif-provider.json",
    "https://cdn.jsdelivr.net/gh/DreamerX00/LinVClipBoard@main/gif-provider.json",
];

/// How long a downloaded `gif-provider.json` is trusted before it is refreshed.
pub const PROVIDER_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// Minimum gap between two download attempts, so a broken key or an outage
/// does not turn every GIF request into a GitHub request as well.
pub const PROVIDER_RETRY_INTERVAL: Duration = Duration::from_secs(60);
/// Per-request timeout. Without one a stalled connection keeps the GIF tab's
/// spinner up indefinitely.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CUSTOMER_ID: &str = "linvclipboard_user";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GifItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub preview_url: String,
    pub gif_url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GifResult {
    pub items: Vec<GifItem>,
    pub page: u32,
    pub has_next: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GifCategory {
    pub category: String,
    pub query: String,
    pub preview_url: String,
}

/// Where and with which key GIF requests are made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GifProvider {
    /// API root without a trailing slash, e.g. `https://api.klipy.com/api/v1`.
    pub base_url: String,
    /// KLIPY app key; empty when none is configured.
    pub api_key: String,
}

impl GifProvider {
    /// Parse `gif-provider.json`. Only `api_key` and `base_url` are read;
    /// unknown fields are ignored so the file can grow without breaking
    /// installed versions.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| format!("gif-provider.json is not valid JSON: {}", e))?;
        let obj = value
            .as_object()
            .ok_or("gif-provider.json is not a JSON object")?;
        let api_key = obj
            .get("api_key")
            .and_then(|k| k.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let base_url = obj
            .get("base_url")
            .and_then(|u| u.as_str())
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .unwrap_or(DEFAULT_BASE_URL);
        Ok(Self {
            base_url: normalize_base_url(base_url),
            api_key,
        })
    }

    /// `{base_url}/{api_key}/{path}` — KLIPY puts the app key in the path.
    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}/{}",
            self.base_url,
            self.api_key,
            path.trim_start_matches('/')
        )
    }
}

fn normalize_base_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// On-disk copy of the last downloaded `gif-provider.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CachedProvider {
    /// Unix seconds of the last successful download.
    fetched_at: u64,
    /// Unix seconds of the last download *attempt* (throttles retries).
    checked_at: u64,
    provider: GifProvider,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(path: &Path) -> Option<CachedProvider> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_cache(path: &Path, cache: &CachedProvider) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(cache) {
        let _ = std::fs::write(path, text);
    }
}

/// Errors from a single KLIPY request, before they are turned into strings.
#[derive(Debug)]
enum ApiError {
    /// KLIPY says the key is invalid — the caller may refresh the provider
    /// file and retry.
    KeyRejected,
    /// DNS / connect / timeout.
    Network,
    /// Anything else, already safe to show (no URL in it).
    Other(String),
}

impl ApiError {
    fn into_message(self) -> String {
        match self {
            ApiError::KeyRejected => GIF_API_KEY_INVALID.to_string(),
            ApiError::Network => GIF_NETWORK_ERROR.to_string(),
            ApiError::Other(m) => m,
        }
    }
}

/// Classify a reqwest error and strip the URL (it contains the API key).
fn classify_reqwest(e: reqwest::Error) -> ApiError {
    if e.is_timeout() || e.is_connect() || e.is_request() {
        ApiError::Network
    } else {
        ApiError::Other(format!("HTTP request failed: {}", e.without_url()))
    }
}

/// KLIPY error payload: `{"result": false, "errors": {"message": ["…"]}}`.
fn api_message(body: &serde_json::Value) -> Option<String> {
    let msg = &body["errors"]["message"];
    if let Some(list) = msg.as_array() {
        let parts: Vec<&str> = list.iter().filter_map(|m| m.as_str()).collect();
        if !parts.is_empty() {
            return Some(parts.join("; "));
        }
    }
    msg.as_str().map(str::to_string)
}

/// KLIPY answers an invalid key with HTTP 404 + "The provided API key is
/// invalid." (observed 2026-09), not 401 — so look at the message as well.
fn is_key_rejected(status: u16, body: &serde_json::Value) -> bool {
    if body["result"].as_bool() != Some(false) {
        return false;
    }
    if status == 401 || status == 403 {
        return true;
    }
    api_message(body).is_some_and(|m| m.to_ascii_lowercase().contains("api key"))
}

/// Resolves the provider (user override → downloaded file → cache) and makes
/// the HTTP calls. Cheap to construct; the commands build one per call so
/// config edits take effect without a restart.
pub struct GifService {
    client: reqwest::Client,
    /// From `config.toml` / `KLIPY_API_KEY`; when set, the remote file is never
    /// consulted.
    user_provider: Option<GifProvider>,
    /// `[gif] base_url`, applied on top of whichever provider is used.
    base_url_override: Option<String>,
    provider_urls: Vec<String>,
    cache_path: PathBuf,
    pub provider_ttl: Duration,
    pub retry_interval: Duration,
}

impl GifService {
    /// Build from `config.toml` and the environment.
    pub fn from_environment() -> Result<Self, String> {
        let cfg = load_gif_config();
        let non_empty =
            |s: Option<&String>| s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let env_key = std::env::var("KLIPY_API_KEY").ok();
        let api_key = non_empty(cfg.api_key.as_ref()).or_else(|| non_empty(env_key.as_ref()));
        let base_url_override = non_empty(cfg.base_url.as_ref()).map(|u| normalize_base_url(&u));
        let user_provider = api_key.map(|api_key| GifProvider {
            base_url: base_url_override
                .clone()
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key,
        });
        let provider_urls = match non_empty(cfg.provider_url.as_ref()) {
            Some(url) => vec![url],
            None => PROVIDER_URLS.iter().map(|s| s.to_string()).collect(),
        };
        let mut service = Self::new(user_provider, provider_urls, provider_cache_path())?;
        service.base_url_override = base_url_override;
        Ok(service)
    }

    pub fn new(
        user_provider: Option<GifProvider>,
        provider_urls: Vec<String>,
        cache_path: PathBuf,
    ) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("LinVClipBoard/", env!("CARGO_PKG_VERSION")))
            // KLIPY's edge has been seen to stall HTTP/2 GETs for 15 s or more
            // while HTTP/1.1 answers the same request in under half a second.
            // reqwest would negotiate h2 via ALPN, so pin HTTP/1.1.
            .http1_only()
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        Ok(Self {
            client,
            user_provider,
            base_url_override: None,
            provider_urls,
            cache_path,
            provider_ttl: PROVIDER_TTL,
            retry_interval: PROVIDER_RETRY_INTERVAL,
        })
    }

    fn apply_override(&self, mut provider: GifProvider) -> GifProvider {
        if let Some(base) = &self.base_url_override {
            provider.base_url = base.clone();
        }
        provider
    }

    /// The provider to use right now. `force_refresh` bypasses the TTL (but
    /// not the retry throttle) — used after KLIPY rejects the current key.
    pub async fn provider(&self, force_refresh: bool) -> Result<GifProvider, String> {
        if let Some(p) = &self.user_provider {
            return Ok(self.apply_override(p.clone()));
        }

        let now = unix_now();
        let cached = read_cache(&self.cache_path);
        let stale = cached.as_ref().is_none_or(|c| {
            force_refresh || now.saturating_sub(c.fetched_at) >= self.provider_ttl.as_secs()
        });
        let throttled = cached
            .as_ref()
            .is_some_and(|c| now.saturating_sub(c.checked_at) < self.retry_interval.as_secs());

        if stale && !throttled {
            match self.download_provider().await {
                Ok(provider) => {
                    write_cache(
                        &self.cache_path,
                        &CachedProvider {
                            fetched_at: now,
                            checked_at: now,
                            provider: provider.clone(),
                        },
                    );
                    return Ok(self.apply_override(provider));
                }
                Err(e) => match cached {
                    // Offline or GitHub down: keep using what we have.
                    Some(mut c) => {
                        c.checked_at = now;
                        write_cache(&self.cache_path, &c);
                        return Ok(self.apply_override(c.provider));
                    }
                    None => return Err(e),
                },
            }
        }

        match cached {
            Some(c) => Ok(self.apply_override(c.provider)),
            None => Err(GIF_NETWORK_ERROR.to_string()),
        }
    }

    async fn download_provider(&self) -> Result<GifProvider, String> {
        let mut last_error = GIF_NETWORK_ERROR.to_string();
        for url in &self.provider_urls {
            let text = match self.client.get(url).send().await {
                Ok(resp) => match resp.error_for_status() {
                    Ok(resp) => resp.text().await.map_err(|e| e.without_url().to_string()),
                    Err(e) => Err(format!("HTTP {}", e.status().map_or(0, |s| s.as_u16()))),
                },
                Err(e) => Err(match classify_reqwest(e) {
                    ApiError::Network => GIF_NETWORK_ERROR.to_string(),
                    other => other.into_message(),
                }),
            };
            match text.and_then(|t| GifProvider::from_json(&t)) {
                Ok(provider) => return Ok(provider),
                Err(e) => last_error = e,
            }
        }
        if last_error == GIF_NETWORK_ERROR {
            Err(last_error)
        } else {
            Err(format!("GIF provider settings unavailable: {}", last_error))
        }
    }

    /// GET `{base}/{key}/{path}` and return the parsed JSON body. Handles a
    /// rotated key transparently: on rejection, re-download the provider file
    /// and retry once with the new key.
    pub async fn get_json(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, String> {
        let mut provider = self.provider(false).await?;
        if provider.api_key.is_empty() {
            // The file may just have been fixed: re-check (throttled) first.
            provider = self.provider(true).await?;
            if provider.api_key.is_empty() {
                return Err(GIF_API_KEY_MISSING.to_string());
            }
        }

        match self.request(&provider, path, params).await {
            Err(ApiError::KeyRejected) => {
                let fresh = self.provider(true).await?;
                if fresh != provider && !fresh.api_key.is_empty() {
                    self.request(&fresh, path, params)
                        .await
                        .map_err(ApiError::into_message)
                } else {
                    Err(GIF_API_KEY_INVALID.to_string())
                }
            }
            other => other.map_err(ApiError::into_message),
        }
    }

    async fn request(
        &self,
        provider: &GifProvider,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, ApiError> {
        let resp = self
            .client
            .get(provider.endpoint(path))
            .query(params)
            .send()
            .await
            .map_err(classify_reqwest)?;
        let status = resp.status();
        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) if !status.is_success() => {
                return Err(ApiError::Other(format!("API error: {}", status)))
            }
            Err(e) => {
                return Err(ApiError::Other(format!(
                    "Failed to parse response: {}",
                    e.without_url()
                )))
            }
        };
        if is_key_rejected(status.as_u16(), &body) {
            return Err(ApiError::KeyRejected);
        }
        if !status.is_success() {
            return Err(ApiError::Other(format!("API error: {}", status)));
        }
        if body["result"].as_bool() != Some(true) {
            let detail = api_message(&body)
                .map(|m| format!(": {}", m))
                .unwrap_or_default();
            return Err(ApiError::Other(format!("API returned error{}", detail)));
        }
        Ok(body)
    }

    /// Fire-and-forget POST (share analytics). Failures are ignored.
    pub async fn post_json(&self, path: &str, body: serde_json::Value) {
        if let Ok(provider) = self.provider(false).await {
            if provider.api_key.is_empty() {
                return;
            }
            let _ = self
                .client
                .post(provider.endpoint(path))
                .json(&body)
                .send()
                .await;
        }
    }
}

/// `[gif]` from `config.toml`, without creating the file (that is clipd's job)
/// and without failing on unrelated sections.
fn load_gif_config() -> GifConfig {
    #[derive(Deserialize, Default)]
    struct GifOnly {
        #[serde(default)]
        gif: GifConfig,
    }
    std::fs::read_to_string(AppConfig::config_path())
        .ok()
        .and_then(|text| toml::from_str::<GifOnly>(&text).ok())
        .map(|c| c.gif)
        .unwrap_or_default()
}

fn linvclip_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("linvclip")
}

fn provider_cache_path() -> PathBuf {
    linvclip_cache_dir().join("gif-provider.json")
}

/// Return the GIF cache directory, creating it if needed.
pub fn gif_cache_dir() -> Result<PathBuf, String> {
    let dir = linvclip_cache_dir().join("gifs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;
    Ok(dir)
}

// ── Response parsing ─────────────────────────────────────────────────────

/// Parse a KLIPY v1 GIF object.
fn parse_gif_item(r: &serde_json::Value) -> Option<GifItem> {
    let id = r["id"]
        .as_i64()
        .or_else(|| r["id"].as_u64().map(|v| v as i64))?;
    let slug = r["slug"].as_str().unwrap_or("").to_string();
    let title = r["title"].as_str().unwrap_or("").to_string();

    // Prefer sm.webp (fast, small) → sm.gif → xs.gif for the preview;
    // hd.gif (→ md.gif) is what users copy.
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

fn parse_gif_result(body: &serde_json::Value, requested_page: u32) -> Result<GifResult, String> {
    let data = &body["data"];
    let has_next = data["has_next"].as_bool().unwrap_or(false);
    let page = data["current_page"]
        .as_u64()
        .unwrap_or(requested_page as u64) as u32;
    let results = data["data"].as_array().ok_or("No data array in response")?;
    Ok(GifResult {
        items: results.iter().filter_map(parse_gif_item).collect(),
        page,
        has_next,
    })
}

fn parse_categories(body: &serde_json::Value) -> Result<Vec<GifCategory>, String> {
    let cats = body["data"]["categories"]
        .as_array()
        .ok_or("No categories in response")?;
    Ok(cats
        .iter()
        .filter_map(|c| {
            Some(GifCategory {
                category: c["category"].as_str()?.to_string(),
                query: c["query"].as_str()?.to_string(),
                preview_url: c["preview_url"].as_str()?.to_string(),
            })
        })
        .collect())
}

// ── Tauri commands ───────────────────────────────────────────────────────

/// Fetch GIFs: trending when `query` is empty, otherwise a search.
#[tauri::command]
pub async fn fetch_gifs(query: String, page: u32, per_page: u32) -> Result<GifResult, String> {
    let service = GifService::from_environment()?;
    let is_search = !query.trim().is_empty();
    let path = if is_search {
        "gifs/search"
    } else {
        "gifs/trending"
    };
    let mut params: Vec<(&str, String)> = vec![
        ("page", page.to_string()),
        ("per_page", per_page.to_string()),
        ("customer_id", CUSTOMER_ID.to_string()),
        ("content_filter", "medium".to_string()),
        ("format_filter", "gif,webp,jpg".to_string()),
    ];
    if is_search {
        params.push(("q", query));
    }
    let body = service.get_json(path, &params).await?;
    parse_gif_result(&body, page)
}

/// Fetch the GIF categories shown on the GIF tab's home view.
#[tauri::command]
pub async fn fetch_gif_categories() -> Result<Vec<GifCategory>, String> {
    let service = GifService::from_environment()?;
    let body = service.get_json("gifs/categories", &[]).await?;
    parse_categories(&body)
}

/// Tell KLIPY a GIF was shared (their terms ask for it). Best effort.
#[tauri::command]
pub async fn register_gif_share(slug: String, query: String) -> Result<String, String> {
    let service = GifService::from_environment()?;
    let mut body = serde_json::Map::new();
    body.insert(
        "customer_id".to_string(),
        serde_json::Value::String(CUSTOMER_ID.to_string()),
    );
    if !query.is_empty() {
        body.insert("q".to_string(), serde_json::Value::String(query));
    }
    service
        .post_json(
            &format!("gifs/share/{}", slug),
            serde_json::Value::Object(body),
        )
        .await;
    Ok("ok".to_string())
}

/// Copy a GIF URL to the system clipboard.
///
/// Desktop GIF keyboards work by copying the direct GIF URL as plain text:
/// chat apps (Discord, Telegram, Slack, …) auto-embed direct `.gif` links.
/// `image/gif` is not accepted by most paste targets and `wl-clipboard`
/// cannot offer several MIME types at once, so the URL is the standard.
#[tauri::command]
pub async fn copy_gif(url: String) -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&url)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;
    Ok("ok".to_string())
}

/// Purge all cached GIF files.
#[tauri::command]
pub async fn clear_gif_cache() -> Result<String, String> {
    if let Ok(dir) = gif_cache_dir() {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to clear GIF cache: {}", e))?;
            std::fs::create_dir_all(&dir).ok();
        }
    }
    Ok("ok".to_string())
}

/// Purge GIF cache files older than the configured expiry. Runs at startup.
pub fn cleanup_expired_gif_cache() {
    let expiry_days = AppConfig::load().storage.expiry_days;
    let max_age = Duration::from_secs(expiry_days as u64 * 86400);

    let Ok(dir) = gif_cache_dir() else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };

    let now = SystemTime::now();
    for entry in entries.flatten() {
        let age = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|m| now.duration_since(m).ok());
        if age.is_some_and(|a| a > max_age) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // ── helpers ──

    static CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A unique, non-existent cache file per test.
    fn temp_cache() -> PathBuf {
        let n = CACHE_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "linvclip-gif-test-{}-{}.json",
            std::process::id(),
            n
        ))
    }

    /// Minimal HTTP/1.1 server. The handler gets the request path (no query
    /// string) and returns `(status, json body)`; every request line is logged.
    struct MockServer {
        base: String,
        listener: Option<TcpListener>,
        requests: Arc<Mutex<Vec<String>>>,
    }

    impl MockServer {
        async fn bind() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            Self {
                base,
                listener: Some(listener),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn serve(&mut self, handler: impl Fn(&str) -> (u16, String) + Send + Sync + 'static) {
            let listener = self.listener.take().expect("serve() called twice");
            let handler = Arc::new(handler);
            let log = self.requests.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((mut sock, _)) = listener.accept().await else {
                        break;
                    };
                    let handler = handler.clone();
                    let log = log.clone();
                    tokio::spawn(async move {
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 1024];
                        loop {
                            match sock.read(&mut chunk).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            }
                            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        let head = String::from_utf8_lossy(&buf);
                        let mut first = head.lines().next().unwrap_or("").split(' ');
                        let method = first.next().unwrap_or("").to_string();
                        let target = first.next().unwrap_or("").to_string();
                        let path = target.split('?').next().unwrap_or("").to_string();
                        log.lock().unwrap().push(format!("{} {}", method, target));
                        let (status, body) = handler(&path);
                        let resp = format!(
                            "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            status,
                            body.len(),
                            body
                        );
                        let _ = sock.write_all(resp.as_bytes()).await;
                        let _ = sock.shutdown().await;
                    });
                }
            });
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        fn count(&self, path_prefix: &str) -> usize {
            self.requests()
                .iter()
                .filter(|r| r.split(' ').nth(1).unwrap_or("").starts_with(path_prefix))
                .count()
        }
    }

    fn provider_json(key: &str, base_url: &str) -> String {
        json!({ "schema": 1, "provider": "klipy", "base_url": base_url, "api_key": key })
            .to_string()
    }

    fn categories_ok() -> (u16, String) {
        (
            200,
            json!({ "result": true, "data": { "categories": [
                { "category": "Cats", "query": "cats", "preview_url": "https://static.example/cats.webp" }
            ] } })
            .to_string(),
        )
    }

    /// What KLIPY actually returns for a bad key (observed 2026-09).
    fn key_invalid() -> (u16, String) {
        (
            404,
            json!({ "result": false, "errors": { "message": ["The provided API key is invalid."] } })
                .to_string(),
        )
    }

    fn service(provider_urls: Vec<String>, cache: &Path) -> GifService {
        let mut s = GifService::new(None, provider_urls, cache.to_path_buf()).unwrap();
        s.retry_interval = Duration::ZERO; // tests that need the throttle set it back
        s
    }

    // ── pure parsing ──

    #[test]
    fn provider_json_parsing() {
        let p = GifProvider::from_json(
            r#"{"schema":1,"provider":"klipy","base_url":"https://api.klipy.com/api/v1/","api_key":" k1 ","future_field":true}"#,
        )
        .unwrap();
        assert_eq!(p.base_url, "https://api.klipy.com/api/v1"); // trailing slash trimmed
        assert_eq!(p.api_key, "k1"); // whitespace trimmed
        assert_eq!(
            p.endpoint("/gifs/trending"),
            "https://api.klipy.com/api/v1/k1/gifs/trending"
        );

        // base_url is optional; api_key may be empty (= not configured yet)
        let p = GifProvider::from_json(r#"{"api_key":""}"#).unwrap();
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
        assert!(p.api_key.is_empty());
        let p = GifProvider::from_json("{}").unwrap();
        assert!(p.api_key.is_empty());

        assert!(GifProvider::from_json("not json").is_err());
        assert!(GifProvider::from_json("[1,2]").is_err());
    }

    #[test]
    fn shipped_provider_file_is_valid() {
        // The file at the repository root is what every installed app reads.
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../gif-provider.json"
        ))
        .expect("gif-provider.json at the repo root");
        let p = GifProvider::from_json(&text).unwrap();
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn key_rejection_detection() {
        let (status, body) = key_invalid();
        assert!(is_key_rejected(
            status,
            &serde_json::from_str(&body).unwrap()
        ));
        assert!(is_key_rejected(401, &json!({ "result": false })));
        assert!(is_key_rejected(
            403,
            &json!({ "result": false, "errors": {} })
        ));
        // Other failures are not key problems
        assert!(!is_key_rejected(
            404,
            &json!({ "result": false, "errors": { "message": ["Not found"] } })
        ));
        assert!(!is_key_rejected(500, &json!({})));
        assert!(!is_key_rejected(200, &json!({ "result": true })));
    }

    #[test]
    fn gif_result_and_category_parsing() {
        let body = json!({ "result": true, "data": { "current_page": 2, "has_next": true, "data": [
            { "id": 42, "slug": "hello", "title": "Hello", "file": {
                "sm": { "webp": { "url": "https://s/sm.webp", "width": 200, "height": 150 } },
                "hd": { "gif": { "url": "https://s/hd.gif" } } } },
            { "id": 43, "file": { "xs": { "gif": { "url": "https://s/xs.gif" } } } },
            { "title": "no id, dropped" }
        ] } });
        let r = parse_gif_result(&body, 1).unwrap();
        assert_eq!(r.page, 2);
        assert!(r.has_next);
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.items[0].id, "42");
        assert_eq!(r.items[0].preview_url, "https://s/sm.webp");
        assert_eq!(r.items[0].gif_url, "https://s/hd.gif");
        assert_eq!((r.items[0].width, r.items[0].height), (200, 150));
        assert_eq!(r.items[1].gif_url, "https://s/xs.gif"); // falls back to the preview
        assert_eq!((r.items[1].width, r.items[1].height), (220, 220));

        assert!(parse_gif_result(&json!({ "result": true, "data": {} }), 1).is_err());

        let (_, cats) = categories_ok();
        let cats = parse_categories(&serde_json::from_str(&cats).unwrap()).unwrap();
        assert_eq!(
            cats,
            vec![GifCategory {
                category: "Cats".into(),
                query: "cats".into(),
                preview_url: "https://static.example/cats.webp".into(),
            }]
        );
        assert!(parse_categories(&json!({ "result": true })).is_err());
    }

    // ── provider resolution over HTTP ──

    #[tokio::test]
    async fn remote_provider_is_downloaded_once_and_cached() {
        let mut server = MockServer::bind().await;
        let base = server.base.clone();
        server.serve(move |path| match path {
            "/gif-provider.json" => (200, provider_json("k1", &format!("{}/api", base))),
            "/api/k1/gifs/categories" => categories_ok(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let svc = service(vec![format!("{}/gif-provider.json", server.base)], &cache);

        let body = svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(body["result"], json!(true));
        svc.get_json("gifs/categories", &[]).await.unwrap();

        assert_eq!(
            server.count("/gif-provider.json"),
            1,
            "second call must use the cache"
        );
        assert_eq!(server.count("/api/k1/gifs/categories"), 2);
        let cached = read_cache(&cache).expect("cache written");
        assert_eq!(cached.provider.api_key, "k1");
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn rotated_key_is_picked_up_without_a_rebuild() {
        let current_key = Arc::new(Mutex::new("old-key".to_string()));
        let mut server = MockServer::bind().await;
        let base = server.base.clone();
        let key = current_key.clone();
        server.serve(move |path| match path {
            "/gif-provider.json" => (
                200,
                provider_json(&key.lock().unwrap(), &format!("{}/api", base)),
            ),
            "/api/new-key/gifs/categories" => categories_ok(),
            p if p.starts_with("/api/") => key_invalid(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let svc = service(vec![format!("{}/gif-provider.json", server.base)], &cache);

        // 1. The published key is bad and re-downloading does not change it.
        let err = svc.get_json("gifs/categories", &[]).await.unwrap_err();
        assert_eq!(err, GIF_API_KEY_INVALID);
        assert_eq!(
            server.count("/gif-provider.json"),
            2,
            "download + one forced refresh"
        );
        assert_eq!(
            server.count("/api/old-key/"),
            1,
            "no blind retry with the same key"
        );

        // 2. Maintainer edits gif-provider.json on main. The cache is still
        //    fresh, so the next request goes out with the old key, gets
        //    rejected, refreshes, and succeeds with the new key.
        *current_key.lock().unwrap() = "new-key".to_string();
        let body = svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(body["result"], json!(true));
        assert_eq!(server.count("/gif-provider.json"), 3);
        assert_eq!(server.count("/api/new-key/"), 1);
        assert_eq!(read_cache(&cache).unwrap().provider.api_key, "new-key");

        // 3. From now on the new key is used directly.
        svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(server.count("/gif-provider.json"), 3);
        assert_eq!(server.count("/api/new-key/"), 2);
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn refreshes_are_throttled() {
        let current_key = Arc::new(Mutex::new("old-key".to_string()));
        let mut server = MockServer::bind().await;
        let base = server.base.clone();
        let key = current_key.clone();
        server.serve(move |path| match path {
            "/gif-provider.json" => (
                200,
                provider_json(&key.lock().unwrap(), &format!("{}/api", base)),
            ),
            "/api/new-key/gifs/categories" => categories_ok(),
            p if p.starts_with("/api/") => key_invalid(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let mut svc = service(vec![format!("{}/gif-provider.json", server.base)], &cache);
        svc.retry_interval = PROVIDER_RETRY_INTERVAL;

        assert_eq!(
            svc.get_json("gifs/categories", &[]).await.unwrap_err(),
            GIF_API_KEY_INVALID
        );
        assert_eq!(
            server.count("/gif-provider.json"),
            1,
            "just downloaded: forced refresh is skipped"
        );

        // Rotation within the throttle window is not seen yet…
        *current_key.lock().unwrap() = "new-key".to_string();
        assert_eq!(
            svc.get_json("gifs/categories", &[]).await.unwrap_err(),
            GIF_API_KEY_INVALID
        );
        assert_eq!(server.count("/gif-provider.json"), 1);

        // …but is once the window has passed.
        let mut c = read_cache(&cache).unwrap();
        c.checked_at -= PROVIDER_RETRY_INTERVAL.as_secs() + 1;
        write_cache(&cache, &c);
        svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(server.count("/gif-provider.json"), 2);
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn stale_cache_is_used_when_github_is_unreachable() {
        let mut api = MockServer::bind().await;
        api.serve(|path| match path {
            "/api/cached-key/gifs/trending" => (
                200,
                json!({ "result": true, "data": { "data": [] } }).to_string(),
            ),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let long_ago = unix_now() - 2 * PROVIDER_TTL.as_secs();
        write_cache(
            &cache,
            &CachedProvider {
                fetched_at: long_ago,
                checked_at: long_ago,
                provider: GifProvider {
                    base_url: format!("{}/api", api.base),
                    api_key: "cached-key".into(),
                },
            },
        );
        // Port 1 refuses connections immediately.
        let svc = service(vec!["http://127.0.0.1:1/gif-provider.json".into()], &cache);

        svc.get_json("gifs/trending", &[]).await.unwrap();
        assert_eq!(api.count("/api/cached-key/gifs/trending"), 1);
        assert!(
            read_cache(&cache).unwrap().checked_at > long_ago,
            "attempt recorded"
        );

        // With no cache at all there is nothing to fall back to.
        let svc = service(
            vec!["http://127.0.0.1:1/gif-provider.json".into()],
            &temp_cache(),
        );
        assert_eq!(
            svc.get_json("gifs/trending", &[]).await.unwrap_err(),
            GIF_NETWORK_ERROR
        );
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn second_provider_url_is_a_fallback() {
        let mut server = MockServer::bind().await;
        let base = server.base.clone();
        server.serve(move |path| match path {
            "/mirror/gif-provider.json" => (200, provider_json("k2", &format!("{}/api", base))),
            "/api/k2/gifs/categories" => categories_ok(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let svc = service(
            vec![
                "http://127.0.0.1:1/gif-provider.json".into(),
                format!("{}/mirror/gif-provider.json", server.base),
            ],
            &cache,
        );
        svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(server.count("/mirror/gif-provider.json"), 1);
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn empty_key_in_provider_file_reports_missing() {
        let mut server = MockServer::bind().await;
        let base = server.base.clone();
        server.serve(move |path| match path {
            "/gif-provider.json" => (200, provider_json("", &format!("{}/api", base))),
            _ => (404, "{}".into()),
        });
        let svc = service(
            vec![format!("{}/gif-provider.json", server.base)],
            &temp_cache(),
        );
        assert_eq!(
            svc.get_json("gifs/categories", &[]).await.unwrap_err(),
            GIF_API_KEY_MISSING
        );
        assert_eq!(server.count("/api/"), 0, "never calls KLIPY without a key");
    }

    #[tokio::test]
    async fn broken_provider_file_is_reported_without_a_url() {
        let mut server = MockServer::bind().await;
        server.serve(|_| (200, "{ this is not json".into()));
        let svc = service(
            vec![format!("{}/gif-provider.json", server.base)],
            &temp_cache(),
        );
        let err = svc.get_json("gifs/categories", &[]).await.unwrap_err();
        assert!(
            err.starts_with("GIF provider settings unavailable:"),
            "{}",
            err
        );
        assert!(!err.contains("http"), "{}", err);
    }

    #[tokio::test]
    async fn user_key_from_config_wins_and_skips_the_download() {
        let mut api = MockServer::bind().await;
        api.serve(|path| match path {
            "/api/user-key/gifs/categories" => categories_ok(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let svc = GifService::new(
            Some(GifProvider {
                base_url: format!("{}/api", api.base),
                api_key: "user-key".into(),
            }),
            vec!["http://127.0.0.1:1/gif-provider.json".into()],
            cache.clone(),
        )
        .unwrap();
        svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(api.count("/api/user-key/gifs/categories"), 1);
        assert!(!cache.exists(), "no provider download, so nothing to cache");
    }

    #[tokio::test]
    async fn base_url_override_applies_to_the_downloaded_provider() {
        let mut server = MockServer::bind().await;
        server.serve(|path| match path {
            "/gif-provider.json" => (200, provider_json("k1", "https://api.klipy.com/api/v1")),
            "/proxy/k1/gifs/categories" => categories_ok(),
            _ => (404, "{}".into()),
        });
        let cache = temp_cache();
        let mut svc = service(vec![format!("{}/gif-provider.json", server.base)], &cache);
        svc.base_url_override = Some(format!("{}/proxy", server.base));
        svc.get_json("gifs/categories", &[]).await.unwrap();
        assert_eq!(server.count("/proxy/k1/gifs/categories"), 1);
        let _ = std::fs::remove_file(&cache);
    }

    #[tokio::test]
    async fn network_errors_never_leak_the_key_or_url() {
        let svc = GifService::new(
            Some(GifProvider {
                base_url: "http://127.0.0.1:1/api".into(),
                api_key: "secret-key".into(),
            }),
            vec![],
            temp_cache(),
        )
        .unwrap();
        let err = svc.get_json("gifs/categories", &[]).await.unwrap_err();
        assert_eq!(err, GIF_NETWORK_ERROR);
    }

    #[tokio::test]
    async fn http_and_api_errors_are_short_and_url_free() {
        let mut api = MockServer::bind().await;
        api.serve(|path| match path {
            "/api/k/gifs/search" => (503, "<html>overloaded</html>".into()),
            "/api/k/gifs/trending" => (
                200,
                json!({ "result": false, "errors": { "message": ["Rate limit exceeded"] } })
                    .to_string(),
            ),
            _ => (404, "{}".into()),
        });
        let svc = GifService::new(
            Some(GifProvider {
                base_url: format!("{}/api", api.base),
                api_key: "k".into(),
            }),
            vec![],
            temp_cache(),
        )
        .unwrap();
        let err = svc
            .get_json("gifs/search", &[("q", "cats".into())])
            .await
            .unwrap_err();
        assert_eq!(err, "API error: 503 Service Unavailable");
        let err = svc.get_json("gifs/trending", &[]).await.unwrap_err();
        assert_eq!(err, "API returned error: Rate limit exceeded");
        assert!(
            api.requests().iter().any(|r| r.contains("q=cats")),
            "{:?}",
            api.requests()
        );
    }

    /// Talks to the real KLIPY API. Run with `cargo test -p linvclip-ui -- --ignored`.
    /// Proves the HTTP/1.1 pin: over h2 the same request has been seen to hang
    /// past the 10 s timeout (→ `gif_network_error`), which is exactly the
    /// "error sending request" users reported. A prompt "invalid key" answer
    /// means the transport works. (No wall-clock bound: DNS alone can take
    /// 5 s in some containers.)
    #[tokio::test]
    #[ignore = "needs network access to api.klipy.com"]
    async fn live_klipy_rejects_a_bogus_key_quickly() {
        let svc = GifService::new(
            Some(GifProvider {
                base_url: DEFAULT_BASE_URL.into(),
                api_key: "not-a-real-key".into(),
            }),
            vec![],
            temp_cache(),
        )
        .unwrap();
        let start = std::time::Instant::now();
        let err = svc.get_json("gifs/categories", &[]).await.unwrap_err();
        eprintln!("KLIPY answered in {:?}", start.elapsed());
        assert_eq!(err, GIF_API_KEY_INVALID);
    }
}
