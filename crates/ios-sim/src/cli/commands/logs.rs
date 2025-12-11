use anyhow::Result;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

use crate::cli::LogsArgs;

/// Stream logs from agent
pub async fn run(args: LogsArgs, agent_url: &str) -> Result<()> {
    let build_id = args.build_id.ok_or_else(|| {
        anyhow::anyhow!("Build ID is required. Use --build-id <uuid>")
    })?;

    // Convert HTTP URL to WebSocket URL
    let ws_url = agent_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let logs_url = format!("{}/logs/{}", ws_url, build_id);

    println!("Connecting to {}...", logs_url);

    let (ws_stream, _) = connect_async(&logs_url).await?;
    let (_, mut read) = ws_stream.split();

    println!("Connected. Streaming logs...\n");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                // Try to parse as LogMessage JSON
                if let Ok(log_msg) = serde_json::from_str::<ios_sim_common::LogMessage>(&text) {
                    print_log_message(&log_msg);
                } else {
                    // Raw text
                    println!("{}", text);
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                println!("\nConnection closed.");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn print_log_message(msg: &ios_sim_common::LogMessage) {
    use ios_sim_common::{LogLevel, LogMessage};

    match msg {
        LogMessage::BuildOutput {
            timestamp,
            level,
            message,
        } => {
            let prefix = match level {
                LogLevel::Error => "\x1b[31m[ERROR]\x1b[0m",
                LogLevel::Warning => "\x1b[33m[WARN]\x1b[0m",
                LogLevel::Info => "\x1b[34m[INFO]\x1b[0m",
                LogLevel::Debug => "\x1b[90m[DEBUG]\x1b[0m",
            };
            println!("{} {}", prefix, message);
        }
        LogMessage::AppLog {
            timestamp,
            process,
            subsystem,
            category,
            message,
        } => {
            let sub = subsystem.as_deref().unwrap_or("");
            let cat = category.as_deref().unwrap_or("");
            if !sub.is_empty() || !cat.is_empty() {
                println!("\x1b[36m[{}:{}:{}]\x1b[0m {}", process, sub, cat, message);
            } else {
                println!("\x1b[36m[{}]\x1b[0m {}", process, message);
            }
        }
        LogMessage::SystemEvent {
            timestamp,
            event,
            message,
        } => {
            println!("\x1b[35m==> {:?}: {}\x1b[0m", event, message);
        }
        LogMessage::BuildProgress {
            timestamp,
            phase,
            target,
            progress_percent,
        } => {
            if let Some(target) = target {
                print!("\x1b[90m[{}] {}", phase, target);
            } else {
                print!("\x1b[90m[{}]", phase);
            }
            if let Some(pct) = progress_percent {
                print!(" {}%", pct);
            }
            println!("\x1b[0m");
        }
    }
}
