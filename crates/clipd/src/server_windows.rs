use platform::ipc::{recv_message, send_message};
use platform::IpcTransport;
use platform::WindowsClipboardProvider;
use platform::WindowsIpcTransport;
use shared::config::AppConfig;
use shared::db::Database;
use shared::models::{ContentType, IpcRequest, IpcResponse};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub async fn run(
    db: Arc<Database>,
    config: Arc<AppConfig>,
    _pipe_path: &Path,
    start_time: Instant,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let transport = WindowsIpcTransport::new_from_str("");
    let mut listener = transport.bind().await?;

    let semaphore = Arc::new(Semaphore::new(10));

    tracing::info!("🔌 Windows IPC server listening on {:?}", transport.path());

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut stream, _addr)) => {
                        let db = Arc::clone(&db);
                        let config = Arc::clone(&config);
                        let start = start_time;
                        let sem = Arc::clone(&semaphore);

                        tokio::spawn(async move {
                            let _permit = match sem.acquire().await {
                                Ok(p) => p,
                                Err(_) => return,
                            };

                            match recv_message::<IpcRequest>(&mut *stream).await {
                                Ok(request) => {
                                    let response = handle_request(&db, &config, request, start).await;
                                    if let Err(e) = send_message(&mut *stream, &response).await {
                                        tracing::error!("Failed to send response: {}", e);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to receive request: {}", e);
                                    let _ = send_message(
                                        &mut *stream,
                                        &IpcResponse::Error {
                                            message: format!("Invalid request: {}", e),
                                        },
                                    )
                                    .await;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
            _ = cancel.cancelled() => {
                tracing::info!("Server received cancellation, stopping");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_request(
    db: &Database,
    config: &AppConfig,
    request: IpcRequest,
    start_time: Instant,
) -> IpcResponse {
    match request {
        IpcRequest::List { offset, limit } => match db.list(offset, limit) {
            Ok((items, total)) => IpcResponse::Items { items, total },
            Err(e) => IpcResponse::Error {
                message: format!("List failed: {}", e),
            },
        },

        IpcRequest::Search {
            query,
            limit,
            offset,
        } => match db.search(&query, limit, offset) {
            Ok((items, total)) => IpcResponse::Items { items, total },
            Err(e) => IpcResponse::Error {
                message: format!("Search failed: {}", e),
            },
        },

        IpcRequest::SearchRegex {
            pattern,
            limit,
            offset,
        } => match db.search_regex(&pattern, limit, offset) {
            Ok((items, total)) => IpcResponse::Items { items, total },
            Err(e) => IpcResponse::Error {
                message: format!("Regex search failed: {}", e),
            },
        },

        IpcRequest::Get { id } => match db.get(&id) {
            Ok(item) => IpcResponse::Item(item),
            Err(e) => IpcResponse::Error {
                message: format!("Get failed: {}", e),
            },
        },

        IpcRequest::Delete { id } => match db.delete(&id) {
            Ok(()) => IpcResponse::Ok {
                message: format!("Deleted item {}", id),
            },
            Err(e) => IpcResponse::Error {
                message: format!("Delete failed: {}", e),
            },
        },

        IpcRequest::BulkDelete { ids } => match db.bulk_delete(&ids) {
            Ok(count) => IpcResponse::Ok {
                message: format!("Deleted {} items", count),
            },
            Err(e) => IpcResponse::Error {
                message: format!("Bulk delete failed: {}", e),
            },
        },

        IpcRequest::BulkPin { ids, pinned } => match db.bulk_pin(&ids, pinned) {
            Ok(count) => {
                let action = if pinned { "Pinned" } else { "Unpinned" };
                IpcResponse::Ok {
                    message: format!("{} {} items", action, count),
                }
            }
            Err(e) => IpcResponse::Error {
                message: format!("Bulk pin failed: {}", e),
            },
        },

        IpcRequest::TogglePin { id } => match db.toggle_pin(&id) {
            Ok(item) => IpcResponse::Item(item),
            Err(e) => IpcResponse::Error {
                message: format!("Toggle pin failed: {}", e),
            },
        },

        IpcRequest::Paste { id } => match paste_impl(db, &id) {
            Ok(msg) => IpcResponse::Ok { message: msg },
            Err(e) => IpcResponse::Error {
                message: format!("Paste failed: {}", e),
            },
        },

        IpcRequest::Clear => match db.clear_unpinned() {
            Ok(count) => IpcResponse::Ok {
                message: format!("Cleared {} items (pinned items kept)", count),
            },
            Err(e) => IpcResponse::Error {
                message: format!("Clear failed: {}", e),
            },
        },

        IpcRequest::Status => {
            let uptime = start_time.elapsed().as_secs();
            let total_items = db.total_items().unwrap_or(0);
            let db_size = db.db_size().unwrap_or(0);

            IpcResponse::Status {
                uptime_secs: uptime,
                total_items,
                db_size_bytes: db_size,
            }
        }

        IpcRequest::AddTag { id, tag } => match db.add_tag(&id, &tag) {
            Ok(item) => IpcResponse::Item(item),
            Err(e) => IpcResponse::Error {
                message: format!("Add tag failed: {}", e),
            },
        },

        IpcRequest::RemoveTag { id, tag } => match db.remove_tag(&id, &tag) {
            Ok(item) => IpcResponse::Item(item),
            Err(e) => IpcResponse::Error {
                message: format!("Remove tag failed: {}", e),
            },
        },

        IpcRequest::GetConfig => IpcResponse::Config(config.clone()),

        IpcRequest::SaveConfig { config: new_config } => {
            let path = AppConfig::config_path();
            match toml::to_string_pretty(&new_config) {
                Ok(content) => {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match std::fs::write(&path, content) {
                        Ok(()) => IpcResponse::Ok {
                            message: "Config saved. Restart clipd to apply changes.".to_string(),
                        },
                        Err(e) => IpcResponse::Error {
                            message: format!("Failed to write config: {}", e),
                        },
                    }
                }
                Err(e) => IpcResponse::Error {
                    message: format!("Failed to serialize config: {}", e),
                },
            }
        }

        IpcRequest::ListSnippets { folder } => match db.list_snippets(folder.as_deref()) {
            Ok(snippets) => IpcResponse::Snippets(snippets),
            Err(e) => IpcResponse::Error {
                message: format!("List snippets failed: {}", e),
            },
        },

        IpcRequest::SearchSnippets { query } => match db.search_snippets(&query) {
            Ok(snippets) => IpcResponse::Snippets(snippets),
            Err(e) => IpcResponse::Error {
                message: format!("Search snippets failed: {}", e),
            },
        },

        IpcRequest::GetSnippet { id } => match db.get_snippet(&id) {
            Ok(snippet) => IpcResponse::Snippet(snippet),
            Err(e) => IpcResponse::Error {
                message: format!("Get snippet failed: {}", e),
            },
        },

        IpcRequest::CreateSnippet {
            name,
            content,
            folder,
            abbreviation,
            variables,
        } => {
            let snippet =
                shared::models::Snippet::new(name, content, folder, abbreviation, variables);
            match db.create_snippet(&snippet) {
                Ok(()) => IpcResponse::Snippet(snippet),
                Err(e) => IpcResponse::Error {
                    message: format!("Create snippet failed: {}", e),
                },
            }
        }

        IpcRequest::UpdateSnippet {
            id,
            name,
            content,
            folder,
            abbreviation,
            variables,
        } => match db.update_snippet(&id, &name, &content, &folder, &abbreviation, &variables) {
            Ok(snippet) => IpcResponse::Snippet(snippet),
            Err(e) => IpcResponse::Error {
                message: format!("Update snippet failed: {}", e),
            },
        },

        IpcRequest::DeleteSnippet { id } => match db.delete_snippet(&id) {
            Ok(()) => IpcResponse::Ok {
                message: "Snippet deleted".to_string(),
            },
            Err(e) => IpcResponse::Error {
                message: format!("Delete snippet failed: {}", e),
            },
        },

        IpcRequest::UpdatePreviewText { id, preview_text } => {
            match db.update_preview_text(&id, &preview_text) {
                Ok(item) => IpcResponse::Item(item),
                Err(e) => IpcResponse::Error {
                    message: format!("Update preview text failed: {}", e),
                },
            }
        }

        IpcRequest::UseSnippet { id, variables } => match db.get_snippet(&id) {
            Ok(snippet) => {
                let rendered = shared::models::render_template(&snippet.content, &variables);
                match paste_text(&rendered) {
                    Ok(()) => {
                        let _ = db.increment_snippet_use(&id);
                        IpcResponse::Ok { message: rendered }
                    }
                    Err(e) => IpcResponse::Error {
                        message: format!("Clipboard set failed: {}", e),
                    },
                }
            }
            Err(e) => IpcResponse::Error {
                message: format!("Snippet not found: {}", e),
            },
        },
    }
}

fn paste_text(text: &str) -> Result<(), String> {
    let provider = WindowsClipboardProvider::new().map_err(|e| e.to_string())?;
    provider.set_text(text).map_err(|e| e.to_string())
}

fn paste_impl(db: &Database, id: &str) -> Result<String, String> {
    let item = db.get(id).map_err(|e| format!("Item not found: {}", e))?;
    let provider = WindowsClipboardProvider::new().map_err(|e| e.to_string())?;

    match item.content_type {
        ContentType::Html => {
            let html = item.content.clone();
            let plain = html2text::from_read(html.as_bytes(), 200).unwrap_or_default();
            provider.set_text(&plain).map_err(|e| e.to_string())?;
            Ok("Pasted HTML to clipboard (plain text fallback)".to_string())
        }
        ContentType::Files => {
            let paths: Vec<String> = serde_json::from_str(&item.content).unwrap_or_default();
            let uri_list: String = paths
                .iter()
                .map(|p| format!("file://{}", p))
                .collect::<Vec<_>>()
                .join("\n");
            provider.set_text(&uri_list).map_err(|e| e.to_string())?;
            Ok("Pasted file URIs to clipboard".to_string())
        }
        ContentType::PlainText | ContentType::RichText | ContentType::Uri => {
            provider
                .set_text(&item.content)
                .map_err(|e| e.to_string())?;
            Ok("Pasted to clipboard".to_string())
        }
        ContentType::Image => {
            let img_path = std::path::Path::new(&item.content);
            if img_path.exists() {
                Err("Image paste not yet implemented on Windows".to_string())
            } else {
                Err(format!("Image file not found: {}", item.content))
            }
        }
    }
}
