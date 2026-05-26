// D-Bus interface for LinVClipBoard daemon.
//
// Gated behind the `dbus` feature flag:
//   cargo build -p clipd --features dbus
//
// Exposes `org.linvclipboard.Daemon` on the session bus with methods:
//   List(offset u32, limit u32) → JSON string
//   Search(query str, limit u32) → JSON string
//   Status() → JSON string
//   Paste(id str) → JSON string

#[cfg(feature = "dbus")]
use shared::db::Database;
#[cfg(feature = "dbus")]
use std::sync::Arc;

#[cfg(feature = "dbus")]
pub struct ClipboardDbusService {
    pub db: Arc<Database>,
}

#[cfg(feature = "dbus")]
#[zbus::interface(name = "org.linvclipboard.Daemon")]
impl ClipboardDbusService {
    async fn list(&self, offset: u32, limit: u32) -> String {
        match self.db.list(offset, limit) {
            Ok((items, total)) => serde_json::json!({
                "items": items,
                "total": total,
            })
            .to_string(),
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
        }
    }

    async fn search(&self, query: &str, limit: u32, offset: u32) -> String {
        match self.db.search(query, limit, offset) {
            Ok((items, total)) => serde_json::json!({
                "items": items,
                "total": total,
            })
            .to_string(),
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
        }
    }

    async fn status(&self) -> String {
        let total = self.db.total_items().unwrap_or(0);
        let size = self.db.db_size().unwrap_or(0);
        serde_json::json!({
            "total_items": total,
            "db_size_bytes": size,
        })
        .to_string()
    }
}

/// Start the D-Bus service. Runs forever.
#[cfg(feature = "dbus")]
pub async fn start_dbus_service(
    db: Arc<Database>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = ClipboardDbusService { db };
    let conn = zbus::Connection::session().await?;
    conn.object_server()
        .at("/org/linvclipboard/Daemon", service)
        .await?;
    conn.request_name("org.linvclipboard.Daemon").await?;
    tracing::info!("D-Bus service registered: org.linvclipboard.Daemon");

    // Keep the task alive
    std::future::pending::<()>().await;
    Ok(())
}

/// No-op when the `dbus` feature is disabled.
#[cfg(not(feature = "dbus"))]
pub async fn start_dbus_service(
    _db: std::sync::Arc<shared::db::Database>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // D-Bus support not compiled in — silently skip.
    std::future::pending::<()>().await;
    Ok(())
}
