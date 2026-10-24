//! The update tells a consumer when lefthook.yml has no gitleaks command.

use std::fs;
use std::process::Command;

fn script() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("scripts/report-hook-drift.sh")
}

fn run_in(dir: &std::path::Path) -> String {
    let output = Command::new("bash")
        .arg(script())
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn missing_hook_file_prints_the_merge_line() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(dir.path());
    assert!(
        out.contains("lefthook.yml has no gitleaks pre-commit"),
        "{out}"
    );
    assert!(
        out.contains("just ckeletin-update does not change this file"),
        "{out}"
    );
}

#[test]
fn hook_without_gitleaks_prints_the_merge_line() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    fmt:\n      run: true\n",
    )
    .unwrap();
    let out = run_in(dir.path());
    assert!(out.contains("gitleaks"), "{out}");
}

#[test]
fn hook_with_gitleaks_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("lefthook.yml"),
        "commands:\n  gitleaks:\n    run: true\n",
    )
    .unwrap();
    assert_eq!(run_in(dir.path()), "");
}
