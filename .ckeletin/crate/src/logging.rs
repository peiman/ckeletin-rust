use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter, Layer,
};

/// Guard that must be held for the lifetime of the application.
/// Dropping it flushes the non-blocking log file writer.
pub struct LogGuard {
    _guard: Option<WorkerGuard>,
}

/// Logging configuration.
pub struct LogConfig {
    /// Console (stderr) log level filter string. "off" to suppress.
    pub console_level: String,
    /// Enable file logging (audit stream).
    pub file_enabled: bool,
    /// Path to the log file.
    pub file_path: String,
    /// File log level filter string (typically "debug" or "trace").
    pub file_level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console_level: "info".to_string(),
            file_enabled: false,
            file_path: "logs/app.log".to_string(),
            file_level: "debug".to_string(),
        }
    }
}

/// Build an EnvFilter from a level string, falling back to the provided default.
fn build_filter(level: &str, fallback: &str) -> EnvFilter {
    EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new(fallback))
}

/// Prepare the log file directory and appender.
/// Returns (non_blocking_writer, guard) or an error if the directory can't be created.
fn prepare_file_appender(
    file_path: &str,
) -> Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard), std::io::Error> {
    let log_path = std::path::Path::new(file_path);
    let log_dir = log_path.parent().unwrap_or(std::path::Path::new("."));
    let log_name = log_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, log_name);
    Ok(tracing_appender::non_blocking(file_appender))
}

/// Initialize tracing with stderr (status stream) and optional file (audit stream).
///
/// CKSPEC-OUT-001: stderr for status, file for audit.
/// CKSPEC-OUT-004: shadow logging — output.rs emits tracing events that land here.
///
/// Returns a guard that must be held until shutdown (flushes file writer).
pub fn init(config: &LogConfig) -> Result<LogGuard, Box<dyn std::error::Error>> {
    let stderr_filter = build_filter(&config.console_level, "info");

    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(true)
        .with_filter(stderr_filter);

    if config.file_enabled {
        let (non_blocking, guard) = prepare_file_appender(&config.file_path)?;

        let file_filter = build_filter(&config.file_level, "debug");

        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_filter(file_filter);

        registry().with(stderr_layer).with(file_layer).init();

        Ok(LogGuard {
            _guard: Some(guard),
        })
    } else {
        registry().with(stderr_layer).init();

        Ok(LogGuard { _guard: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LogConfig tests ─────────────────────────────────────────

    #[test]
    fn default_config_has_sane_values() {
        let config = LogConfig::default();
        assert_eq!(config.console_level, "info");
        assert!(!config.file_enabled);
        assert_eq!(config.file_path, "logs/app.log");
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

    // ── build_filter tests ──────────────────────────────────────

    #[test]
    fn build_filter_accepts_valid_level() {
        let filter = build_filter("debug", "info");
        // Filter should accept debug-level events
        assert_eq!(format!("{filter}"), "debug");
    }

    #[test]
    fn build_filter_handles_all_standard_levels() {
        for level in &["trace", "debug", "info", "warn", "error", "off"] {
            let filter = build_filter(level, "info");
            assert_eq!(
                format!("{filter}"),
                *level,
                "Filter should match level {level}"
            );
        }
    }

    #[test]
    fn build_filter_handles_off() {
        let filter = build_filter("off", "info");
        assert_eq!(format!("{filter}"), "off");
    }

    #[test]
    fn build_filter_handles_trace() {
        let filter = build_filter("trace", "info");
        assert_eq!(format!("{filter}"), "trace");
    }

    // ── prepare_file_appender tests ─────────────────────────────

    #[test]
    fn prepare_file_appender_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("subdir").join("test.log");

        let result = prepare_file_appender(log_path.to_str().unwrap());
        assert!(
            result.is_ok(),
            "Should create subdirectory and return appender"
        );
        assert!(
            dir.path().join("subdir").exists(),
            "Subdirectory should exist"
        );
    }

    #[test]
    fn prepare_file_appender_handles_existing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let result = prepare_file_appender(log_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn prepare_file_appender_fails_on_invalid_path() {
        // /dev/null/impossible is not a valid directory on any OS
        let result = prepare_file_appender("/dev/null/impossible/test.log");
        assert!(
            result.is_err(),
            "Should fail when directory can't be created"
        );
    }

    // ── LogGuard tests ──────────────────────────────────────────

    #[test]
    fn log_guard_without_file_has_no_worker() {
        let guard = LogGuard { _guard: None };
        assert!(guard._guard.is_none());
    }
}
