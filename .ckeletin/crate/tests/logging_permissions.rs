// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! Integration tests for audit log file/directory permissions.
//!
//! Runs as a separate test binary so it can call logging::init() once
//! without conflicting with other test files that also set the global
//! tracing subscriber.

use ckeletin::logging::{init, LogConfig};
use std::fs;

#[cfg(unix)]
#[test]
fn log_directory_and_file_have_restricted_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(1);
    if uid == 0 {
        eprintln!("SKIP: running as root, permission tests are unreliable");
        return;
    }

    let base = tempfile::tempdir().unwrap();
    let log_path = base.path().join("logs").join("app.log");
    let config = LogConfig {
        console_level: "off".to_string(),
        file_enabled: true,
        file_path: log_path.to_str().unwrap().to_string(),
        file_level: "debug".to_string(),
    };

    let guard = init(&config).expect("init must succeed");
    drop(guard);

    // Directory must be 0700 (owner-only).
    let log_dir = base.path().join("logs");
    let dir_mode = fs::metadata(&log_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "audit log directory must be 0700, got {dir_mode:o}"
    );

    // At least one log file must exist with mode 0600.
    let entries: Vec<_> = fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!entries.is_empty(), "at least one log file must exist");
    for entry in &entries {
        let file_mode = fs::metadata(entry.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            file_mode,
            0o600,
            "audit log file {:?} must be 0600, got {file_mode:o}",
            entry.path()
        );
    }
}
