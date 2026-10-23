//! Every CI job gets a short temp root and deletes it on the way out.
//!
//! The prepare step cannot call a repo script: it runs before checkout.
//! This test executes that step against a fixture.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE: &str = "/tmp/ck-ci.XXXXXXXX";

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .and_then(|parent| parent.parent())
        .unwrap()
        .to_path_buf()
}

fn workflows() -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(workspace_root().join(".github/workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yml"))
        .collect();
    paths.sort();
    paths
}

struct Job {
    name: String,
    body: String,
}

fn jobs(text: &str) -> Vec<Job> {
    let jobs_at = text.find("\njobs:\n").expect("workflow has no jobs");
    let rest = &text[jobs_at + "\njobs:\n".len()..];
    let lines: Vec<&str> = rest.lines().collect();
    let mut starts = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let name = line.strip_prefix("  ").unwrap_or("");
        let is_job = !name.is_empty()
            && !line.starts_with("   ")
            && name.ends_with(':')
            && name[..name.len() - 1]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if is_job {
            starts.push(index);
        }
    }
    let mut found = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(lines.len());
        let name = lines[*start].trim().trim_end_matches(':').to_string();
        let body = lines[*start..end].join("\n");
        found.push(Job { name, body });
    }
    found
}

fn top_steps(body: &str) -> Vec<String> {
    let mut steps = Vec::new();
    let mut current = String::new();
    let mut in_steps = false;
    for line in body.lines() {
        if line == "    steps:" {
            in_steps = true;
            continue;
        }
        if !in_steps {
            continue;
        }
        if line.starts_with("      - ") {
            if !current.is_empty() {
                steps.push(std::mem::take(&mut current));
            }
            current.push_str(line);
            current.push('\n');
            continue;
        }
        let comment = line.trim_start().starts_with('#');
        let nested = line.starts_with("        ") || line.trim().is_empty() || comment;
        if current.is_empty() && (line.trim().is_empty() || comment) {
            continue;
        }
        if !current.is_empty() && nested {
            current.push_str(line);
            current.push('\n');
        } else {
            break;
        }
    }
    if !current.is_empty() {
        steps.push(current);
    }
    steps
}

fn run_body(step: &str) -> String {
    if let Some(inline) = step
        .lines()
        .find_map(|line| line.trim().strip_prefix("run: "))
        && inline != "|"
    {
        return format!("{inline}\n");
    }
    let mut body = String::new();
    let mut in_run = false;
    for line in step.lines() {
        if line.trim_start().starts_with("run:") {
            in_run = true;
            continue;
        }
        if in_run && let Some(code) = line.strip_prefix("          ") {
            body.push_str(code);
            body.push('\n');
        }
    }
    body
}

fn assert_no_stray_tmp(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    for (number, line) in text.lines().enumerate() {
        let code = line.split('#').next().unwrap();
        if !code.contains("/tmp") {
            continue;
        }
        assert!(
            code.contains("CKELETIN_CI_TEMP: /tmp/ck-ci.XXXXXXXX"),
            "{}:{} has a /tmp path outside the job-root template:\n{line}",
            path.display(),
            number + 1
        );
    }
}

#[test]
fn ci_template_leaves_socket_room() {
    assert!(TEMPLATE.starts_with("/tmp/ck-"));
    assert!(TEMPLATE.ends_with(".XXXXXXXX"));
    // mktemp keeps this length. The launcher adds `/t-` plus 8 characters,
    // and a fixture may append 60 bytes. Unix socket paths stop at 104.
    assert!(TEMPLATE.len() + "/t-".len() + 8 + 60 <= 104);
}

#[test]
fn just_pin_is_one_file() {
    let pin = fs::read_to_string(workspace_root().join(".github/JUST_VERSION")).unwrap();
    assert_eq!(pin, "1.52.0\n");
    for path in workflows() {
        let text = fs::read_to_string(&path).unwrap();
        if text.contains("cargo install just") {
            assert!(
                text.contains(".github/JUST_VERSION"),
                "{} must read the just pin",
                path.display()
            );
            assert!(
                !text.contains("cargo install just --version 1."),
                "{} still hardcodes a just version",
                path.display()
            );
        }
    }
}

#[test]
fn every_job_prepares_and_removes_its_temp_root() {
    let mut count = 0;
    let mut prepare = None;
    let mut cleanup = None;
    let mut saw_secret_scan = false;
    let mut saw_per_commit = false;
    for path in workflows() {
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("CKELETIN_CI_TEMP: /tmp/ck-ci.XXXXXXXX"),
            "{} is missing the job-root template",
            path.display()
        );
        let found = jobs(&text);
        assert!(!found.is_empty(), "{} has no jobs", path.display());
        for job in found {
            let steps = top_steps(&job.body);
            assert!(
                steps.len() >= 2,
                "{} job {} has too few steps",
                path.display(),
                job.name
            );
            assert!(
                steps[0].contains("name: Prepare job temp folder"),
                "{} job {} must prepare before checkout",
                path.display(),
                job.name
            );
            let last = steps.last().unwrap();
            assert!(
                last.contains("name: Remove job temp folder"),
                "{} job {} must clean up last",
                path.display(),
                job.name
            );
            assert!(
                last.contains("if: always()"),
                "{} job {} cleanup must run when the job fails",
                path.display(),
                job.name
            );
            let this_prepare = run_body(&steps[0]);
            let this_cleanup = run_body(last);
            match &prepare {
                None => prepare = Some(this_prepare),
                Some(expected) => assert_eq!(&this_prepare, expected, "prepare scripts differ"),
            }
            match &cleanup {
                None => cleanup = Some(this_cleanup),
                Some(expected) => assert_eq!(&this_cleanup, expected, "cleanup scripts differ"),
            }
            if job.name == "secret-scan" {
                saw_secret_scan = job.body.contains("fetch-depth: 0");
            }
            if job.name == "per-commit" {
                saw_per_commit = job.body.contains("fetch-depth: 0")
                    && job.body.contains("just ckeletin-fmt-check")
                    && job.body.contains("just ckeletin-clippy")
                    && job.body.contains("cargo check --workspace --all-targets")
                    && job.body.contains("just ckeletin-deny-history")
                    && job.body.contains("not on each historical commit")
                    && !job.body.contains("just ckeletin-deny\n")
                    && job.body.contains("github.event_name == 'pull_request'");
            }
            count += 1;
        }
    }
    assert!(count >= 12, "checked {count} jobs, expected at least 12");
    assert!(
        saw_secret_scan,
        "secret-scan must keep its full-history checkout"
    );
    assert!(saw_per_commit, "per-commit slice is missing its checks");

    let prepare = prepare.unwrap();
    assert!(prepare.contains("-maxdepth 1"));
    assert!(prepare.contains("-type d"));
    assert!(prepare.contains("-name 'ck-*'"));
    assert!(prepare.contains("-user"));
    assert!(prepare.contains("-mmin +360"));
    assert!(prepare.contains("-exec rm -rf --"));
    assert!(prepare.contains("mktemp -d"));
    assert!(!prepare.contains("-follow"));
    assert!(!prepare.contains(" -L"));
    assert_eq!(cleanup.unwrap(), "rm -rf -- \"$CKELETIN_CI_TEMP\"\n");
    execute_prepare_and_cleanup(&prepare);
}

fn execute_prepare_and_cleanup(prepare: &str) {
    let fixture = tempfile::tempdir().unwrap();
    let old = fixture.path().join("ck-abandoned");
    let recent = fixture.path().join("ck-recent");
    let target = fixture.path().join("keep-target");
    let link = fixture.path().join("ck-link");
    let file = fixture.path().join("ck-not-a-dir");
    fs::create_dir(&old).unwrap();
    fs::create_dir(&recent).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(&file, "keep").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let touched = Command::new("touch")
        .args(["-t", "202001010000"])
        .arg(&old)
        .status()
        .unwrap();
    assert!(touched.success(), "touch failed");

    let created = run_prepare(fixture.path(), prepare, "");
    assert!(!old.exists(), "abandoned folder must be removed");
    assert!(recent.exists(), "recent folder must be kept");
    assert!(link.exists(), "symlink must be kept");
    assert!(target.exists(), "symlink target must be kept");
    assert!(
        file.exists(),
        "a file must not be removed by the directory sweep"
    );
    assert!(created.is_dir(), "mktemp did not create the job root");

    let other_old = fixture.path().join("ck-other-user");
    fs::create_dir(&other_old).unwrap();
    let other_touch = Command::new("touch")
        .args(["-t", "202001010000"])
        .arg(&other_old)
        .status()
        .unwrap();
    assert!(other_touch.success(), "touch failed");
    let _kept = run_prepare(fixture.path(), prepare, "id() { echo 65534; }\n");
    assert!(
        other_old.exists(),
        "sweep must not remove another user's abandoned folder"
    );

    let leaked = Command::new("python3")
        .args([
            "-c",
            "import tempfile; from pathlib import Path; p = Path(tempfile.mkdtemp(prefix='leak-')) / 'x'; p.write_text('z'); print(p)",
        ])
        .env("TMPDIR", &created)
        .env("TMP", &created)
        .env("TEMP", &created)
        .output()
        .unwrap();
    assert!(
        leaked.status.success(),
        "python leak failed: {}",
        String::from_utf8_lossy(&leaked.stderr)
    );
    let leaked_path = PathBuf::from(String::from_utf8_lossy(&leaked.stdout).trim());
    assert!(
        leaked_path.starts_with(&created),
        "tempfile escaped the job root: {}",
        leaked_path.display()
    );

    let cleanup = "rm -rf -- \"$CKELETIN_CI_TEMP\"\n";
    run_cleanup(&created, cleanup);
    assert!(!created.exists(), "cleanup must remove the job root");
    assert!(
        !leaked_path.exists(),
        "cleanup must remove a leaked tempfile"
    );
    run_cleanup(&created, cleanup);
}

fn run_prepare(fixture: &Path, prepare: &str, prelude: &str) -> PathBuf {
    let script = fixture.join(if prelude.is_empty() {
        "prepare.sh"
    } else {
        "prepare-other.sh"
    });
    let github_env = fixture.join(if prelude.is_empty() {
        "github-env"
    } else {
        "github-env-other"
    });
    fs::write(&script, format!("{prelude}{prepare}")).unwrap();
    let template = fixture.join("ck-ci.XXXXXXXX");
    let output = Command::new("bash")
        .arg(&script)
        .env("CKELETIN_CI_TEMP", &template)
        .env("GITHUB_ENV", &github_env)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "prepare failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let env_text = fs::read_to_string(&github_env).unwrap();
    let created = env_text
        .lines()
        .filter_map(|line| line.strip_prefix("CKELETIN_CI_TEMP="))
        .next_back()
        .expect("prepare did not publish CKELETIN_CI_TEMP")
        .to_string();
    for name in ["CKELETIN_CI_TEMP", "TMPDIR", "TMP", "TEMP"] {
        assert!(
            env_text.contains(&format!("{name}={created}")),
            "missing {name}={created} in:\n{env_text}"
        );
    }
    PathBuf::from(created)
}

fn run_cleanup(root: &Path, cleanup: &str) {
    let script = root
        .parent()
        .unwrap()
        .join(format!("cleanup-{}.sh", std::process::id()));
    fs::write(&script, cleanup).unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .env("CKELETIN_CI_TEMP", root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "cleanup failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn workflows_and_justfiles_have_no_stray_tmp_paths() {
    for path in workflows() {
        assert_no_stray_tmp(&path);
    }
    assert_no_stray_tmp(&workspace_root().join("Justfile"));
    assert_no_stray_tmp(&workspace_root().join(".ckeletin/Justfile"));
}
