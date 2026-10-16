//! The private-home launcher is a Python program. This runs its suite.

use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|parent| parent.parent())
        .unwrap()
        .to_path_buf()
}

#[test]
fn private_home_launcher_suite() {
    let root = workspace_root();
    let output = Command::new("python3")
        .arg(root.join(".ckeletin/scripts/test_runner_test.py"))
        .current_dir(&root)
        .output()
        .expect("python3");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
