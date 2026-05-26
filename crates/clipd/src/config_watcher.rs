use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;

/// Watch the config file for changes and log when it is modified.
pub async fn watch(config_path: PathBuf) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(4);

    let _watcher = {
        let tx = tx.clone();
        let mut w = match notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = tx.blocking_send(());
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Config watcher init failed: {}", e);
                return;
            }
        };

        if let Err(e) = w.watch(&config_path, RecursiveMode::NonRecursive) {
            tracing::warn!("Could not watch config file: {}", e);
        }
        w
    };

    while let Some(()) = rx.recv().await {
        tracing::info!("Config file changed — reload will take effect on next daemon restart");
    }
}
