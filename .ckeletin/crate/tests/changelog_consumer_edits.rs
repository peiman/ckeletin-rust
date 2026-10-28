//! The update failure names changelog entries a consumer has to edit.

use std::process::Command;

fn script() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("scripts/changelog-consumer-edits.sh")
}

#[test]
fn changelog_entries_that_name_a_consumer_edit_are_printed() {
    let dir = tempfile::tempdir().unwrap();
    let changelog = dir.path().join("CHANGELOG.md");
    std::fs::write(
        &changelog,
        "\
## [Unreleased]

## [0.2.35] - 2026-10-28

### Fixed

- **Per-commit CI falls back when `just ckeletin-deny-history` is missing.**
  The loop lives in `.github/workflows`. An update does not replace that file.
- Clippy is stricter. Fix the lint in the crate.

## [0.2.34] - 2026-10-24

- Older note about CI_TEMP_PREFIX.
",
    )
    .unwrap();

    let output = Command::new("bash")
        .arg(script())
        .arg(&changelog)
        .arg("0.2.35")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("ckeletin-deny-history"), "{stdout}");
    assert!(stdout.contains(".github/"), "{stdout}");
    assert!(
        !stdout.contains("CI_TEMP_PREFIX"),
        "an older version's entry must not be included:\n{stdout}"
    );
    assert!(
        !stdout.contains("Clippy is stricter"),
        "a bullet that names no consumer file must not be included:\n{stdout}"
    );
}
