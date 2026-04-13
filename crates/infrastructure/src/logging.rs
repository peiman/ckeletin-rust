use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Guard that must be held for the lifetime of the application.
/// Dropping it flushes the non-blocking log file writer.
pub struct LogGuard {
    _guard: Option<WorkerGuard>,
}

/// Logging configuration.
pub struct LogConfig {
    /// Console (stderr) log level. "off" to suppress.
    pub console_level: String,
    /// Enable file logging (audit stream).
    pub file_enabled: bool,
    /// Path to the log file.
    pub file_path: String,
    /// File log level (typically "debug" or "trace").
    pub file_level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console_level: "info".to_string(),
            file_enabled: false,
            file_path: "logs/ckeletin-rust.log".to_string(),
            file_level: "debug".to_string(),
        }
    }
}

/// Initialize tracing with stderr (status stream) and optional file (audit stream).
///
/// CKSPEC-OUT-001: stderr for status, file for audit.
/// CKSPEC-OUT-004: shadow logging — output.rs emits tracing events that land here.
///
/// Returns a guard that must be held until shutdown (flushes file writer).
pub fn init(config: &LogConfig) -> Result<LogGuard, Box<dyn std::error::Error>> {
    let stderr_filter =
        EnvFilter::try_new(&config.console_level).unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(true)
        .with_filter(stderr_filter);

    if config.file_enabled {
        let log_path = std::path::Path::new(&config.file_path);
        let log_dir = log_path.parent().unwrap_or(std::path::Path::new("."));
        let log_name = log_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        std::fs::create_dir_all(log_dir)?;

        let file_appender = tracing_appender::rolling::daily(log_dir, log_name);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_filter =
            EnvFilter::try_new(&config.file_level).unwrap_or_else(|_| EnvFilter::new("debug"));

        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_filter(file_filter);

        tracing_subscriber::registry()
            .with(stderr_layer)
            .with(file_layer)
            .init();

        Ok(LogGuard {
            _guard: Some(guard),
        })
    } else {
        tracing_subscriber::registry().with(stderr_layer).init();

        Ok(LogGuard { _guard: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = LogConfig::default();
        assert_eq!(config.console_level, "info");
        assert!(!config.file_enabled);
        assert_eq!(config.file_path, "logs/ckeletin-rust.log");
        assert_eq!(config.file_level, "debug");
    }

    #[test]
    fn config_allows_custom_values() {
        let config = LogConfig {
            console_level: "debug".to_string(),
            file_enabled: true,
            file_path: "/tmp/test.log".to_string(),
            file_level: "trace".to_string(),
        };
        assert_eq!(config.console_level, "debug");
        assert!(config.file_enabled);
        assert_eq!(config.file_level, "trace");
    }
}
