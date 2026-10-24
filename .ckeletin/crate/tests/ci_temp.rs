//! Every CI job gets a short temp root and deletes it on the way out.
//!
//! The prepare step cannot call a repo script: it runs before checkout.
//! This test executes that step against a fixture.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn ci_prefix() -> String {
    let path = workspace_root().join(".github/CI_TEMP_PREFIX");
    let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "read {}: {err}. Set the CI temp prefix in that file. The next framework update requires it, because a shared ck- sweep deletes another project's folders.",
            path.display()
        )
    });
    let prefix = raw.trim();
    assert!(
        !prefix.is_empty()
            && !prefix.starts_with('-')
            && !prefix.ends_with('-')
            && prefix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
        ".github/CI_TEMP_PREFIX must be a short lowercase token, got {prefix:?}"
    );
    prefix.to_string()
}

fn ci_template() -> String {
    format!("/tmp/{}-ci.XXXXXXXX", ci_prefix())
}

fn assert_no_stray_tmp(path: &Path) {
    let marker = format!("CKELETIN_CI_TEMP: {}", ci_template());
    let text = fs::read_to_string(path).unwrap();
    for (number, line) in text.lines().enumerate() {
        let code = line.split('#').next().unwrap();
        if !code.contains("/tmp") {
            continue;
        }
        assert!(
            code.contains(&marker),
            "{}:{} has a /tmp path outside the job-root template:\n{line}",
            path.display(),
            number + 1
        );
    }
}

#[test]
fn ci_template_leaves_socket_room() {
    let template = ci_template();
    let prefix = ci_prefix();
    assert!(template.starts_with("/tmp/"));
    assert!(template.ends_with(".XXXXXXXX"));
    assert!(template.contains(&format!("/{prefix}-ci.")));
    // mktemp keeps this length. The launcher adds `/t-` plus 8 characters,
    // and a fixture may append 60 bytes. Unix socket paths stop at 104.
    assert!(
        template.len() + "/t-".len() + 8 + 60 <= 104,
        "prefix {prefix} leaves no room for a socket path"
    );
}

fn just_action_pins(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find("just@") {
        rest = &rest[index + "just@".len()..];
        let version: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if !version.is_empty() {
            found.push(version);
        }
    }
    found
}

#[test]
fn just_pin_is_one_file() {
    let pin = fs::read_to_string(workspace_root().join(".github/JUST_VERSION")).unwrap();
    assert_eq!(pin, "1.52.0\n");
    let version = pin.trim();
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
        for pinned in just_action_pins(&text) {
            assert_eq!(
                pinned,
                version,
                "{} pins just@{pinned}; JUST_VERSION is {version}",
                path.display()
            );
        }
    }
}

fn cleanup_runs_when_the_job_fails(step: &str) -> bool {
    step.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("if:") && trimmed.contains("always()")
    })
}

#[test]
fn every_job_prepares_and_removes_its_temp_root() {
    let mut count = 0;
    let mut prepare = None;
    let mut cleanup = None;
    let mut saw_secret_scan = false;
    let mut saw_secret_gate = false;
    let mut slice_names = Vec::new();
    for path in workflows() {
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains(&format!("CKELETIN_CI_TEMP: {}", ci_template())),
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
                cleanup_runs_when_the_job_fails(last),
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
            if job.body.contains("needs.secret-scan.result")
                && job.body.contains("!= \"success\"")
                && job.body.contains("needs:")
            {
                saw_secret_gate = true;
            }
            let is_slice = job.body.contains("fetch-depth: 0")
                && job.body.contains("just ckeletin-fmt-check")
                && job.body.contains("just ckeletin-clippy")
                && job.body.contains("cargo check --workspace --all-targets")
                && job.body.contains("just ckeletin-deny-history")
                && job.body.contains("not on each historical commit")
                && !job.body.contains("just ckeletin-deny\n")
                && job.body.contains("github.event_name == 'pull_request'")
                && job.body.contains("required-gates")
                && job.body.contains("skipped on push");
            if is_slice {
                slice_names.push(job.name.clone());
            }
            count += 1;
        }
    }
    assert!(count >= 12, "checked {count} jobs, expected at least 12");
    assert!(
        saw_secret_scan,
        "secret-scan must keep its full-history checkout"
    );
    assert!(
        saw_secret_gate,
        "a gate must wait for secret-scan and accept only success"
    );
    assert!(
        !slice_names.is_empty(),
        "a per-commit slice is missing its checks"
    );
    for path in workflows() {
        let text = fs::read_to_string(&path).unwrap();
        for job in jobs(&text) {
            for line in job.body.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with("needs:") {
                    continue;
                }
                for name in &slice_names {
                    assert!(
                        !trimmed.contains(name.as_str()),
                        "{} job {} needs the per-commit slice {name}. That job is skipped on push to main, so a required-gates needs entry fails success.",
                        path.display(),
                        job.name
                    );
                }
            }
        }
    }

    let prepare = prepare.unwrap();
    let prefix = ci_prefix();
    assert!(prepare.contains("-maxdepth 1"));
    assert!(prepare.contains("-type d"));
    assert!(prepare.contains(&format!("-name '{prefix}-*'")));
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
    let prefix = ci_prefix();
    let old = fixture.path().join(format!("{prefix}-abandoned"));
    let recent = fixture.path().join(format!("{prefix}-recent"));
    let target = fixture.path().join("keep-target");
    let link = fixture.path().join(format!("{prefix}-link"));
    let file = fixture.path().join(format!("{prefix}-not-a-dir"));
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

    let other_old = fixture.path().join(format!("{prefix}-other-user"));
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
    let template = fixture.join(format!("{}-ci.XXXXXXXX", ci_prefix()));
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
