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
    ] {
        assert!(
            stdout.contains(expect),
            "doctor output missing {expect:?}.\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
}
