//! Worked example: bake the binary's build identity into compile-time env vars
//! that `crates/cli/src/version.rs` reads. This is the wiring half of ckeletin's
//! build-identity primitive — adopters keep, customize, or delete it like the
//! `ping` command. Best-effort: any git failure degrades to an honest "unknown",
//! never a build failure and never a fabricated value.

use std::process::Command;

fn main() {
    // Rebuild when HEAD or the index moves, so the baked identity stays honest.
    // `--git-path` resolves the real location even through worktree/submodule
    // indirection (where `.git` is a file, not a directory).
    for p in ["HEAD", "index"] {
        if let Some(path) = git(&["rev-parse", "--git-path", p]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");

    // ONE command resolves commit + dirty atomically, so there is no
    // independent-failure gap where a dirty check fails while the commit read
    // succeeds and bakes a false-clean identity (the two-command trap workhorse
    // hit in SH-004). If `describe` fails outright, both degrade together.
    let (commit, dirty) = match git(&["describe", "--always", "--dirty", "--abbrev=7"]) {
        Some(d) if d.ends_with("-dirty") => (d.trim_end_matches("-dirty").to_string(), "true"),
        Some(d) => (d, "false"),
        None => ("unknown".to_string(), "false"),
    };
    // Date is informational; its independent failure degrades to an honest
    // "unknown" date, not a false cleanliness claim — so a separate call is safe.
    let date =
        git(&["show", "-s", "--format=%cs", "HEAD"]).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=CKELETIN_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=CKELETIN_BUILD_DIRTY={dirty}");
    println!("cargo:rustc-env=CKELETIN_BUILD_DATE={date}");
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
