use chrono::Local;
use serde_json::json;
use std::fmt::Display;

use crate::mcp::stdio::{Message, Transport};

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warning => write!(f, "warning"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Log a message to stderr with timestamp and log level
pub fn log(level: LogLevel, message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    eprintln!("[{}] [{}] {}", timestamp, level, message);
}

/// Log debug level message
pub fn debug(message: &str) {
    log(LogLevel::Debug, message);
}

/// Log info level message
pub fn info(message: &str) {
    log(LogLevel::Info, message);
}

/// Log warning level message
pub fn warn(message: &str) {
    log(LogLevel::Warning, message);
}

/// Log error level message
pub fn error(message: &str) {
    log(LogLevel::Error, message);
}

/// Send a log message to the client via MCP
pub async fn send_log_message<T: Transport>(
    transport: &T,
    level: LogLevel,
    message: &str,
) -> Result<(), crate::mcp::stdio::Error> {
    // Create a log notification as per MCP protocol
    let log_notification = Message::Notification {
        jsonrpc: "2.0".to_string(),
        method: "$/log".to_string(),
        params: Some(json!({
            "level": level.to_string(),
            "message": message
        })),
    };

    transport.send(log_notification).await
}

/// Log a message both to stderr and to the client via MCP
pub async fn log_both<T: Transport>(
    transport: &T,
    level: LogLevel,
    message: &str,
) -> Result<(), crate::mcp::stdio::Error> {
    // Log to stderr
    log(level, message);

    // Send to client
    send_log_message(transport, level, message).await
}
