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
