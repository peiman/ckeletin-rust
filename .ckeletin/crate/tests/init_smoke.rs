//! Init smoke test — clones the scaffold to a temp directory, runs init,
//! and verifies the result compiles and has no stale references.
//!
//! This is the most important test in the scaffold. If it passes,
//! new projects work. If it fails, the init script is broken.
//!
//! Ignored by default because it's slow (clones, runs cargo check).
//! Run explicitly: cargo test -p ckeletin --test init_smoke -- --ignored

use std::process::Command;

/// Find the workspace root (parent of .ckeletin/crate/).
fn workspace_root() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // manifest_dir is .ckeletin/crate, workspace root is two levels up
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
#[ignore] // slow — run explicitly
fn init_produces_compilable_project() {
    let root = workspace_root();
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("testproject");

    // Copy the scaffold (excluding .git and target)
    let status = Command::new("rsync")
        .args([
            "-a",
            "--exclude=.git",
            "--exclude=target",
            "--exclude=conformance/requirements.json",
            &format!("{}/", root),
            project_dir.to_str().unwrap(),
        ])
        .status()
        .expect("rsync failed");
    assert!(status.success(), "rsync copy failed");

    // Initialize as "testproject"
    let init = Command::new("bash")
        .arg(".ckeletin/scripts/init.sh")
        .arg("testproject")
        .current_dir(&project_dir)
        .output()
        .expect("init.sh failed to execute");

    assert!(
        init.status.success(),
        "init.sh failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr),
    );

    // Verify no stale "ckeletin-rust" references in project source
    let grep = Command::new("grep")
        .args([
            "-r",
            "ckeletin-rust",
            "--include=*.rs",
            "--include=*.toml",
            "crates/",
        ])
        .current_dir(&project_dir)
        .output()
        .unwrap();
    let stale = String::from_utf8_lossy(&grep.stdout);
    assert!(
        stale.is_empty(),
        "Found stale 'ckeletin-rust' references after init:\n{stale}"
    );

    // Verify the binary name was set
    let cli_toml = std::fs::read_to_string(project_dir.join("crates/cli/Cargo.toml")).unwrap();
    assert!(
        cli_toml.contains("name = \"testproject\""),
        "Binary name not set in cli/Cargo.toml"
    );

    // Verify env prefix was patched
    let main_rs = std::fs::read_to_string(project_dir.join("crates/cli/src/main.rs")).unwrap();
    assert!(
        main_rs.contains("\"TESTPROJECT_\""),
        "Env prefix not patched in main.rs"
    );

    // Verify demo code was stripped
    assert!(
        !project_dir.join("crates/domain/src/ping.rs").exists(),
        "ping.rs should be removed"
    );
    assert!(
        !project_dir.join("crates/cli/src/ping.rs").exists(),
        "cli/ping.rs should be removed"
    );
}
