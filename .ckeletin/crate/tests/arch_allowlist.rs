// OUT-005 exception: test skip-signal writes to stderr are legitimate test-harness
// communication (not library output). The Output struct cannot be used here.
#![allow(clippy::print_stderr)]
//! Dependency allowlist invariant tests (CKSPEC-ARCH-003/004/005).
//!
//! These tests parse the Cargo.toml files for declared layer crates and assert
//! that their `[dependencies]` sections contain EXACTLY the allowed set — no
//! more, no less. The public-layer ban on domain, infrastructure, and cli
//! also reads `[dev-dependencies]`, `[build-dependencies]`, and those tables
//! under `[target]`.
//!
//! **Configuration:** consumer projects declare their layout and allowlists in
//! `ckeletin-project.toml` at the workspace root (project-owned, survives
//! `just ckeletin-update`). Framework-owned files (this file) are replaced on
//! every update — do NOT edit them to tailor allowlists. Put tailoring in
//! `ckeletin-project.toml` instead.
//!
//! **Behavior matrix:**
//!   a. Config present → enforce strictly per config:
//!      - declared layer crate missing → FAIL (you declared it)
//!      - undeclared dep in a declared layer → FAIL
//!   b. Config absent + scaffold layout present → enforce scaffold strict defaults
//!      (domain=["serde"], infra=["ckeletin"]) — keeps fresh scaffolds gated
//!   c. Config absent + scaffold layout absent → SKIP LOUDLY: eprintln a nudge
//!      naming ckeletin-project.toml. Never panic.
//!
//! **The framework-crate self-check** (`.ckeletin/crate` must not contain CLI
//! frameworks) stays HARDCODED — that is a framework fact, true everywhere,
//! not project-configurable.

use std::collections::BTreeSet;

use ckeletin::project_config;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Resolve the workspace root from CARGO_MANIFEST_DIR (.ckeletin/crate).
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR"); // .ckeletin/crate
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

/// Parse the [dependencies] section of a Cargo.toml and return the set of
/// direct dependency names (not version-qualified, not dev-dependencies).
fn parse_dependencies(toml_path: &std::path::Path) -> BTreeSet<String> {
    project_config::manifest_deps(toml_path)
        .unwrap_or_else(|e| panic!("{e}"))
        .normal
}

fn package_name(toml_path: &std::path::Path) -> String {
    let content = std::fs::read_to_string(toml_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", toml_path.display(), e));
    let parsed: toml::Value = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("cannot parse {}: {}", toml_path.display(), e));
    parsed
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or_else(|| panic!("{} has no [package] name", toml_path.display()))
        .to_string()
}

fn declared_package_names(
    root: &std::path::Path,
    dirs: &[String],
    layer: &str,
) -> BTreeSet<String> {
    dirs.iter()
        .map(|dir| {
            let cargo_toml = root.join(dir).join("Cargo.toml");
            assert!(
                cargo_toml.exists(),
                "{layer} crate declared in ckeletin-project.toml [layers] not found: {}",
                cargo_toml.display()
            );
            package_name(&cargo_toml)
        })
        .collect()
}

// ── .ckeletin/crate: framework primitives — hardcoded, always enforced ────────
//
// The framework crate (ckeletin) provides Output, Config, logging — it MUST NOT
// pull in clap or other CLI-framework deps (those belong only in cli).
// This check is framework-owned and not overrideable by ckeletin-project.toml.
const FRAMEWORK_FORBIDDEN_DEPS: &[&str] = &[
    "clap",      // CLI framework — belongs ONLY in crates/cli
    "structopt", // older clap wrapper
    "argh",      // another CLI arg parser
    "pico-args", // CLI arg parser
];

#[test]
fn framework_crate_does_not_contain_cli_framework_deps() {
    let root = workspace_root();
    let path = root.join(".ckeletin/crate/Cargo.toml");
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

// ── domain / infrastructure: driven by ckeletin-project.toml ──────────────────

#[test]
fn domain_dependencies_are_exactly_the_allowlist() {
    let root = workspace_root();

    let outcome = project_config::load(&root)
        .unwrap_or_else(|e| panic!("ckeletin-project.toml parse error: {e}"));

    if outcome.should_skip() {
        eprintln!(
            "SKIP domain_dependencies_are_exactly_the_allowlist: \
             no ckeletin-project.toml and scaffold layout absent.\n\
             Declare your layer crates and allowlists in ckeletin-project.toml \
             at the workspace root to enable this gate."
        );
        return;
    }

    let config = outcome.config();

    for domain_dir in &config.layers.domain {
        let cargo_toml = root.join(domain_dir).join("Cargo.toml");
        assert!(
            cargo_toml.exists(),
            "Domain crate declared in ckeletin-project.toml [layers] not found: {}\n\
             Either the path is wrong or the crate has not been created yet.",
            cargo_toml.display()
        );

        let actual = parse_dependencies(&cargo_toml);
        let public_names = declared_package_names(&root, &config.layers.public, "Public");
        let (extra, missing) =
            project_config::allowlist_difference(&actual, &config.allowlists.domain, &public_names);

        assert!(
            extra.is_empty(),
            "{}/Cargo.toml [dependencies] contains forbidden entries: {:?}\n\
             Domain MUST depend only on the allowlist (CKSPEC-ARCH-003/004).\n\
             To add a legitimate dep: add it to [allowlists] domain in \
             ckeletin-project.toml and add a comment justifying it.",
            domain_dir,
            extra
        );
        assert!(
            missing.is_empty(),
            "{}/Cargo.toml [dependencies] is missing expected entries: {:?}\n\
             Update [allowlists] domain in ckeletin-project.toml to match the actual file.",
            domain_dir,
            missing
        );
    }
}

#[test]
fn infrastructure_dependencies_are_exactly_the_allowlist() {
    let root = workspace_root();

    let outcome = project_config::load(&root)
        .unwrap_or_else(|e| panic!("ckeletin-project.toml parse error: {e}"));

    if outcome.should_skip() {
        eprintln!(
            "SKIP infrastructure_dependencies_are_exactly_the_allowlist: \
             no ckeletin-project.toml and scaffold layout absent.\n\
             Declare your layer crates and allowlists in ckeletin-project.toml \
             at the workspace root to enable this gate."
        );
        return;
    }

    let config = outcome.config();

    for infra_dir in &config.layers.infrastructure {
        let cargo_toml = root.join(infra_dir).join("Cargo.toml");
        assert!(
            cargo_toml.exists(),
            "Infrastructure crate declared in ckeletin-project.toml [layers] not found: {}\n\
             Either the path is wrong or the crate has not been created yet.",
            cargo_toml.display()
        );

        let actual = parse_dependencies(&cargo_toml);
        let public_names = declared_package_names(&root, &config.layers.public, "Public");
        let (extra, missing) = project_config::allowlist_difference(
            &actual,
            &config.allowlists.infrastructure,
            &public_names,
        );

        assert!(
            extra.is_empty(),
            "{}/Cargo.toml [dependencies] contains forbidden entries: {:?}\n\
             Infrastructure MUST NOT import domain or cli (CKSPEC-ARCH-005).\n\
             To add a legitimate dep: add it to [allowlists] infrastructure in \
             ckeletin-project.toml and add a comment justifying it.",
            infra_dir,
            extra
        );
        assert!(
            missing.is_empty(),
            "{}/Cargo.toml [dependencies] is missing expected entries: {:?}\n\
             Update [allowlists] infrastructure in ckeletin-project.toml to match \
             the actual file.",
            infra_dir,
            missing
        );
    }
}

#[test]
fn public_dependencies_are_exactly_the_allowlist_and_do_not_reach_private_layers() {
    let root = workspace_root();

    let outcome = project_config::load(&root)
        .unwrap_or_else(|e| panic!("ckeletin-project.toml parse error: {e}"));

    if outcome.should_skip() {
        eprintln!(
            "SKIP public_dependencies_are_exactly_the_allowlist_and_do_not_reach_private_layers: \
             no ckeletin-project.toml and scaffold layout absent.\n\
             Declare a public layer in ckeletin-project.toml to enable this gate."
        );
        return;
    }

    let config = outcome.config();
    if config.layers.public.is_empty() {
        eprintln!(
            "SKIP public_dependencies_are_exactly_the_allowlist_and_do_not_reach_private_layers: \
             [layers] public is empty. Declare public crates to enable this gate."
        );
        return;
    }

    let public_names = declared_package_names(&root, &config.layers.public, "Public");
    let mut private = declared_package_names(&root, &config.layers.domain, "Domain");
    private.extend(declared_package_names(
        &root,
        &config.layers.infrastructure,
        "Infrastructure",
    ));
    private.extend(declared_package_names(&root, &config.layers.cli, "CLI"));

    for public_dir in &config.layers.public {
        let cargo_toml = root.join(public_dir).join("Cargo.toml");
        let manifest = project_config::manifest_deps(&cargo_toml).unwrap_or_else(|e| panic!("{e}"));
        let actual = manifest.normal;
        let reachable = manifest.packages;
        let self_name = package_name(&cargo_toml);
        let mut other_public = public_names.clone();
        other_public.remove(&self_name);

        let forbidden = project_config::private_layer_deps(&reachable, &private);
        assert!(
            forbidden.is_empty(),
            "{public_dir}/Cargo.toml depends on private layer crate(s) {forbidden:?}.\n\
             A public crate MUST NOT name domain, infrastructure, or cli in \
             dependencies, dev-dependencies, or build-dependencies (CKSPEC-ARCH-007)."
        );

        let (extra, missing) =
            project_config::allowlist_difference(&actual, &config.allowlists.public, &other_public);
        assert!(
            extra.is_empty(),
            "{public_dir}/Cargo.toml [dependencies] contains forbidden entries: {extra:?}\n\
             Public deps must match [allowlists] public in ckeletin-project.toml.\n\
             Another declared public crate may be an extra path dependency."
        );
        assert!(
            missing.is_empty(),
            "{public_dir}/Cargo.toml [dependencies] is missing expected entries: {missing:?}\n\
             Update [allowlists] public in ckeletin-project.toml to match the actual file."
        );
    }
}
