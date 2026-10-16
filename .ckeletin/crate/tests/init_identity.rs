//! `just init` commits and tags with a process-local identity.
//!
//! The fallback is exported only for this process. A host gitconfig, including
//! one guessed from the machine name, must not be required and must not be
//! copied into the new repository.

use std::fs;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|parent| parent.parent())
        .unwrap()
        .to_path_buf()
}

fn identity_function() -> String {
    let script = fs::read_to_string(workspace_root().join(".ckeletin/scripts/init.sh")).unwrap();
    let start = script
        .find("ckeletin_init_git_identity() {")
        .expect("init.sh must define ckeletin_init_git_identity");
    let rest = &script[start..];
    let end = rest.find("\n}\n").expect("identity function must close");
    let function = rest[..=end + 1].to_string();
    assert!(
        !function.contains("git config"),
        "the fallback must not write a sticky git identity"
    );
    function
}

fn run_identity(prelude: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("run.sh");
    let body = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
unset GIT_AUTHOR_NAME GIT_AUTHOR_EMAIL GIT_COMMITTER_NAME GIT_COMMITTER_EMAIL
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_SYSTEM=/dev/null
export HOME="{home}"
{function}
{prelude}
git init -b main
ckeletin_init_git_identity
git add -A
git commit --allow-empty -m scaffold
git tag -a v0.0.0 -m tag
echo "author=$(git log -1 --format=%an)"
echo "email=$(git log -1 --format=%ae)"
echo "tagger=$(git for-each-ref refs/tags/v0.0.0 --format='%(taggername) %(taggeremail)')"
if git config --local --get user.name; then
    echo "sticky-local-name" >&2
    exit 1
fi
"#,
        home = dir.path().display(),
        function = identity_function(),
        prelude = prelude,
    );
    fs::write(&script_path, body).unwrap();
    let output = Command::new("bash")
        .arg(&script_path)
        .current_dir(dir.path())
        .output()
        .expect("bash");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn init_commits_as_ckeletin_init_without_a_host_identity() {
    let got = run_identity("");
    assert!(
        got.contains("author=ckeletin-init\n"),
        "commit author must be the fallback, got:\n{got}"
    );
    assert!(
        got.contains("email=ckeletin-init@example.invalid\n"),
        "commit email must be the fallback, got:\n{got}"
    );
    assert!(
        got.contains("tagger=ckeletin-init <ckeletin-init@example.invalid>"),
        "annotated tagger must use the fallback, got:\n{got}"
    );
}

#[test]
fn init_keeps_an_identity_the_caller_already_exported() {
    let got = run_identity(
        "export GIT_AUTHOR_NAME=kept-name\nexport GIT_AUTHOR_EMAIL=kept@example.invalid\n",
    );
    assert!(
        got.contains("author=kept-name\n"),
        "caller identity must win, got:\n{got}"
    );
    assert!(
        got.contains("email=kept@example.invalid\n"),
        "caller email must win, got:\n{got}"
    );
}
