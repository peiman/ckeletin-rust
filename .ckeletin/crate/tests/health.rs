// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! `ckeletin-health` exit-code tests.
//!
//! ## Finding #5 — ckeletin-health exit code (MEDIUM)
//! `ckeletin-health` previously exited 0 even when the workspace was BROKEN
//! (the `|| echo "Workspace: BROKEN"` construct swallows the exit code).
//! Fix: use an if/else that calls `exit 1` on compile failure.
//!
//! The fixture is a tiny crate. The health recipe still checks the real
//! workspace when `just check` runs it. These tests must not cargo-check
//! that workspace a second time.

use std::{path::Path, process::Command};

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn framework_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR"); // .ckeletin/crate
    Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

/// Git env for hermetic commits (CI runners have no identity).
fn git_env() -> [(&'static str, &'static str); 4] {
    [
        ("GIT_AUTHOR_NAME", "ckeletin test"),
        ("GIT_AUTHOR_EMAIL", "test@ckeletin.test"),
        ("GIT_COMMITTER_NAME", "ckeletin test"),
        ("GIT_COMMITTER_EMAIL", "test@ckeletin.test"),
    ]
}

/// A one-member workspace plus the real health recipe.
///
/// `cargo check` writes under the fixture's `target/`. That directory is not
/// the consumer workspace's build, so the check does not take the parent
/// `target/debug/.cargo-lock`.
fn health_fixture(broken: bool) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path();
    let app = project.join("app/src");
    let ck = project.join(".ckeletin");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&ck).unwrap();
    std::fs::write(
        project.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"app\"]\n",
    )
    .unwrap();
    std::fs::write(
        project.join("app/Cargo.toml"),
        "[package]\nname = \"health-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let source = if broken {
        "this is not valid rust @@@\n"
    } else {
        "pub fn ok() {}\n"
    };
    std::fs::write(app.join("lib.rs"), source).unwrap();
    let framework = framework_root().join(".ckeletin");
    std::fs::copy(framework.join("Justfile"), ck.join("Justfile")).unwrap();
    std::fs::write(ck.join("VERSION"), "0.0.0\n").unwrap();
    std::fs::write(project.join("Justfile"), "import '.ckeletin/Justfile'\n").unwrap();

    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(project)
        .status()
        .unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(project)
        .status()
        .unwrap();
    let commit = Command::new("git")
        .args(["commit", "-m", "fixture"])
        .current_dir(project)
        .envs(git_env())
        .status()
        .unwrap();
    assert!(commit.success(), "fixture commit failed");
    tmp
}

fn run_health(project: &Path) -> std::process::Output {
    let target = project.join("target");
    let build = target.join("build");
    std::fs::create_dir_all(&build).unwrap();
    Command::new("just")
        .arg("ckeletin-health")
        .current_dir(project)
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_BUILD_BUILD_DIR", &build)
        .output()
        .expect("failed to run `just ckeletin-health`")
}

/// `just ckeletin-health` on a compiling tree must exit 0 and report
/// "Workspace: compiles". This is the test `ckeletin-health-lock` runs.
#[test]
fn health_exits_zero_on_clean_tree() {
    if !have("just") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!("FAIL health_exits_zero_on_clean_tree: `just` required on CI");
        }
        eprintln!("SKIP health_exits_zero_on_clean_tree: `just` not on PATH");
        return;
    }

    let fixture = health_fixture(false);
    let out = run_health(fixture.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "ckeletin-health must exit 0 on a clean compiling tree.\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("Workspace: compiles"),
        "ckeletin-health must report 'Workspace: compiles'.\nstdout: {stdout}"
    );
}

/// `just ckeletin-health` on a crate with a compile error must exit non-zero
/// and report "Workspace: BROKEN".
#[test]
fn health_exits_nonzero_on_broken_workspace() {
    if !have("just") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!("FAIL health_exits_nonzero_on_broken_workspace: `just` required on CI");
        }
        eprintln!("SKIP health_exits_nonzero_on_broken_workspace: `just` not on PATH");
        return;
    }

    let fixture = health_fixture(true);
    let out = run_health(fixture.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "ckeletin-health must exit non-zero when workspace is BROKEN.\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("Workspace: BROKEN"),
        "ckeletin-health must report 'Workspace: BROKEN'.\nstdout: {stdout}"
    );
}
