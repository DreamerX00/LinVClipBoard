#[cfg(unix)]
mod config_watcher;
mod dbus_service;
#[cfg(unix)]
mod monitor;
#[cfg(windows)]
mod monitor_windows;
#[cfg(unix)]
mod server;
#[cfg(windows)]
mod server_windows;

use shared::config::AppConfig;
use shared::db::Database;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.daemon.log_level)),
        )
        .with_target(false)
        .compact()
        .init();

    tracing::info!(
        "🚀 LinVClipBoard daemon v{} starting...",
        env!("CARGO_PKG_VERSION")
    );

    let blob_dir = AppConfig::blob_dir();
    std::fs::create_dir_all(&blob_dir)?;

    let db_path = AppConfig::db_path();
    let db = Database::open(&db_path)?;
    tracing::info!("Database opened at {:?}", db_path);

    let db = Arc::new(db);
    let config = Arc::new(config);
    let start_time = std::time::Instant::now();
    let cancel = CancellationToken::new();

    #[cfg(unix)]
    run_unix(&db, &config, start_time, &cancel).await?;

    #[cfg(windows)]
    run_windows(&db, &config, start_time, &cancel).await?;

    let dbus_db = Arc::clone(&db);
    let _dbus_handle = tokio::spawn(async move {
        if let Err(e) = dbus_service::start_dbus_service(dbus_db).await {
            tracing::warn!("D-Bus service unavailable: {}", e);
        }
    });

    #[cfg(unix)]
    wait_for_shutdown().await;

    #[cfg(windows)]
    wait_for_shutdown_windows().await?;

    cancel.cancel();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    })
    .await;

    #[cfg(unix)]
    {
        let socket_path = AppConfig::socket_path();
        if socket_path.exists() {
            let _ = std::fs::remove_file(&socket_path);
        }
    }

    tracing::info!("👋 Daemon stopped.");
    Ok(())
}

/// Wait for Ctrl+C or SIGTERM (Unix only).
#[cfg(unix)]
async fn wait_for_shutdown() {
    let mut sigterm = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to register SIGTERM handler: {}", e);
            return;
        }
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("SIGINT received, shutting down…");
        }
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received, shutting down…");
        }
    }
}

/// Wait for Ctrl+C on Windows.
#[cfg(windows)]
async fn wait_for_shutdown_windows() -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await?;
    tracing::info!("Ctrl+C received, shutting down…");
    Ok(())
}

#[cfg(unix)]
async fn run_unix(
    db: &Arc<Database>,
    config: &Arc<AppConfig>,
    start_time: std::time::Instant,
    cancel: &CancellationToken,
) -> anyhow::Result<()> {
    let monitor_db = Arc::clone(db);
    let monitor_config = Arc::clone(config);
    let monitor_cancel = cancel.clone();
    let _monitor_handle = tokio::spawn(async move {
        if let Err(e) = monitor::run(monitor_db, monitor_config, monitor_cancel).await {
            tracing::error!("Monitor error: {}", e);
        }
    });

    let server_db = Arc::clone(db);
    let server_config = Arc::clone(config);
    let socket_path = AppConfig::socket_path();
    let server_cancel = cancel.clone();
    let _server_handle = tokio::spawn(async move {
        if let Err(e) = server::run(
            server_db,
            server_config,
            &socket_path,
            start_time,
            server_cancel,
        )
        .await
        {
            tracing::error!("Server error: {}", e);
        }
    });

    let config_path = AppConfig::config_path();
    let _config_watcher_handle = tokio::spawn(async move {
        config_watcher::watch(config_path).await;
    });

    tracing::info!(
        "✨ Daemon ready. Listening on {:?}",
        AppConfig::socket_path()
    );

    Ok(())
}

/// Run the Windows daemon (monitor + server stub).
#[cfg(windows)]
async fn run_windows(
    db: &Arc<Database>,
    config: &Arc<AppConfig>,
    start_time: std::time::Instant,
    cancel: &CancellationToken,
) -> anyhow::Result<()> {
    let monitor_db = Arc::clone(db);
    let monitor_config = Arc::clone(config);
    let monitor_cancel = cancel.clone();
    let _monitor_handle = tokio::spawn(async move {
        if let Err(e) = monitor_windows::run(monitor_db, monitor_config, monitor_cancel).await {
            tracing::error!("Windows monitor error: {}", e);
        }
    });

    let server_db = Arc::clone(db);
    let server_config = Arc::clone(config);
    let pipe_path = AppConfig::socket_path();
    let server_cancel = cancel.clone();
    let _server_handle = tokio::spawn(async move {
        if let Err(e) = server_windows::run(
            server_db,
            server_config,
            &pipe_path,
            start_time,
            server_cancel,
        )
        .await
        {
            tracing::error!("Windows server error: {}", e);
        }
    });

    tracing::info!("✨ Daemon ready. Pipe at {:?}", AppConfig::socket_path());

    Ok(())
}
