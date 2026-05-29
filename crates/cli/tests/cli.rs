use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("ckeletin-rust").unwrap()
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

// ── Audit log tests (CKSPEC-OUT-004 — audit is on by default) ──
// These run in a temp cwd so the audit log lands there, not in the repo.

#[test]
fn audit_log_written_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    cmd().current_dir(tmp.path()).arg("ping").assert().success();
    assert!(
        tmp.path().join("logs").is_dir(),
        "audit log directory should be created by default"
    );
}

#[test]
fn no_audit_flag_disables_the_log_file() {
    let tmp = tempfile::tempdir().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["--no-audit", "ping"])
        .assert()
        .success();
    assert!(
        !tmp.path().join("logs").exists(),
        "--no-audit should write no audit log"
    );
}

#[test]
fn first_run_prints_audit_notice_to_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    cmd()
        .current_dir(tmp.path())
        .arg("ping")
        .assert()
        .success()
        .stderr(predicate::str::contains("audit log"));
}

#[test]
fn audit_notice_is_silent_on_later_runs() {
    let tmp = tempfile::tempdir().unwrap();
    // First run creates the log dir and prints the one-time notice.
    cmd().current_dir(tmp.path()).arg("ping").assert().success();
    // Second run: the dir already exists, so no notice.
    cmd()
        .current_dir(tmp.path())
        .arg("ping")
        .assert()
        .success()
        .stderr(predicate::str::contains("audit log").not());
}

#[test]
fn json_mode_suppresses_the_audit_notice() {
    let tmp = tempfile::tempdir().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["--output", "json", "ping"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
