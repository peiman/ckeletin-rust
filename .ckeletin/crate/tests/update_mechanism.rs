// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! Hermetic update-mechanism tests.
//!
//! These tests build minimal consumer + upstream git fixtures in temp dirs and
//! exercise the real `just ckeletin-update` / `ckeletin-update-check-compatibility`
//! shell recipes through real git operations — no string-assertion about the
//! Justfile text, only observed file-system and process outcomes.
//!
//! ## What is tested here (findings from 2026-06-09 code review)
//!
//! ### Finding #1 — wholesale replacement (HIGH)
//! `git checkout <ref> -- .ckeletin/` does NOT delete files that are absent from
//! the ref.  The fix is `git restore --source=<ref> --staged --worktree --
//! .ckeletin/`.  Regression: upstream deletes a file → consumer update removes it.
//!
//! ### Finding #2 — rollback/restore leaves added files staged (HIGH)
//! `git checkout HEAD -- .ckeletin/` does NOT remove staged-but-not-committed
//! files. The fix uses `git restore --source=HEAD --staged --worktree` in both
//! the tier-1 rollback and the check-compatibility restore() trap.
//! Regression (a): check-compatibility across a file-adding release leaves
//! `git status --porcelain .ckeletin/` empty.
//! Regression (b): a forced compile-fail update rolls back cleanly; the
//! CKELETIN_UPDATE_RESULT JSON contains `"rolled_back":true` and is valid.
//!
//! ### Finding #3 — tag-pinned updates (MEDIUM)
//! `ckeletin-upstream/v0.2.0` is not a valid ref (tags are `refs/tags/`, not
//! `refs/remotes/<remote>/`). Fix: fetch the tag explicitly, use FETCH_HEAD.
//! Regression: `ckeletin-update version=<tag>` succeeds when the upstream has
//! that tag.
//!
//! ### Finding #4 — machine contract coverage (MEDIUM)
//! CKELETIN_UPDATE_RESULT must be emitted and parseable on success and
//! compile-failed paths.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR"); // .ckeletin/crate
    Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

/// Run `git` in `dir`, asserting success. Returns trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} failed to spawn: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed (exit {:?})\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
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

/// Returns the path of the upstream work tree (sibling of the bare repo).
fn upstream_work(upstream_bare: &Path) -> PathBuf {
    upstream_bare.parent().unwrap().join("upstream_work")
}

/// Build a bare upstream repo at `upstream_bare`.
///
/// Layout of `.ckeletin/`:
/// - `VERSION` = `version`
/// - `sentinel.txt` = `"upstream"` (canary file for deletion tests)
/// - `crate/` — minimal valid Rust workspace member
/// - `Justfile` — stub so consumer `import '.ckeletin/Justfile'` parses
///
/// Strategy:
/// 1. Create a bare repo at `upstream_bare`.
/// 2. Clone it as a normal work tree (`upstream_work`).
/// 3. Populate & commit in the work tree, push to bare — so `origin` in the
///    work tree always refers to the bare clone.
fn make_upstream(upstream_bare: &Path, version: &str) {
    let parent = upstream_bare.parent().unwrap();
    let work = upstream_work(upstream_bare);

    // 1. Create bare repo.
    git(
        parent,
        &[
            "init",
            "--bare",
            "-b",
            "main",
            upstream_bare.to_str().unwrap(),
        ],
    );

    // 2. Clone bare → work tree.
    git(
        parent,
        &[
            "clone",
            upstream_bare.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    git(&work, &["config", "user.name", "ckeletin test"]);
    git(&work, &["config", "user.email", "test@ckeletin.test"]);

    // 3. Populate.
    let ck = work.join(".ckeletin");
    let crate_dir = ck.join("crate").join("src");
    std::fs::create_dir_all(&crate_dir).unwrap();

    std::fs::write(
        ck.join("crate").join("Cargo.toml"),
        "[package]\nname = \"ckeletin\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(crate_dir.join("lib.rs"), "// framework\n").unwrap();
    std::fs::write(ck.join("VERSION"), version).unwrap();
    std::fs::write(ck.join("sentinel.txt"), "upstream").unwrap();
    // Same Justfile and local-diff script the consumer will run. `ckeletin-update`
    // applies no name rewrite, so a clean derived tree must match this commit
    // byte for byte or the update stops.
    copy_framework_update_files(&ck);

    git(&work, &["add", "-A"]);
    Command::new("git")
        .args(["commit", "-m", "upstream initial"])
        .current_dir(&work)
        .envs(git_env())
        .status()
        .unwrap();
    git(&work, &["push", "origin", "main"]);
}

/// Extend the upstream with a new commit (adds/updates/deletes files under the
/// work tree, then pushes to the bare repo).
fn upstream_add_commit(upstream_bare: &Path, msg: &str, add: &[(&str, &str)], delete: &[&str]) {
    let work = upstream_work(upstream_bare);
    for (rel, content) in add {
        let dest = work.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&dest, content).unwrap();
    }
    for rel in delete {
        let _ = std::fs::remove_file(work.join(rel));
        git(&work, &["rm", "--ignore-unmatch", rel]);
    }
    git(&work, &["add", "-A"]);
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(&work)
        .envs(git_env())
        .status()
        .unwrap();
    git(&work, &["push", "origin", "main"]);
}

/// Tag the current HEAD of the upstream work tree and push the tag to the bare.
///
/// Uses `-c tag.gpgSign=false` so the test works on hosts with GPG signing
/// enabled globally (e.g. peiman's dev machine with `tag.gpgSign = true`).
fn upstream_tag(upstream_bare: &Path, tag: &str) {
    let work = upstream_work(upstream_bare);
    // Lightweight tag: pass -c tag.gpgSign=false to override host config.
    let out = Command::new("git")
        .args(["-c", "tag.gpgSign=false", "tag", tag])
        .current_dir(&work)
        .envs(git_env())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git tag {tag} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    git(&work, &["push", "origin", tag]);
}

/// Build a minimal consumer project in `consumer_path`.
///
/// The consumer has:
/// - Root Cargo.toml (workspace, name rewritten away from `peiman/ckeletin-rust`)
/// - `.ckeletin/VERSION` (initial version)
/// - `.ckeletin/sentinel.txt` (present from the initial scaffold)
/// - A minimal `.ckeletin/Justfile`
/// - A `ckeletin-upstream` remote pointing at `upstream_bare`
/// - One committed state (so rollback has a `HEAD` to restore to)
fn make_consumer(consumer_path: &Path, upstream_bare: &Path, initial_version: &str) {
    std::fs::create_dir_all(consumer_path).unwrap();
    git(consumer_path, &["init", "-b", "main"]);
    git(consumer_path, &["config", "user.name", "ckeletin test"]);
    git(
        consumer_path,
        &["config", "user.email", "test@ckeletin.test"],
    );

    // Workspace Cargo.toml — slug is NOT the upstream slug (simulates an init'd project).
    std::fs::write(
        consumer_path.join("Cargo.toml"),
        "[workspace]\nmembers = [\".ckeletin/crate\"]\n\n[workspace.metadata]\nrepository = \"https://github.com/peiman/testproject\"\n",
    )
    .unwrap();

    // Bootstrap .ckeletin/ matching the upstream's current state.
    let ck = consumer_path.join(".ckeletin");
    std::fs::create_dir_all(&ck).unwrap();

    let crate_dir = ck.join("crate");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"ckeletin\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(crate_dir.join("src").join("lib.rs"), "// framework\n").unwrap();

    std::fs::write(ck.join("VERSION"), initial_version).unwrap();
    std::fs::write(ck.join("sentinel.txt"), "upstream").unwrap();
    std::fs::write(
        ck.join("Justfile"),
        "# minimal upstream justfile for tests\n",
    )
    .unwrap();

    git(consumer_path, &["add", "-A"]);
    Command::new("git")
        .args(["commit", "-m", "initial scaffold"])
        .current_dir(consumer_path)
        .envs(git_env())
        .status()
        .unwrap();

    // Wire the upstream remote.
    git(
        consumer_path,
        &[
            "remote",
            "add",
            "ckeletin-upstream",
            upstream_bare.to_str().unwrap(),
        ],
    );
}

/// Parse the CKELETIN_UPDATE_RESULT line from combined stdout+stderr output.
/// Returns the JSON string after the `=` sign.
fn parse_update_result(output: &str) -> serde_json::Value {
    let line = output
        .lines()
        .find(|l| l.starts_with("CKELETIN_UPDATE_RESULT="))
        .unwrap_or_else(|| panic!("CKELETIN_UPDATE_RESULT not found in output:\n{output}"));
    let json_str = line.strip_prefix("CKELETIN_UPDATE_RESULT=").unwrap();
    serde_json::from_str(json_str)
        .unwrap_or_else(|e| panic!("CKELETIN_UPDATE_RESULT is not valid JSON: {e}\nLine: {line}"))
}

// ── Finding #1 — wholesale replacement ────────────────────────────────────────

/// After an update that deletes `.ckeletin/sentinel.txt` upstream, the consumer
/// must NOT have that file in its working tree.
#[test]
fn update_deletes_files_removed_upstream() {
    if !have("just") || !have("rsync") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!("FAIL update_deletes_files_removed_upstream: `just`/`rsync` required on CI");
        }
        eprintln!("SKIP update_deletes_files_removed_upstream: `just`/`rsync` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream.git");
    let consumer = tmp.path().join("consumer");

    // Build upstream at v1 (has sentinel.txt).
    make_upstream(&upstream, "1.0.0");
    // Build consumer pointing at upstream.
    make_consumer(&consumer, &upstream, "1.0.0");

    // Upstream deletes sentinel.txt and bumps to v2.
    upstream_add_commit(
        &upstream,
        "v2: delete sentinel.txt",
        &[(".ckeletin/VERSION", "2.0.0")],
        &[".ckeletin/sentinel.txt"],
    );

    // Consumer runs `just ckeletin-update` pointing at the real ckeletin-rust
    // Justfile (we copy it into the consumer for its update recipe).
    install_ckeletin_justfile_into(&consumer);

    let out = Command::new("just")
        .args(["ckeletin-update"])
        .current_dir(&consumer)
        .envs(git_env())
        .output()
        .unwrap_or_else(|e| panic!("just ckeletin-update failed to spawn: {e}"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");

    assert!(
        out.status.success(),
        "ckeletin-update should succeed when compile passes\nstdout: {stdout}\nstderr: {stderr}"
    );

    // The deleted file MUST be gone from the consumer working tree.
    assert!(
        !consumer.join(".ckeletin/sentinel.txt").exists(),
        "sentinel.txt should have been deleted by the wholesale update, but it still exists"
    );

    // VERSION was updated.
    let ver = std::fs::read_to_string(consumer.join(".ckeletin/VERSION")).unwrap();
    assert_eq!(ver.trim(), "2.0.0", "VERSION should be 2.0.0 after update");

    // Machine contract: CKELETIN_UPDATE_RESULT emitted and parseable (finding #4).
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "updated", "status should be 'updated'");
    assert_eq!(result["committed"], true, "committed should be true");
    assert_eq!(result["rolled_back"], false, "rolled_back should be false");
    assert!(
        result["reason"].is_null(),
        "a clean derived update has no stop reason: {result}"
    );
    assert_eq!(
        result["discarded"], false,
        "a clean derived update discards nothing"
    );
}

// ── Finding #2a — check-compatibility leaves no staged files ─────────────────

/// After `ckeletin-update-check-compatibility` across a file-ADDING upstream
/// release, `git status --porcelain .ckeletin/` must be empty (no staged files
/// left behind).
#[test]
fn check_compatibility_leaves_no_staged_files() {
    if !have("just") || !have("rsync") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!(
                "FAIL check_compatibility_leaves_no_staged_files: `just`/`rsync` required on CI"
            );
        }
        eprintln!("SKIP check_compatibility_leaves_no_staged_files: `just`/`rsync` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream.git");
    let consumer = tmp.path().join("consumer");

    make_upstream(&upstream, "1.0.0");
    make_consumer(&consumer, &upstream, "1.0.0");

    // Upstream ADDS a new file (common case for every framework release).
    upstream_add_commit(
        &upstream,
        "v2: add newfile.txt",
        &[
            (".ckeletin/VERSION", "2.0.0"),
            (".ckeletin/newfile.txt", "new content"),
        ],
        &[],
    );

    install_ckeletin_justfile_into(&consumer);

    let out = Command::new("just")
        .arg("ckeletin-update-check-compatibility")
        .current_dir(&consumer)
        .output()
        .unwrap_or_else(|e| panic!("check-compatibility failed to spawn: {e}"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // check-compatibility exits 0 when compatible.
    assert!(
        out.status.success(),
        "check-compatibility should pass\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Working tree must be clean after the restore() trap fires.
    let porcelain = git(&consumer, &["status", "--porcelain", ".ckeletin/"]);
    assert!(
        porcelain.is_empty(),
        "git status --porcelain .ckeletin/ must be empty after check-compatibility,\n\
         but got:\n{porcelain}"
    );

    // The added file must NOT be present in the working tree.
    assert!(
        !consumer.join(".ckeletin/newfile.txt").exists(),
        "newfile.txt must not persist in the consumer after check-compatibility"
    );
}

// ── Finding #2b — compile-fail rollback is genuine ───────────────────────────

/// When `ckeletin-update` triggers tier-1 rollback (upstream introduced a
/// compile error), the working tree is restored to a pristine state AND the
/// CKELETIN_UPDATE_RESULT JSON contains `"rolled_back":true`.
#[test]
fn update_compile_fail_rolls_back_cleanly() {
    if !have("just") || !have("rsync") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!("FAIL update_compile_fail_rolls_back_cleanly: `just`/`rsync` required on CI");
        }
        eprintln!("SKIP update_compile_fail_rolls_back_cleanly: `just`/`rsync` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream.git");
    let consumer = tmp.path().join("consumer");

    make_upstream(&upstream, "1.0.0");
    make_consumer(&consumer, &upstream, "1.0.0");

    // Upstream adds a file AND introduces a compile error in the framework crate.
    upstream_add_commit(
        &upstream,
        "v2: broken compile",
        &[
            (".ckeletin/VERSION", "2.0.0"),
            (".ckeletin/newfile.txt", "this file will be rolled back"),
            // Deliberate syntax error so cargo check --workspace fails.
            (
                ".ckeletin/crate/src/lib.rs",
                "this is not valid rust syntax @@@\n",
            ),
        ],
        &[],
    );

    install_ckeletin_justfile_into(&consumer);

    let out = Command::new("just")
        .arg("ckeletin-update")
        .current_dir(&consumer)
        .envs(git_env())
        .output()
        .unwrap_or_else(|e| panic!("just ckeletin-update failed to spawn: {e}"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");

    // Recipe should exit non-zero (tier-1 rollback path).
    assert!(
        !out.status.success(),
        "ckeletin-update should fail on a compile error\nstdout: {stdout}\nstderr: {stderr}"
    );

    // After rollback the working tree must be clean.
    let porcelain = git(&consumer, &["status", "--porcelain", ".ckeletin/"]);
    assert!(
        porcelain.is_empty(),
        "git status --porcelain .ckeletin/ must be empty after rollback,\n\
         but got:\n{porcelain}"
    );

    // The added newfile.txt must NOT persist.
    assert!(
        !consumer.join(".ckeletin/newfile.txt").exists(),
        "newfile.txt must not persist after a rolled-back update"
    );

    // VERSION must be back to the original.
    let ver = std::fs::read_to_string(consumer.join(".ckeletin/VERSION")).unwrap();
    assert_eq!(
        ver.trim(),
        "1.0.0",
        "VERSION must be restored to 1.0.0 after rollback"
    );

    // Machine contract: CKELETIN_UPDATE_RESULT with rolled_back:true (finding #4).
    let result = parse_update_result(&combined);
    assert_eq!(
        result["status"], "compile_failed",
        "status should be 'compile_failed'"
    );
    assert_eq!(result["rolled_back"], true, "rolled_back must be true");
    assert_eq!(result["committed"], false, "committed must be false");
}

// ── Finding #3 — tag-pinned updates ──────────────────────────────────────────

/// `ckeletin-update version=<tag>` must succeed when the upstream has that tag.
#[test]
fn update_with_explicit_tag_works() {
    if !have("just") || !have("rsync") {
        let ci = std::env::var("CI").unwrap_or_default();
        if ci == "true" {
            panic!("FAIL update_with_explicit_tag_works: `just`/`rsync` required on CI");
        }
        eprintln!("SKIP update_with_explicit_tag_works: `just`/`rsync` not on PATH");
        return;
    }

    // Build upstream at 0.9.0, then publish a tagged v1.0.0 release.
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream.git");
    let consumer = tmp.path().join("consumer");

    make_upstream(&upstream, "0.9.0");
    make_consumer(&consumer, &upstream, "0.9.0");

    // Upstream publishes v1.0.0 and tags it.
    upstream_add_commit(
        &upstream,
        "v1.0.0 release",
        &[(".ckeletin/VERSION", "1.0.0")],
        &[],
    );
    upstream_tag(&upstream, "v1.0.0");

    install_ckeletin_justfile_into(&consumer);

    // Update consumer to the tagged v1.0.0.
    // `just` accepts positional args for recipe parameters: `just recipe arg`.
    let out = Command::new("just")
        .args(["ckeletin-update", "v1.0.0"])
        .current_dir(&consumer)
        .envs(git_env())
        .output()
        .unwrap_or_else(|e| panic!("just ckeletin-update v1.0.0 failed to spawn: {e}"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");

    assert!(
        out.status.success(),
        "ckeletin-update v1.0.0 should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );

    let ver = std::fs::read_to_string(consumer.join(".ckeletin/VERSION")).unwrap();
    assert_eq!(
        ver.trim(),
        "1.0.0",
        "VERSION should be 1.0.0 after tag-pinned update"
    );

    // Machine contract: success verdict (finding #4).
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "updated");
}

// ── helper: install the real ckeletin Justfile into a consumer ────────────────

/// Copy the real update recipe and the local-diff script into `.ckeletin/`.
///
/// Both the upstream baseline and the consumer must get these bytes. The update
/// command does not rewrite names, so a file that exists on only one side is a
/// local difference and stops a clean derived update.
fn copy_framework_update_files(ck: &Path) {
    let root = workspace_root();
    let justfile = root.join(".ckeletin/Justfile");
    std::fs::copy(&justfile, ck.join("Justfile"))
        .unwrap_or_else(|e| panic!("copy {}: {e}", justfile.display()));
    let scripts = ck.join("scripts");
    std::fs::create_dir_all(&scripts).unwrap();
    for name in ["framework-local-diff.sh", "framework-lock.sh"] {
        let script = root.join(".ckeletin/scripts").join(name);
        std::fs::copy(&script, scripts.join(name))
            .unwrap_or_else(|e| panic!("copy {}: {e}", script.display()));
    }
}

fn commit_all(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    let status = Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(dir)
        .envs(git_env())
        .status()
        .unwrap();
    assert!(status.success(), "git commit failed: {msg}");
}

fn require_just(test_name: &str) -> bool {
    if have("just") {
        return true;
    }
    let ci = std::env::var("CI").unwrap_or_default();
    if ci == "true" {
        panic!("FAIL {test_name}: `just` required on CI");
    }
    eprintln!("SKIP {test_name}: `just` not on PATH");
    false
}

fn run_just(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("just")
        .args(args)
        .current_dir(dir)
        .envs(git_env())
        .output()
        .unwrap_or_else(|e| panic!("just {args:?} failed to spawn: {e}"))
}

fn install_ckeletin_justfile_into(consumer: &Path) {
    copy_framework_update_files(&consumer.join(".ckeletin"));

    // Minimal root Justfile that imports the framework.
    // The root `check` recipe is needed because ckeletin-update calls `just check`.
    let root_justfile = consumer.join("Justfile");
    std::fs::write(
        &root_justfile,
        "import '.ckeletin/Justfile'\n\ncheck:\n    cargo check --workspace -q\n",
    )
    .unwrap();

    // Commit the updated Justfiles so the working tree is clean before update.
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(consumer)
        .status()
        .unwrap();
    let status = Command::new("git")
        .args(["commit", "-m", "add justfiles"])
        .current_dir(consumer)
        .envs(git_env())
        .status()
        .unwrap();
    // Ignore "nothing to commit" (exit 1) — already committed.
    let _ = status;
}

struct UpdateFixture {
    _tmp: tempfile::TempDir,
    consumer: PathBuf,
}

/// Consumer and upstream both start at 1.0.0 with the same `.ckeletin/` bytes.
/// `bump_to` adds a later upstream commit (new version and `newfile.txt`).
fn update_fixture(bump_to: Option<&str>) -> UpdateFixture {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream.git");
    let consumer = tmp.path().join("consumer");
    make_upstream(&upstream, "1.0.0");
    make_consumer(&consumer, &upstream, "1.0.0");
    if let Some(version) = bump_to {
        upstream_add_commit(
            &upstream,
            "bump",
            &[
                (".ckeletin/VERSION", version),
                (".ckeletin/newfile.txt", "from upstream\n"),
            ],
            &[],
        );
    }
    install_ckeletin_justfile_into(&consumer);
    UpdateFixture {
        _tmp: tmp,
        consumer,
    }
}

fn assert_stopped(out: &std::process::Output, reason: &str) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        !out.status.success(),
        "expected update to stop\nstdout: {stdout}\nstderr: {stderr}"
    );
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "stopped", "{result}");
    assert_eq!(result["reason"], reason, "{result}");
    assert!(
        result["to"].is_null(),
        "stopped update must not name a new version: {result}"
    );
    assert_eq!(result["committed"], false, "{result}");
    assert_eq!(result["discarded"], false, "{result}");
    result
}

/// One extra committed file under `.ckeletin/` is a local difference. The
/// update stops, prints it, and does not replace the tree.
#[test]
fn committed_local_file_stops_update() {
    if !require_just("committed_local_file_stops_update") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let local = fix.consumer.join(".ckeletin/local-only.txt");
    std::fs::write(&local, "local patch\n").unwrap();
    commit_all(&fix.consumer, "local framework patch");

    let out = run_just(&fix.consumer, &["ckeletin-update"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_stopped(&out, "local_difference");
    assert!(
        stdout.contains("local-only.txt"),
        "stop should show the local difference\n{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
    assert!(local.exists(), "local file must survive a stopped update");
    assert!(
        !fix.consumer.join(".ckeletin/newfile.txt").exists(),
        "upstream file must not be applied when the update stops"
    );
}

/// `discard_local=true` continues only for a committed difference, says so,
/// and records `"discarded":true` on success.
#[test]
fn discard_local_true_replaces_committed_difference() {
    if !require_just("discard_local_true_replaces_committed_difference") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let local = fix.consumer.join(".ckeletin/local-only.txt");
    std::fs::write(&local, "local patch\n").unwrap();
    commit_all(&fix.consumer, "local framework patch");

    let out = run_just(&fix.consumer, &["ckeletin-update", "discard_local=true"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        out.status.success(),
        "discard_local=true should update\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        combined.contains("Discarding local framework changes."),
        "must say it is discarding local framework changes\n{combined}"
    );
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "updated", "{result}");
    assert_eq!(result["discarded"], true, "{result}");
    assert_eq!(result["committed"], true, "{result}");
    assert!(result["reason"].is_null(), "{result}");
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "2.0.0"
    );
    assert!(!local.exists(), "discarded local file must be gone");
    assert!(fix.consumer.join(".ckeletin/newfile.txt").exists());
}

/// An explicit discard must not bypass uncommitted changes.
#[test]
fn discard_local_does_not_bypass_uncommitted_changes() {
    if !require_just("discard_local_does_not_bypass_uncommitted_changes") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let dirty = fix.consumer.join(".ckeletin/dirty.txt");
    std::fs::write(&dirty, "not committed\n").unwrap();

    let out = run_just(&fix.consumer, &["ckeletin-update", "discard_local=true"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let result = assert_stopped(&out, "uncommitted");
    assert_eq!(result["from"], "1.0.0", "{result}");
    assert!(
        !stdout.contains("Discarding local framework changes."),
        "uncommitted changes must not be discarded\n{stdout}"
    );
    assert!(dirty.exists());
    assert_eq!(std::fs::read_to_string(&dirty).unwrap(), "not committed\n");
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
    assert!(!fix.consumer.join(".ckeletin/newfile.txt").exists());
}

/// Dry-run prints a committed local difference, changes nothing, and exits 0.
#[test]
fn dry_run_prints_local_difference_and_changes_nothing() {
    if !require_just("dry_run_prints_local_difference_and_changes_nothing") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let local = fix.consumer.join(".ckeletin/local-only.txt");
    std::fs::write(&local, "local patch\n").unwrap();
    commit_all(&fix.consumer, "local framework patch");
    let before = git(&fix.consumer, &["status", "--porcelain"]);

    let out = run_just(&fix.consumer, &["ckeletin-update-dry-run"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "dry-run should exit 0\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("local-only.txt"),
        "dry-run should print the local difference\n{stdout}"
    );
    assert!(local.exists());
    assert_eq!(std::fs::read_to_string(&local).unwrap(), "local patch\n");
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
    assert!(
        !fix.consumer.join(".ckeletin/newfile.txt").exists(),
        "dry-run must not apply upstream files"
    );
    assert_eq!(
        git(&fix.consumer, &["status", "--porcelain"]),
        before,
        "dry-run must not change the worktree"
    );
}

/// Compatibility check exits 1 on a local difference and leaves the tree as found.
#[test]
fn compatibility_check_stops_on_local_difference_without_modifying() {
    if !require_just("compatibility_check_stops_on_local_difference_without_modifying") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let local = fix.consumer.join(".ckeletin/local-only.txt");
    std::fs::write(&local, "local patch\n").unwrap();
    commit_all(&fix.consumer, "local framework patch");

    let out = run_just(&fix.consumer, &["ckeletin-update-check-compatibility"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_stopped(&out, "local_difference");
    assert!(stdout.contains("local-only.txt"), "{stdout}");
    assert!(local.exists());
    assert!(!fix.consumer.join(".ckeletin/newfile.txt").exists());
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
    assert!(
        git(&fix.consumer, &["status", "--porcelain", ".ckeletin/"]).is_empty(),
        "compatibility check must not modify .ckeletin/"
    );
}

/// A VERSION upstream has never recorded cannot be compared, so the update stops.
#[test]
fn unknown_framework_version_stops_update() {
    if !require_just("unknown_framework_version_stops_update") {
        return;
    }
    let fix = update_fixture(None);
    std::fs::write(fix.consumer.join(".ckeletin/VERSION"), "9.9.9\n").unwrap();
    commit_all(&fix.consumer, "bump local framework version");

    let out = run_just(&fix.consumer, &["ckeletin-update"]);
    let result = assert_stopped(&out, "unknown_version");
    assert_eq!(result["from"], "9.9.9", "{result}");
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "9.9.9"
    );
}

/// `commit=false` applies the update, prints the result, and does not commit.
#[test]
fn update_commit_false_applies_without_committing() {
    if !require_just("update_commit_false_applies_without_committing") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let before = git(&fix.consumer, &["rev-parse", "HEAD"]);

    let out = run_just(&fix.consumer, &["ckeletin-update", "commit=false"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        out.status.success(),
        "commit=false should still apply a green update\nstdout: {stdout}\nstderr: {stderr}"
    );
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "updated", "{result}");
    assert_eq!(result["committed"], false, "{result}");
    assert_eq!(result["from"], "1.0.0", "{result}");
    assert_eq!(result["to"], "2.0.0", "{result}");
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "2.0.0"
    );
    assert_eq!(
        git(&fix.consumer, &["rev-parse", "HEAD"]),
        before,
        "commit=false must not create a commit"
    );
    assert!(
        !git(&fix.consumer, &["status", "--porcelain", ".ckeletin/"]).is_empty(),
        "the applied framework must remain uncommitted"
    );
}

/// An existing lockfile is refreshed with `cargo update -p ckeletin`, which
/// prints Cargo's "Locking" line. `cargo generate-lockfile` does not.
#[test]
fn update_refreshes_only_the_framework_package_in_the_lockfile() {
    if !require_just("update_refreshes_only_the_framework_package_in_the_lockfile") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let lock_status = Command::new("cargo")
        .args(["generate-lockfile"])
        .current_dir(&fix.consumer)
        .status()
        .unwrap();
    assert!(lock_status.success(), "cargo generate-lockfile failed");
    commit_all(&fix.consumer, "add lockfile");

    let out = run_just(&fix.consumer, &["ckeletin-update", "commit=false"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        out.status.success(),
        "update with a lockfile should succeed\n{combined}"
    );
    assert!(
        combined.contains("Locking"),
        "an existing Cargo.lock must be refreshed with cargo update -p ckeletin, \
         whose output contains \"Locking\"\n{combined}"
    );
}

/// A dirty framework that is already the update under test is refused and left
/// in place. `from` is the committed version, not the version on disk.
#[test]
fn compatibility_check_does_not_restore_an_uncommitted_update() {
    if !require_just("compatibility_check_does_not_restore_an_uncommitted_update") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    std::fs::write(fix.consumer.join(".ckeletin/VERSION"), "2.0.0\n").unwrap();

    let out = run_just(&fix.consumer, &["ckeletin-update-check-compatibility"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let result = assert_stopped(&out, "uncommitted");
    assert_eq!(
        result["from"], "1.0.0",
        "from must be the committed version\n{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "2.0.0",
        "refusing must not put the committed framework back\n{stdout}"
    );
    assert!(
        !fix.consumer.join(".ckeletin/newfile.txt").exists(),
        "refusing must not apply the upstream tree either"
    );
}

/// The compatibility trap restores `Cargo.lock` bytes from HEAD. Regenerating
/// the lock would drop a committed comment.
#[test]
fn compatibility_restores_the_committed_lockfile() {
    if !require_just("compatibility_restores_the_committed_lockfile") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let lock_status = Command::new("cargo")
        .args(["generate-lockfile"])
        .current_dir(&fix.consumer)
        .status()
        .unwrap();
    assert!(lock_status.success(), "cargo generate-lockfile failed");
    let lock_path = fix.consumer.join("Cargo.lock");
    let mut lock = std::fs::read_to_string(&lock_path).unwrap();
    lock.push_str("# lock-bytes-must-survive\n");
    std::fs::write(&lock_path, &lock).unwrap();
    commit_all(&fix.consumer, "lock with a committed comment");

    let out = run_just(&fix.consumer, &["ckeletin-update-check-compatibility"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "compatibility should pass\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(&lock_path).unwrap(),
        lock,
        "Cargo.lock must be restored from HEAD, not regenerated"
    );
    assert_eq!(
        std::fs::read_to_string(fix.consumer.join(".ckeletin/VERSION"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
}

/// A leftover `.ckeletin/cache/` stamp must not refuse the update or get committed.
#[test]
fn update_ignores_a_leftover_framework_cache() {
    if !require_just("update_ignores_a_leftover_framework_cache") {
        return;
    }
    let fix = update_fixture(Some("2.0.0"));
    let cache = fix.consumer.join(".ckeletin/cache");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join(".last-update-check"), "1\n").unwrap();

    let out = run_just(&fix.consumer, &["ckeletin-update"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        out.status.success(),
        "a cache stamp must not block the update\n{combined}"
    );
    let result = parse_update_result(&combined);
    assert_eq!(result["status"], "updated", "{result}");
    let committed = git(
        &fix.consumer,
        &["show", "--name-only", "--pretty=format:", "HEAD"],
    );
    assert!(
        !committed.contains(".ckeletin/cache"),
        "the update commit must not record the cache\n{committed}"
    );
}

/// The health warning names the command that discards a committed local difference.
#[test]
fn health_warning_names_the_discard_command() {
    if !require_just("health_warning_names_the_discard_command") {
        return;
    }
    let fix = update_fixture(None);
    std::fs::write(fix.consumer.join(".ckeletin/local.txt"), "x\n").unwrap();

    let out = run_just(&fix.consumer, &["ckeletin-health"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains("just ckeletin-update discard_local=true"),
        "the warning must name the command\nstdout: {stdout}\nstderr: {stderr}"
    );
}
