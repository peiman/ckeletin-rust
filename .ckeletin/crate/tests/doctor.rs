// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! `ckeletin-doctor` smoke test.
//!
//! The doctor is an environment diagnostic: it reports the framework version and
//! the toolchain + tools the framework depends on. It is INFORMATIONAL — it must
//! exit 0 even when a tool is missing (it reports status, it does not gate the
//! build), so it is deliberately NOT part of `just check`. This test asserts it
//! runs and surfaces the key sections. Unlike the update self-guard, the doctor
//! is not upstream-specific, so it also runs cleanly inside an init'd project.

use std::process::Command;

fn workspace_root() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR"); // .ckeletin/crate
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn doctor_reports_environment_and_never_fails() {
    if !have("just") {
        eprintln!("SKIP doctor: `just` not on PATH");
        return;
    }

    let out = Command::new("just")
        .arg("ckeletin-doctor")
        .current_dir(workspace_root())
        .output()
        .expect("failed to run `just ckeletin-doctor`");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "ckeletin-doctor is informational and must exit 0.\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Key sections the doctor must surface.
    for expect in [
        "ckeletin framework v",
        "Toolchain",
        "Tools",
        "cargo-deny",
        "just",
        "Audit log:",
        "writable:",
    ] {
        assert!(
            stdout.contains(expect),
            "doctor output missing {expect:?}.\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
}

#[test]
fn doctor_json_is_machine_readable() {
    // For autonomous operation (workhorse driving ckeletin), `ckeletin-doctor
    // json` must emit a single valid JSON object an agent can parse — framework
    // version, toolchain, and tool presence as booleans.
    if !have("just") {
        eprintln!("SKIP doctor json: `just` not on PATH");
        return;
    }

    let out = Command::new("just")
        .args(["ckeletin-doctor", "json"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run `just ckeletin-doctor json`");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "doctor json must exit 0.\nstdout: {stdout}"
    );

    // Must be exactly one valid JSON object.
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("doctor json is not valid JSON: {e}\nstdout: {stdout}"));

    assert!(
        v.get("framework_version")
            .and_then(|x| x.as_str())
            .is_some(),
        "json missing framework_version: {v}"
    );
    assert!(
        v.pointer("/toolchain/pinned")
            .and_then(|x| x.as_str())
            .is_some(),
        "json missing toolchain.pinned: {v}"
    );
    // tool presence must be booleans an agent can branch on.
    assert!(
        v.pointer("/tools/cargo-deny")
            .and_then(|x| x.as_bool())
            .is_some(),
        "json missing boolean tools.cargo-deny: {v}"
    );

    // rustfmt and clippy components (required by `just check`) must appear in
    // JSON parity with the text mode that has always reported them.
    assert!(
        v.pointer("/components/rustfmt")
            .and_then(|x| x.as_bool())
            .is_some(),
        "json missing boolean components.rustfmt: {v}"
    );
    assert!(
        v.pointer("/components/clippy")
            .and_then(|x| x.as_bool())
            .is_some(),
        "json missing boolean components.clippy: {v}"
    );
    // The path is the consumer's, so this copied test must not hardcode the
    // framework binary name. The resolver script is the source of that path.
    let script =
        std::path::Path::new(&workspace_root()).join(".ckeletin/scripts/audit-log-path.py");
    let resolved = Command::new("python3")
        .arg(&script)
        .current_dir(workspace_root())
        .output()
        .expect("python3 audit-log-path.py");
    let expected = String::from_utf8_lossy(&resolved.stdout);
    let expected = expected.trim();
    let path = v
        .pointer("/audit_log/path")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if resolved.status.success() && !expected.is_empty() {
        assert_eq!(
            path, expected,
            "audit_log.path must be the consumer resolver output\n{v}"
        );
    } else {
        assert_eq!(
            path, "logs/app.log",
            "doctor falls back to logs/app.log when the resolver does not run\n{v}"
        );
    }
    assert!(
        v.pointer("/audit_log/writable")
            .and_then(|x| x.as_bool())
            .is_some(),
        "json missing boolean audit_log.writable: {v}"
    );
}

#[test]
fn doctor_json_and_text_report_same_components() {
    // Parity test: the text mode and JSON mode must both report rustfmt and
    // clippy component presence. This catches the case where one mode is
    // updated without the other (finding #10 from the 2026-06-09 code review).
    if !have("just") {
        eprintln!("SKIP doctor_json_and_text_report_same_components: `just` not on PATH");
        return;
    }

    let root = workspace_root();

    let text_out = Command::new("just")
        .arg("ckeletin-doctor")
        .current_dir(&root)
        .output()
        .expect("failed to run `just ckeletin-doctor`");
    let text_stdout = String::from_utf8_lossy(&text_out.stdout);

    let json_out = Command::new("just")
        .args(["ckeletin-doctor", "json"])
        .current_dir(&root)
        .output()
        .expect("failed to run `just ckeletin-doctor json`");
    let json_stdout = String::from_utf8_lossy(&json_out.stdout);

    let v: serde_json::Value = serde_json::from_str(json_stdout.trim())
        .unwrap_or_else(|e| panic!("doctor json is not valid JSON: {e}\njson: {json_stdout}"));

    // Text mode must mention both components.
    assert!(
        text_stdout.contains("rustfmt component"),
        "text mode must mention 'rustfmt component'.\nstdout: {text_stdout}"
    );
    assert!(
        text_stdout.contains("clippy component"),
        "text mode must mention 'clippy component'.\nstdout: {text_stdout}"
    );

    // JSON must have components.rustfmt and components.clippy as booleans.
    let rustfmt_json = v.pointer("/components/rustfmt").and_then(|x| x.as_bool());
    let clippy_json = v.pointer("/components/clippy").and_then(|x| x.as_bool());

    assert!(
        rustfmt_json.is_some(),
        "json must have boolean /components/rustfmt.\njson: {v}"
    );
    assert!(
        clippy_json.is_some(),
        "json must have boolean /components/clippy.\njson: {v}"
    );

    // Both modes must agree on installed/not-installed for both components.
    // Text mode says "installed" vs "NOT FOUND", JSON mode uses true/false.
    let rustfmt_text_installed = text_stdout.contains("rustfmt component: installed");
    let clippy_text_installed = text_stdout.contains("clippy component: installed");

    assert_eq!(
        rustfmt_text_installed,
        rustfmt_json.unwrap(),
        "rustfmt presence disagrees between text ({rustfmt_text_installed}) and json ({:?})",
        rustfmt_json
    );
    assert_eq!(
        clippy_text_installed,
        clippy_json.unwrap(),
        "clippy presence disagrees between text ({clippy_text_installed}) and json ({:?})",
        clippy_json
    );
}

/// The audit path uses the consumer's `[[bin]]` name, not the framework's.
#[test]
fn audit_path_uses_the_consumer_bin_name() {
    if !have("python3") {
        eprintln!("SKIP audit_path_uses_the_consumer_bin_name: `python3` not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"workhorse\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"workhorse\"\npath = \"src/main.rs\"\n",
    )
    .unwrap();
    let script =
        std::path::Path::new(&workspace_root()).join(".ckeletin/scripts/audit-log-path.py");
    let config_home = dir.path().join("config");
    let out = Command::new("python3")
        .arg(&script)
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .env_remove("CKELETIN_LOG_FILE_PATH")
        .env_remove("CKELETIN_LOG_LOCATION")
        .output()
        .expect("python3 audit-log-path.py");
    let path = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "audit path script failed\nstdout: {path}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        path.contains("workhorse"),
        "path must use the consumer bin name:\n{path}"
    );
    assert!(
        !path.contains("ckeletin-rust"),
        "path must not use the framework bin name:\n{path}"
    );
}

fn audit_script() -> std::path::PathBuf {
    std::path::Path::new(&workspace_root()).join(".ckeletin/scripts/audit-log-path.py")
}

fn write_bin_project(dir: &std::path::Path, rust: &str) {
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"workhorse\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"workhorse\"\npath = \"src/main.rs\"\n",
    )
    .unwrap();
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), rust).unwrap();
}

fn run_audit(dir: &std::path::Path, env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new("python3");
    cmd.arg(audit_script()).current_dir(dir).env("HOME", dir);
    for key in [
        "CKELETIN_LOG_FILE_PATH",
        "CKELETIN_LOG_LOCATION",
        "WORKHORSE_LOG_FILE_PATH",
        "WORKHORSE_LOG_LOCATION",
        "ACME_LOG_FILE_PATH",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "APPDATA",
    ] {
        cmd.env_remove(key);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output().expect("python3 audit-log-path.py")
}

const WORKHORSE_CALLS: &str = r#"
// Config::load(None, "NOPE_")
const SAMPLE: &str = "Config::load(None, \"NOPE_\")";
fn welcome() {
    let _ = Config::load(None, "WORKHORSE_");
}
fn run() {
    let _ = Config::load(
        cli.config.as_deref(),
        "WORKHORSE_",
    );
}
fn ignored() {
    Config::load_with_project_toml(None, "NOPE_");
}
"#;

/// The prefix is the second argument of `Config::load`, not `CKELETIN_` and
/// not a name hardcoded for one consumer.
#[test]
fn env_prefix_is_the_second_config_load_argument() {
    if !have("python3") {
        eprintln!("SKIP env_prefix_is_the_second_config_load_argument: `python3` not on PATH");
        return;
    }
    let script = std::fs::read_to_string(audit_script()).unwrap();
    assert!(
        !script.contains("WORKHORSE_"),
        "the resolver must not hardcode a consumer prefix"
    );

    let dir = tempfile::tempdir().unwrap();
    write_bin_project(dir.path(), WORKHORSE_CALLS);
    let config_home = dir.path().join("cfg");
    let out = run_audit(
        dir.path(),
        &[
            ("XDG_CONFIG_HOME", config_home.to_str().unwrap()),
            ("WORKHORSE_LOG_FILE_PATH", "logs/consumer-override.log"),
            ("WORKHORSE_LOG_LOCATION", "config"),
            ("CKELETIN_LOG_FILE_PATH", "logs/from-scaffold.log"),
            ("CKELETIN_LOG_LOCATION", "platform"),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "resolver failed\nstdout: {stdout}\nstderr: {stderr}"
    );
    let expected = config_home.join("workhorse/logs/consumer-override.log");
    assert_eq!(
        stdout.trim(),
        expected.to_str().unwrap(),
        "stderr: {stderr}"
    );

    let absolute = run_audit(
        dir.path(),
        &[
            ("WORKHORSE_LOG_FILE_PATH", "/var/audit/absolute.log"),
            ("CKELETIN_LOG_FILE_PATH", "/var/audit/scaffold.log"),
        ],
    );
    let absolute_out = String::from_utf8_lossy(&absolute.stdout);
    assert!(
        absolute.status.success(),
        "absolute path failed\nstdout: {absolute_out}\nstderr: {}",
        String::from_utf8_lossy(&absolute.stderr)
    );
    assert_eq!(absolute_out.trim(), "/var/audit/absolute.log");
}

#[test]
fn env_prefix_calls_must_agree_and_be_literals() {
    if !have("python3") {
        eprintln!("SKIP env_prefix_calls_must_agree_and_be_literals: `python3` not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    write_bin_project(
        dir.path(),
        "fn a() { Config::load(None, \"WORKHORSE_\"); }\nfn b() { Config::load(None, \"ACME_\"); }\n",
    );
    let disagree = run_audit(dir.path(), &[]);
    let stderr = String::from_utf8_lossy(&disagree.stderr);
    assert!(
        !disagree.status.success(),
        "disagreeing prefixes must fail\nstderr: {stderr}"
    );
    assert!(stderr.contains("WORKHORSE_"), "{stderr}");
    assert!(stderr.contains("ACME_"), "{stderr}");

    write_bin_project(
        dir.path(),
        "fn a() { let prefix = \"WORKHORSE_\"; Config::load(None, prefix); }\n",
    );
    let variable = run_audit(dir.path(), &[]);
    let stderr = String::from_utf8_lossy(&variable.stderr);
    assert!(
        !variable.status.success(),
        "a non-literal prefix must fail\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("string literal"),
        "stderr must name the requirement\n{stderr}"
    );
}

#[test]
fn env_prefix_falls_back_to_the_scaffold_name() {
    if !have("python3") {
        eprintln!("SKIP env_prefix_falls_back_to_the_scaffold_name: `python3` not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"workhorse\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"workhorse\"\npath = \"src/main.rs\"\n",
    )
    .unwrap();
    let config_home = dir.path().join("cfg");
    let out = run_audit(
        dir.path(),
        &[
            ("XDG_CONFIG_HOME", config_home.to_str().unwrap()),
            ("CKELETIN_LOG_FILE_PATH", "logs/from-scaffold.log"),
            ("WORKHORSE_LOG_FILE_PATH", "logs/consumer-override.log"),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "scaffold fallback failed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let expected = config_home.join("workhorse/logs/from-scaffold.log");
    assert_eq!(stdout.trim(), expected.to_str().unwrap());
}
