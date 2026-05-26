use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser, Subcommand};
use colored::*;
use shared::config::AppConfig;
use shared::ipc::send_request;
use shared::models::{ContentType, IpcRequest, IpcResponse};

#[derive(Parser)]
#[command(name = "clipctl")]
#[command(about = "LinVClipBoard CLI — manage your clipboard history")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List clipboard history
    List {
        /// Maximum number of items to show
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
        /// Offset for pagination
        #[arg(short, long, default_value_t = 0)]
        offset: u32,
    },
    /// Search clipboard history
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
        /// Pagination offset
        #[arg(short, long, default_value_t = 0)]
        offset: u32,
    },
    /// Paste an item back to clipboard
    Paste {
        /// Item ID
        id: String,
    },
    /// Toggle pin on an item
    Pin {
        /// Item ID
        id: String,
    },
    /// Delete an item
    Delete {
        /// Item ID
        id: String,
    },
    /// Add a tag to an item
    Tag {
        /// Item ID
        id: String,
        /// Tag to add
        tag: String,
    },
    /// Remove a tag from an item
    Untag {
        /// Item ID
        id: String,
        /// Tag to remove
        tag: String,
    },
    /// Clear all non-pinned items
    Clear,
    /// Show daemon status
    Status,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
    /// Generate man pages
    Manpage {
        /// Output directory for man pages
        out_dir: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle offline subcommands that don't need the daemon
    match &cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "clipctl", &mut std::io::stdout());
            return;
        }
        Commands::Manpage { out_dir } => {
            let cmd = Cli::command();
            if let Err(e) = std::fs::create_dir_all(out_dir) {
                eprintln!("{} Failed to create dir: {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
            let man = clap_mangen::Man::new(cmd);
            let mut buf = Vec::new();
            if let Err(e) = man.render(&mut buf) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
            let out_path = out_dir.join("clipctl.1");
            if let Err(e) = std::fs::write(&out_path, buf) {
                eprintln!("{} {}", "Error:".red().bold(), e);
                std::process::exit(1);
            }
            println!("Man page written to {}", out_path.display());
            return;
        }
        _ => {}
    }

    let socket_path = AppConfig::socket_path();

    if !socket_path.exists() {
        eprintln!("{}", "Error: clipd daemon is not running.".red().bold());
        eprintln!("Start it with: {}", "clipd".cyan());
        eprintln!(
            "Or enable the service: {}",
            "systemctl --user enable --now clipd".cyan()
        );
        std::process::exit(1);
    }

    let request = match &cli.command {
        Commands::List { limit, offset } => IpcRequest::List {
            offset: *offset,
            limit: *limit,
        },
        Commands::Search {
            query,
            limit,
            offset,
        } => IpcRequest::Search {
            query: query.clone(),
            limit: *limit,
            offset: *offset,
        },
        Commands::Paste { id } => IpcRequest::Paste { id: id.clone() },
        Commands::Pin { id } => IpcRequest::TogglePin { id: id.clone() },
        Commands::Delete { id } => IpcRequest::Delete { id: id.clone() },
        Commands::Tag { id, tag } => IpcRequest::AddTag {
            id: id.clone(),
            tag: tag.clone(),
        },
        Commands::Untag { id, tag } => IpcRequest::RemoveTag {
            id: id.clone(),
            tag: tag.clone(),
        },
        Commands::Clear => IpcRequest::Clear,
        Commands::Status => IpcRequest::Status,
        // Completions and Manpage are handled above and never reach here
        Commands::Completions { .. } | Commands::Manpage { .. } => unreachable!(),
    };

    match send_request(&socket_path, &request).await {
        Ok(response) => print_response(&cli.command, response),
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn print_response(_command: &Commands, response: IpcResponse) {
    match response {
        IpcResponse::Items { items, total } => {
            if items.is_empty() {
                println!("{}", "No items found.".dimmed());
                return;
            }

            println!(
                "{} ({} total)\n",
                "📋 Clipboard History".bold().cyan(),
                total.to_string().yellow()
            );

            for item in &items {
                let pin_icon = if item.pinned { "📌 " } else { "   " };
                let type_icon = match item.content_type {
                    ContentType::PlainText => "📝",
                    ContentType::Html => "🌐",
                    ContentType::Image => "🖼️",
                    ContentType::RichText => "📄",
                    ContentType::Files => "📁",
                    ContentType::Uri => "🔗",
                };

                let time = format_time(&item.created_at);
                let id_short = &item.id[..8];

                // Preview: truncate to 60 chars
                let preview = if item.preview_text.len() > 60 {
                    format!("{}...", &item.preview_text[..57])
                } else {
                    item.preview_text.clone()
                };
                // Replace newlines for display
                let preview = preview.replace('\n', "↵ ");

                println!(
                    "{}{} {} {} {}",
                    pin_icon,
                    id_short.dimmed(),
                    type_icon,
                    preview,
                    time.dimmed()
                );
            }

            println!(
                "\n{} Use {} to paste an item",
                "💡".dimmed(),
                "clipctl paste <id>".cyan()
            );
        }

        IpcResponse::Item(item) => {
            let pin_status = if item.pinned {
                "📌 Pinned"
            } else {
                "Unpinned"
            };
            println!("{} {}", "Item:".bold(), item.id.cyan());
            println!("  Type: {:?}", item.content_type);
            println!("  Status: {}", pin_status);
            println!("  Size: {} bytes", item.size_bytes);
            println!("  Created: {}", item.created_at.format("%Y-%m-%d %H:%M:%S"));
            if item.content_type != ContentType::Image {
                let preview = if item.preview_text.len() > 200 {
                    format!("{}...", &item.preview_text[..197])
                } else {
                    item.preview_text.clone()
                };
                println!("  Preview: {}", preview);
            }
        }

        IpcResponse::Ok { message } => {
            println!("{} {}", "✅".green(), message);
        }

        IpcResponse::Error { message } => {
            eprintln!("{} {}", "❌".red(), message);
            std::process::exit(1);
        }

        IpcResponse::Status {
            uptime_secs,
            total_items,
            db_size_bytes,
        } => {
            println!("{}", "📊 LinVClipBoard Status".bold().cyan());
            println!("  Uptime:      {}", format_duration(uptime_secs).yellow());
            println!("  Total items: {}", total_items.to_string().yellow());
            println!("  DB size:     {}", format_bytes(db_size_bytes).yellow());
        }

        IpcResponse::Config(config) => {
            println!("{}", "⚙️  Current Configuration".bold().cyan());
            println!(
                "{}",
                serde_json::to_string_pretty(&config).unwrap_or_default()
            );
        }
        IpcResponse::Snippets(snippets) => {
            println!("{} snippet(s)", snippets.len());
            for s in &snippets {
                println!("  {} — {} (used {}×)", s.name, s.abbreviation, s.use_count);
            }
        }
        IpcResponse::Snippet(s) => {
            println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
        }
    }
}

fn format_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(*dt);

    let secs = diff.num_seconds().max(0);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
