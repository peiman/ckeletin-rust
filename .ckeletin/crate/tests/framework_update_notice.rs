// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! The framework-update notice compares `.ckeletin/VERSION` by semver and
//! rides `just ckeletin-health`, which `just check` already runs.
//!
//! A commit that does not change the version is not an update. A local
//! version ahead of upstream is not an update. The notice does not fail
//! the build, does not run when CI is set, honours the opt-out, and stays
//! quiet offline. `check json` prints current, latest, and update_available.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

fn script_path() -> PathBuf {
    workspace_root().join(".ckeletin/scripts/check-framework-update.sh")
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} failed to spawn: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn init_repo(dir: &Path, version: &str) {
    fs::create_dir_all(dir.join(".ckeletin")).unwrap();
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@ckeletin.test"]);
    git(dir, &["config", "user.name", "ckeletin test"]);
    fs::write(dir.join(".ckeletin/VERSION"), format!("{version}\n")).unwrap();
    fs::write(dir.join(".ckeletin/f"), "a\n").unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"down\"\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "c1"]);
}

/// Upstream at `up_version`, downstream clone at the same commit, then the
/// downstream version is rewritten to `down_version` when they differ.
fn pair(up_version: &str, down_version: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let downstream = tmp.path().join("downstream");
    fs::create_dir_all(&upstream).unwrap();
    init_repo(&upstream, up_version);
    git(
        tmp.path(),
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            downstream.to_str().unwrap(),
        ],
    );
    git(&downstream, &["config", "user.email", "test@ckeletin.test"]);
    git(&downstream, &["config", "user.name", "ckeletin test"]);
    git(
        &downstream,
        &["remote", "rename", "origin", "ckeletin-upstream"],
    );
    if down_version != up_version {
        fs::write(
            downstream.join(".ckeletin/VERSION"),
            format!("{down_version}\n"),
        )
        .unwrap();
        git(&downstream, &["add", "-A"]);
        git(&downstream, &["commit", "-m", "downstream version"]);
    }
    (tmp, downstream)
}

fn bump(upstream: &Path, version: &str) {
    fs::write(upstream.join(".ckeletin/VERSION"), format!("{version}\n")).unwrap();
    git(upstream, &["add", "-A"]);
    git(upstream, &["commit", "-m", "bump"]);
}

struct ScriptOut {
    status: i32,
    stdout: String,
}

fn run_script(dir: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> ScriptOut {
    let mut cmd = Command::new("bash");
    cmd.arg(script_path()).args(args).current_dir(dir);
    cmd.env_remove("CI");
    cmd.env_remove("CKELETIN_SKIP_UPDATE_CHECK");
    cmd.env("CKELETIN_UPDATE_CHECK_TTL", "0");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("script failed to spawn: {e}"));
    ScriptOut {
        status: out.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
    }
}

#[test]
fn notice_when_upstream_version_is_newer() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let downstream = tmp.path().join("downstream");
    fs::create_dir_all(&upstream).unwrap();
    init_repo(&upstream, "0.1.0");
    git(
        tmp.path(),
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            downstream.to_str().unwrap(),
        ],
    );
    git(&downstream, &["config", "user.email", "test@ckeletin.test"]);
    git(&downstream, &["config", "user.name", "ckeletin test"]);
    git(
        &downstream,
        &["remote", "rename", "origin", "ckeletin-upstream"],
    );
    bump(&upstream, "0.2.0");

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains("Update available"),
        "newer semver must notice.\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("just ckeletin-update"),
        "notice must name the update command.\n{}",
        out.stdout
    );
}

#[test]
fn notice_ignores_a_commit_that_does_not_change_version() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let downstream = tmp.path().join("downstream");
    fs::create_dir_all(&upstream).unwrap();
    init_repo(&upstream, "0.1.0");
    git(
        tmp.path(),
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            downstream.to_str().unwrap(),
        ],
    );
    git(&downstream, &["config", "user.email", "test@ckeletin.test"]);
    git(&downstream, &["config", "user.name", "ckeletin test"]);
    git(
        &downstream,
        &["remote", "rename", "origin", "ckeletin-upstream"],
    );
    fs::write(upstream.join(".ckeletin/f"), "a\nb\n").unwrap();
    git(&upstream, &["add", "-A"]);
    git(&upstream, &["commit", "-m", "file only"]);

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        !out.stdout.contains("Update available"),
        "a non-version commit is not an update.\n{}",
        out.stdout
    );
}

#[test]
fn notice_silent_when_local_is_ahead() {
    let (_keep, downstream) = pair("0.2.0", "0.2.0");
    let upstream = _keep.path().join("upstream");
    fs::write(downstream.join(".ckeletin/VERSION"), "0.3.0\n").unwrap();
    git(&downstream, &["add", "-A"]);
    git(&downstream, &["commit", "-m", "local ahead"]);
    bump(&upstream, "0.2.1");

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        !out.stdout.contains("Update available"),
        "local ahead is not an update.\n{}",
        out.stdout
    );
}

#[test]
fn notice_skips_in_ci_and_honours_opt_out() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let downstream = tmp.path().join("downstream");
    fs::create_dir_all(&upstream).unwrap();
    init_repo(&upstream, "0.1.0");
    git(
        tmp.path(),
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            downstream.to_str().unwrap(),
        ],
    );
    git(&downstream, &["config", "user.email", "test@ckeletin.test"]);
    git(&downstream, &["config", "user.name", "ckeletin test"]);
    git(
        &downstream,
        &["remote", "rename", "origin", "ckeletin-upstream"],
    );
    bump(&upstream, "0.2.0");

    let ci = run_script(&downstream, &[], &[("CI", "true")]);
    assert_eq!(ci.status, 0);
    assert!(!ci.stdout.contains("Update available"), "{}", ci.stdout);

    let opted = run_script(&downstream, &[], &[("CKELETIN_SKIP_UPDATE_CHECK", "1")]);
    assert_eq!(opted.status, 0);
    assert!(
        !opted.stdout.contains("Update available"),
        "{}",
        opted.stdout
    );
}

#[test]
fn notice_does_not_fail_offline_or_without_a_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("solo");
    fs::create_dir_all(&dir).unwrap();
    init_repo(&dir, "0.1.0");
    let missing = run_script(&dir, &[], &[]);
    assert_eq!(missing.status, 0, "{}", missing.stdout);
    assert!(!missing.stdout.contains("Update available"));

    git(
        &dir,
        &[
            "remote",
            "add",
            "ckeletin-upstream",
            tmp.path().join("missing.git").to_str().unwrap(),
        ],
    );
    let offline = run_script(&dir, &[], &[]);
    assert_eq!(offline.status, 0, "{}", offline.stdout);
    assert!(!offline.stdout.contains("Update available"));
}

#[test]
fn json_check_reports_current_latest_and_availability() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let downstream = tmp.path().join("downstream");
    fs::create_dir_all(&upstream).unwrap();
    init_repo(&upstream, "0.2.9");
    git(
        tmp.path(),
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            downstream.to_str().unwrap(),
        ],
    );
    git(&downstream, &["config", "user.email", "test@ckeletin.test"]);
    git(&downstream, &["config", "user.name", "ckeletin test"]);
    git(
        &downstream,
        &["remote", "rename", "origin", "ckeletin-upstream"],
    );
    bump(&upstream, "0.2.10");

    let out = run_script(
        &downstream,
        &["check", "json"],
        &[("CI", "true"), ("CKELETIN_SKIP_UPDATE_CHECK", "1")],
    );
    assert_eq!(out.status, 0, "{}", out.stdout);
    let body: serde_json::Value = serde_json::from_str(out.stdout.trim())
        .unwrap_or_else(|e| panic!("json parse: {e}\n{}", out.stdout));
    assert_eq!(body["current"], "0.2.9");
    assert_eq!(body["latest"], "0.2.10");
    assert_eq!(body["update_available"], true);
}

#[test]
fn json_check_is_false_when_local_is_ahead() {
    let (_keep, downstream) = pair("0.1.0", "0.4.0");
    let out = run_script(&downstream, &["check", "json"], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    let body: serde_json::Value = serde_json::from_str(out.stdout.trim())
        .unwrap_or_else(|e| panic!("json parse: {e}\n{}", out.stdout));
    assert_eq!(body["update_available"], false);
    assert_eq!(body["current"], "0.4.0");
    assert_eq!(body["latest"], "0.1.0");
}

#[test]
fn upstream_repository_is_exempt() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("up");
    fs::create_dir_all(&dir).unwrap();
    init_repo(&dir, "0.1.0");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"ckeletin-rust\"\nrepository = \"peiman/ckeletin-rust\"\n",
    )
    .unwrap();
    let notice = run_script(&dir, &[], &[]);
    assert_eq!(notice.status, 0);
    assert!(
        !notice.stdout.contains("Update available"),
        "{}",
        notice.stdout
    );

    let json = run_script(&dir, &["check", "json"], &[]);
    assert_eq!(json.status, 0, "{}", json.stdout);
    let body: serde_json::Value = serde_json::from_str(json.stdout.trim())
        .unwrap_or_else(|e| panic!("json parse: {e}\n{}", json.stdout));
    assert_eq!(body["applicable"], false);
}

#[test]
fn health_recipe_runs_the_notice() {
    let out = Command::new("just")
        .args(["--show", "ckeletin-health"])
        .current_dir(workspace_root())
        .output()
        .expect("just --show ckeletin-health");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "just --show failed.\n{text}");
    assert!(
        text.contains("check-framework-update.sh"),
        "ckeletin-health must call the shared update script so consumers get the notice after ckeletin-update.\n{text}"
    );
}

/// The notice stamp must not sit inside `.ckeletin/`. A file there makes
/// `git status --porcelain .ckeletin/` dirty and the next update refuses.
#[test]
fn notice_stamp_stays_out_of_the_framework_tree() {
    let (_tmp, downstream) = pair("0.1.0", "0.1.0");
    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        !downstream.join(".ckeletin/cache").exists(),
        "the update-check stamp must not live under .ckeletin/"
    );
    let stamp_out = Command::new("git")
        .args(["rev-parse", "--git-path", "ckeletin-last-update-check"])
        .current_dir(&downstream)
        .output()
        .unwrap();
    assert!(stamp_out.status.success());
    let stamp = String::from_utf8_lossy(&stamp_out.stdout)
        .trim()
        .to_string();
    let stamp_path = Path::new(&stamp);
    let stamp_path = if stamp_path.is_absolute() {
        stamp_path.to_path_buf()
    } else {
        downstream.join(stamp_path)
    };
    assert!(
        stamp_path.is_file(),
        "expected stamp at {}",
        stamp_path.display()
    );
    let status_out = Command::new("git")
        .args(["status", "--porcelain", "--", ".ckeletin/"])
        .current_dir(&downstream)
        .output()
        .unwrap();
    let porcelain = String::from_utf8_lossy(&status_out.stdout);
    assert!(
        porcelain.trim().is_empty(),
        "notice must not dirty .ckeletin/:\n{porcelain}"
    );
}

/// The note travels with the upstream tree. A project that already has this
/// script sees it from `just check` before it updates.
#[test]
fn notice_prints_the_upstream_update_note() {
    let (keep, downstream) = pair("0.1.0", "0.1.0");
    let upstream = keep.path().join("upstream");
    fs::write(
        upstream.join(".ckeletin/UPDATE_NOTE"),
        "Delete the retired stamp.\nrm -rf .ckeletin/cache\n",
    )
    .unwrap();
    bump(&upstream, "0.2.0");

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains("Update available: v0.1.0 → v0.2.0"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("rm -rf .ckeletin/cache"),
        "the notice must print the upstream note.\n{}",
        out.stdout
    );

    let check = run_script(&downstream, &["check"], &[]);
    assert_eq!(check.status, 0, "{}", check.stdout);
    assert!(
        check.stdout.contains("rm -rf .ckeletin/cache"),
        "an explicit check must print the upstream note.\n{}",
        check.stdout
    );
}

#[test]
fn notice_without_an_update_note_stays_the_command_lines() {
    let (keep, downstream) = pair("0.1.0", "0.1.0");
    bump(&keep.path().join("upstream"), "0.2.0");

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    let lines: Vec<&str> = out.stdout.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(
        lines,
        [
            "Update available: v0.1.0 → v0.2.0",
            "Run: just ckeletin-update"
        ]
    );
}

#[test]
fn up_to_date_notice_does_not_print_the_update_note() {
    let (keep, downstream) = pair("0.2.0", "0.2.0");
    let upstream = keep.path().join("upstream");
    fs::write(
        upstream.join(".ckeletin/UPDATE_NOTE"),
        "rm -rf .ckeletin/cache\n",
    )
    .unwrap();
    git(&upstream, &["add", "-A"]);
    git(&upstream, &["commit", "-m", "note only"]);

    let out = run_script(&downstream, &[], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(!out.stdout.contains("Update available"), "{}", out.stdout);
    assert!(
        !out.stdout.contains("rm -rf"),
        "a current framework must not print the note.\n{}",
        out.stdout
    );
}

#[test]
fn json_check_does_not_print_the_update_note() {
    let (keep, downstream) = pair("0.1.0", "0.1.0");
    let upstream = keep.path().join("upstream");
    fs::write(
        upstream.join(".ckeletin/UPDATE_NOTE"),
        "rm -rf .ckeletin/cache\n",
    )
    .unwrap();
    bump(&upstream, "0.2.0");

    let out = run_script(&downstream, &["check", "json"], &[]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    let body: serde_json::Value = serde_json::from_str(out.stdout.trim())
        .unwrap_or_else(|e| panic!("json parse: {e}\n{}", out.stdout));
    assert_eq!(body["update_available"], true);
    assert!(
        !out.stdout.contains("rm -rf"),
        "json output must stay one object.\n{}",
        out.stdout
    );
}
