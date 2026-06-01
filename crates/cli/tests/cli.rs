use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    let mut c = Command::cargo_bin("ckeletin-rust").unwrap();
    // These tests don't care about the audit log; disable it so runs don't
    // write into the developer's real ~/.config dir. Audit-specific tests opt
    // back in via `audit_cmd`, redirecting the log to a temp dir.
    c.arg("--no-audit");
    c
}

/// A command with audit logging ENABLED but its base dir (XDG config home)
/// redirected into `xdg`, so the default `~/.config/<app>/logs` lands in a
/// temp dir instead of the developer's real config dir.
fn audit_cmd(xdg: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("ckeletin-rust").unwrap();
    c.env("XDG_CONFIG_HOME", xdg);
    c
}

#[test]
fn help_shows_usage() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ckeletin-rust"));
}

#[test]
fn version_shows_version() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn ping_human_mode() {
    cmd()
        .arg("ping")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pong! ckeletin-rust is alive"));
}

#[test]
fn ping_json_mode_has_success_status() {
    cmd()
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""));
}

#[test]
fn ping_json_mode_has_command_name() {
    cmd()
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"ping\""));
}

#[test]
fn ping_json_mode_has_data() {
    cmd()
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"message\": \"ckeletin-rust is alive\"",
        ));
}

#[test]
fn ping_json_mode_no_stderr_noise() {
    cmd()
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn no_subcommand_shows_error() {
    cmd().assert().failure();
}

#[test]
fn unknown_subcommand_fails() {
    cmd().arg("nonexistent").assert().failure();
}

// ── Error path tests (robustness) ─────────────────────────────

#[test]
fn json_mode_bad_config_produces_json_error_on_stdout() {
    // CKSPEC-OUT-002: errors in JSON mode MUST be JSON envelopes on stdout
    cmd()
        .args([
            "--output",
            "json",
            "--config",
            "/nonexistent/config.toml",
            "ping",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"status\": \"error\""))
        .stdout(predicate::str::contains("\"error\""));
}

#[test]
fn json_mode_error_envelope_identifies_failing_subcommand() {
    // CKSPEC-OUT-003: the envelope's `command` field MUST identify
    // the failing subcommand so downstream consumers can correlate
    // envelopes to commands. A hardcoded placeholder (e.g. "init")
    // violates the spirit of this requirement even though the envelope
    // is structurally valid.
    cmd()
        .args([
            "--output",
            "json",
            "--config",
            "/nonexistent/config.toml",
            "ping",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"status\": \"error\""))
        .stdout(predicate::str::contains("\"command\": \"ping\""));
}

#[test]
fn json_mode_error_has_no_stderr() {
    // JSON mode: stderr must be clean even on errors
    cmd()
        .args([
            "--output",
            "json",
            "--config",
            "/nonexistent/config.toml",
            "ping",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty());
}

#[test]
fn human_mode_error_goes_to_stderr() {
    // Human mode: errors go to stderr, not stdout
    cmd()
        .args(["--config", "/nonexistent/config.toml", "ping"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn json_verbose_no_stderr_leak() {
    // --json + --verbose: verbose must not leak debug logs to stderr
    cmd()
        .args(["--output", "json", "--verbose", "ping"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ── Audit log tests (CKSPEC-OUT-004 — audit on by default) ──
// Audit defaults to ~/.config/<app>/logs; these redirect XDG_CONFIG_HOME to a
// temp dir so the log lands there, not in the developer's real config dir.
// The "ckeletin-rust" path segment is the binary name (CARGO_BIN_NAME), which
// `just init` renames alongside this file.

#[test]
fn audit_log_written_under_config_home_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    audit_cmd(tmp.path()).arg("ping").assert().success();
    assert!(
        tmp.path().join("ckeletin-rust/logs").is_dir(),
        "audit log should be created under <config>/<app>/logs by default"
    );
}

#[test]
fn no_audit_flag_disables_the_log_file() {
    let tmp = tempfile::tempdir().unwrap();
    audit_cmd(tmp.path())
        .args(["--no-audit", "ping"])
        .assert()
        .success();
    assert!(
        !tmp.path().join("ckeletin-rust").exists(),
        "--no-audit should write no audit log"
    );
}

#[test]
fn first_run_prints_audit_notice_to_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    audit_cmd(tmp.path())
        .arg("ping")
        .assert()
        .success()
        .stderr(predicate::str::contains("audit log"));
}

#[test]
fn audit_notice_is_silent_on_later_runs() {
    let tmp = tempfile::tempdir().unwrap();
    // First run creates the log dir and prints the one-time notice.
    audit_cmd(tmp.path()).arg("ping").assert().success();
    // Second run: the dir already exists, so no notice.
    audit_cmd(tmp.path())
        .arg("ping")
        .assert()
        .success()
        .stderr(predicate::str::contains("audit log").not());
}

#[test]
fn json_mode_suppresses_the_audit_notice() {
    let tmp = tempfile::tempdir().unwrap();
    audit_cmd(tmp.path())
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
