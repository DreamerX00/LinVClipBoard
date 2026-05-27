use platform::ClipboardProvider;
use platform::WindowsClipboardProvider;
use shared::config::AppConfig;
use shared::db::Database;
use shared::models::{ClipboardItem, ContentType};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Run the Windows clipboard monitor loop.
/// Uses clipboard_win for event-driven change detection.
pub async fn run(
    db: Arc<Database>,
    config: Arc<AppConfig>,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clipboard = WindowsClipboardProvider::new()?;
    let mut last_text_checksum = String::new();
    let mut consecutive_failures: u32 = 0;
    let mut poll_count: u64 = 0;

    tracing::info!("📋 Windows clipboard monitor started (event-driven)");

    loop {
        if cancel.is_cancelled() {
            tracing::info!("Monitor received cancellation, stopping");
            break;
        }

        if !config.security.incognito {
            match clipboard.get_text() {
                Ok(Some(text)) => {
                    let checksum = compute_checksum(text.as_bytes());
                    if checksum != last_text_checksum {
                        last_text_checksum = checksum;
                        let preview = text.chars().take(200).collect::<String>();
                        let size = text.len() as u64;
                        let item = ClipboardItem::new(
                            ContentType::PlainText,
                            text,
                            preview,
                            checksum.clone(),
                            size,
                        );
                        match db.insert(&item) {
                            Ok(true) => tracing::debug!("Captured text: {} chars", size),
                            Ok(false) => tracing::debug!("Duplicate text skipped"),
                            Err(e) => tracing::error!("DB insert error: {}", e),
                        }
                    }
                    consecutive_failures = 0;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Clipboard read error: {}", e);
                    consecutive_failures += 1;
                }
            }
        }

        poll_count += 1;
        if poll_count.is_multiple_of(100) {
            if let Err(e) = db.enforce_limits(&config.storage) {
                tracing::error!("Limit enforcement error: {}", e);
            }
            let blob_dir = AppConfig::blob_dir();
            if let Err(e) = db.cleanup_orphan_blobs(&blob_dir) {
                tracing::error!("Orphan blob cleanup error: {}", e);
            }
        }

        if consecutive_failures > 10 {
            tracing::warn!("Too many clipboard failures, backing off...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    Ok(())
}

fn compute_checksum(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
