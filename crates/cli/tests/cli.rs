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
        .args(["--json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""));
}

#[test]
fn ping_json_mode_has_command_name() {
    cmd()
        .args(["--json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"ping\""));
}

#[test]
fn ping_json_mode_has_data() {
    cmd()
        .args(["--json", "ping"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"message\": \"ckeletin-rust is alive\"",
        ));
}

#[test]
fn ping_json_mode_no_stderr_noise() {
    cmd()
        .args(["--json", "ping"])
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
