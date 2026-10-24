//! The scan config is the root file when the project has one.

use std::fs;
use std::process::Command;

fn script() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("scripts/gitleaks-config.sh")
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
fn root_config_wins_when_present() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".gitleaks.toml"), "title = \"project\"\n").unwrap();
    assert_eq!(run_in(dir.path()), ".gitleaks.toml\n");
}

#[test]
fn framework_config_is_the_default() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run_in(dir.path()), ".ckeletin/configs/gitleaks.toml\n");
}
