//! Drift guard: vendored violation test files in `.ckeletin/tests/violations/`
//! must be byte-identical to their active project copies under
//! `crates/domain/tests/violations/` and `crates/infrastructure/tests/violations/`.
//!
//! **Why:** the vendored copies are the canonical versions that propagate to
//! consumer repos via `just ckeletin-update`. The project copies are what the
//! trybuild tests actually run. If they diverge, a consumer gets stale violation
//! tests and the enforcement claim is backed by different code than what runs.
//!
//! **Design choice (Finding #8):** we keep both copies (vendored = canonical,
//! project = active) and enforce byte-identity here rather than removing one set.
//! Removing the vendored copies would break the propagation contract (consumers
//! receive `.ckeletin/**` wholesale). Removing the project copies would break
//! trybuild (which reads relative to the crate root). The drift guard is the
//! minimum-viable tie between them: divergence is loud (this test fails in CI),
//! reconciliation is a one-line copy.

use std::collections::HashMap;

/// Map of vendored file name → active project file path(s).
/// Each vendored file must match at least one of the listed active paths.
fn violation_file_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            ".ckeletin/tests/violations/domain_imports_clap.rs",
            "crates/domain/tests/violations/domain_imports_clap.rs",
        ),
        (
            ".ckeletin/tests/violations/domain_imports_figment.rs",
            "crates/domain/tests/violations/domain_imports_figment.rs",
        ),
        (
            ".ckeletin/tests/violations/domain_imports_infrastructure.rs",
            "crates/domain/tests/violations/domain_imports_infrastructure.rs",
        ),
        (
            ".ckeletin/tests/violations/domain_imports_tracing.rs",
            "crates/domain/tests/violations/domain_imports_tracing.rs",
        ),
        (
            ".ckeletin/tests/violations/domain_imports_ckeletin.rs",
            "crates/domain/tests/violations/domain_imports_ckeletin.rs",
        ),
        (
            ".ckeletin/tests/violations/infra_imports_clap.rs",
            "crates/infrastructure/tests/violations/infra_imports_clap.rs",
        ),
        (
            ".ckeletin/tests/violations/infra_imports_domain.rs",
            "crates/infrastructure/tests/violations/infra_imports_domain.rs",
        ),
    ]
}

/// Resolve a path relative to the workspace root.
/// Tests run from either the crate root (.ckeletin/crate/) or the workspace root.
fn workspace_path(relative: &str) -> std::path::PathBuf {
    // When tests run from `.ckeletin/crate/`, go up two levels to workspace root.
    // When tests run from workspace root (e.g. `cargo test --workspace`), stay put.
    let from_crate_root = std::path::Path::new("../../").join(relative);
    let from_workspace_root = std::path::Path::new(relative);
    if from_crate_root.exists() {
        from_crate_root
    } else {
        from_workspace_root.to_path_buf()
    }
}

#[test]
fn vendored_violation_tests_are_byte_identical_to_project_copies() {
    let pairs = violation_file_pairs();
    let mut failures: HashMap<String, String> = HashMap::new();

    for (vendored_rel, active_rel) in pairs {
        let vendored_path = workspace_path(vendored_rel);
        let active_path = workspace_path(active_rel);

        let vendored = match std::fs::read(&vendored_path) {
            Ok(b) => b,
            Err(e) => {
                failures.insert(
                    vendored_rel.to_string(),
                    format!(
                        "cannot read vendored file {}: {}",
                        vendored_path.display(),
                        e
                    ),
                );
                continue;
            }
        };

        let active = match std::fs::read(&active_path) {
            Ok(b) => b,
            Err(e) => {
                failures.insert(
                    active_rel.to_string(),
                    format!("cannot read active file {}: {}", active_path.display(), e),
                );
                continue;
            }
        };

        if vendored != active {
            failures.insert(
                vendored_rel.to_string(),
                format!(
                    "vendored {} differs from active {}; copy the canonical version to reconcile:\n  cp {} {}",
                    vendored_path.display(),
                    active_path.display(),
                    vendored_path.display(),
                    active_path.display(),
                ),
            );
        }
    }

    assert!(
        failures.is_empty(),
        "Vendored violation tests diverged from project copies ({} pair(s)):\n{}",
        failures.len(),
        failures.values().cloned().collect::<Vec<_>>().join("\n")
    );
}
