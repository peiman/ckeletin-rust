//! Dependency allowlist invariant tests (Finding #7 / CKSPEC-ARCH-003/004/005).
//!
//! These tests parse the Cargo.toml files for the domain, infrastructure, and
//! framework (.ckeletin/crate) crates and assert that their [dependencies]
//! sections contain EXACTLY the allowed set — no more, no less.
//!
//! **Purpose:** the boundary enforcement relies on these Cargo.toml files being
//! correct. A new forbidden dependency (e.g. clap added to domain) breaks the
//! architecture but may not immediately fail `cargo check`. These invariant tests
//! make a forbidden addition a loud CI failure instead of a silent drift.
//!
//! **Adding a legitimate dependency:**
//!   1. Add it to the Cargo.toml as usual.
//!   2. Add it to the allowlist constant below.
//!   3. Update the comment explaining why it belongs there.
//!
//! The test is intentionally strict: the allowlist is the complete [dependencies]
//! section, not just a denylist. This means adding ANY new dependency requires a
//! conscious update here — which is the point.

use std::collections::BTreeSet;

/// Resolve a path relative to the workspace root.
fn workspace_path(relative: &str) -> std::path::PathBuf {
    let from_crate_root = std::path::Path::new("../../").join(relative);
    if from_crate_root.exists() {
        return from_crate_root;
    }
    std::path::Path::new(relative).to_path_buf()
}

/// Parse the [dependencies] section of a Cargo.toml and return the set of
/// direct dependency names (not version-qualified, not dev-dependencies).
fn parse_dependencies(toml_path: &std::path::Path) -> BTreeSet<String> {
    let content = std::fs::read_to_string(toml_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", toml_path.display(), e));
    let parsed: toml::Value = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("cannot parse {}: {}", toml_path.display(), e));

    parsed
        .get("dependencies")
        .and_then(|d| d.as_table())
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

// ── domain: MUST depend only on serde ───────────────────────────
// CKSPEC-ARCH-003 (no clap), CKSPEC-ARCH-004 (no infra deps)
// If you need to add a legitimate dep to domain, add it here AND justify it.
// (spoiler: almost nothing belongs in domain — return data, let cli/infra process it)
const DOMAIN_ALLOWED_DEPS: &[&str] = &[
    "serde", // required: typed serialization for domain result types
];

// ── infrastructure: re-exports domain via the framework crate ──
// CKSPEC-ARCH-005: infrastructure MUST NOT import domain or cli.
// The framework crate (ckeletin) provides Output, Config, logging — infrastructure
// re-exports these without touching domain or cli.
const INFRA_ALLOWED_DEPS: &[&str] = &[
    "ckeletin", // framework crate: Output, Config, logging, build_info, catalog
];

// ── .ckeletin/crate: framework primitives — MUST NOT contain CLI frameworks ──
// The framework crate provides Output, Config, logging, and other shared services.
// It MUST NOT pull in clap or other CLI-framework deps (those belong only in cli).
// serde/figment/tracing are legitimate infrastructure primitives.
const FRAMEWORK_FORBIDDEN_DEPS: &[&str] = &[
    "clap",      // CLI framework — belongs ONLY in crates/cli
    "structopt", // older clap wrapper
    "argh",      // another CLI arg parser
    "pico-args", // CLI arg parser
];

#[test]
fn domain_dependencies_are_exactly_the_allowlist() {
    let path = workspace_path("crates/domain/Cargo.toml");
    let actual = parse_dependencies(&path);
    let allowed: BTreeSet<String> = DOMAIN_ALLOWED_DEPS.iter().map(|s| s.to_string()).collect();

    let extra: BTreeSet<_> = actual.difference(&allowed).collect();
    let missing: BTreeSet<_> = allowed.difference(&actual).collect();

    assert!(
        extra.is_empty(),
        "crates/domain/Cargo.toml [dependencies] contains forbidden entries: {:?}\n\
         Domain MUST depend only on serde (CKSPEC-ARCH-003/004).\n\
         To add a legitimate dep: add it to DOMAIN_ALLOWED_DEPS in arch_allowlist.rs and justify why.",
        extra
    );
    assert!(
        missing.is_empty(),
        "crates/domain/Cargo.toml [dependencies] is missing expected entries: {:?}\n\
         Update DOMAIN_ALLOWED_DEPS in arch_allowlist.rs to match the actual file.",
        missing
    );
}

#[test]
fn infrastructure_dependencies_are_exactly_the_allowlist() {
    let path = workspace_path("crates/infrastructure/Cargo.toml");
    let actual = parse_dependencies(&path);
    let allowed: BTreeSet<String> = INFRA_ALLOWED_DEPS.iter().map(|s| s.to_string()).collect();

    let extra: BTreeSet<_> = actual.difference(&allowed).collect();
    let missing: BTreeSet<_> = allowed.difference(&actual).collect();

    assert!(
        extra.is_empty(),
        "crates/infrastructure/Cargo.toml [dependencies] contains forbidden entries: {:?}\n\
         Infrastructure MUST NOT import domain or cli (CKSPEC-ARCH-005).\n\
         To add a legitimate dep: add it to INFRA_ALLOWED_DEPS in arch_allowlist.rs and justify why.",
        extra
    );
    assert!(
        missing.is_empty(),
        "crates/infrastructure/Cargo.toml [dependencies] is missing expected entries: {:?}\n\
         Update INFRA_ALLOWED_DEPS in arch_allowlist.rs to match the actual file.",
        missing
    );
}

#[test]
fn framework_crate_does_not_contain_cli_framework_deps() {
    let path = workspace_path(".ckeletin/crate/Cargo.toml");
    let actual = parse_dependencies(&path);

    let forbidden_found: Vec<&str> = FRAMEWORK_FORBIDDEN_DEPS
        .iter()
        .filter(|dep| actual.contains(**dep))
        .copied()
        .collect();

    assert!(
        forbidden_found.is_empty(),
        ".ckeletin/crate/Cargo.toml [dependencies] contains CLI framework dep(s): {:?}\n\
         The framework crate MUST NOT depend on CLI arg-parsing libraries — those belong only in crates/cli.\n\
         To remove: delete the dep from .ckeletin/crate/Cargo.toml.",
        forbidden_found
    );
}
