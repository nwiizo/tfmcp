use std::fmt::Display;

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

/// Log through the configured subscriber, which writes to stderr.
pub fn log(level: LogLevel, message: &str) {
    match level {
        LogLevel::Debug => tracing::debug!("{message}"),
        LogLevel::Info => tracing::info!("{message}"),
        LogLevel::Warning => tracing::warn!("{message}"),
        LogLevel::Error => tracing::error!("{message}"),
    }
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
