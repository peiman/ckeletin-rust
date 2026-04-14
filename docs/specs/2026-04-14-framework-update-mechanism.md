# Framework Update Mechanism for ckeletin-rust

**Date:** 2026-04-14
**Status:** Design approved
**Scope:** Restructure ckeletin-rust to support framework updates, project initialization, and migration of existing projects

## Problem

Projects built from ckeletin-rust receive no upstream improvements. When the scaffold's infrastructure code improves (output, config, logging, conformance tooling), every project must manually copy changes and fix crate name mismatches. This doesn't survive time pressure (Principle 9: Automated Enforcement).

The manual `sed` rename during project creation also breaks `.stderr` violation test files. There is no init flow and no update flow.

## Design Principles Applied

- **Truth-Seeking (1):** Framework vs project code has an explicit, enforceable boundary
- **Lean Iteration (4):** Vendored path dependency now; crates.io publishing when API stabilizes
- **Platforms, Not Features (5):** The update mechanism is a platform — every future framework improvement flows through it
- **Single Source of Truth (7):** Framework code lives in one place (`.ckeletin/`), not duplicated across projects
- **Separation of Concerns (8):** Framework concerns (output, config, logging) separated from project concerns (business logic, commands)
- **Automated Enforcement (9):** Cargo workspace structure enforces architecture boundaries at compile time
- **Feedback Cycle (10):** Projects discover improvements, contribute back to framework, framework distributes to all projects via update

## Key Insight: Rust Doesn't Need Import Rewriting

Go's ckeletin-go requires AST-based import rewriting on every init and update because Go imports are file paths (`github.com/peiman/ckeletin-go/pkg/output`). Forking a project means every import carries the old module name.

Rust separates naming from location. A crate's name is declared in `Cargo.toml` and resolved by Cargo. `use ckeletin::output::Output` works in every project regardless of the project name. The framework crate is always called `ckeletin`. No renaming. No AST tools.

This makes init trivial (set binary name, strip demos) and update trivial (replace `.ckeletin/` directory).

## Directory Layout

```
project-root/
├── .ckeletin/                          # FRAMEWORK-OWNED — replaced wholesale on update
│   ├── VERSION                         # Framework version (e.g., "0.2.0")
│   ├── CHANGELOG.md                    # Framework change history
│   ├── crate/                          # Framework library crate
│   │   ├── Cargo.toml                  # name = "ckeletin"
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── output.rs               # Envelope, renderer, modes
│   │       ├── config.rs               # Figment loader
│   │       ├── logging.rs              # Tracing setup, guards
│   │       └── process.rs              # Command execution
│   ├── conform/                        # Conformance generator
│   │   ├── Cargo.toml                  # name = "ckeletin-conform"
│   │   └── src/main.rs
│   ├── tests/                          # Violation test templates
│   │   └── violations/
│   ├── Justfile                        # Framework recipes (ckeletin-* prefix)
│   └── scripts/
│       ├── init.sh                     # Project initialization
│       ├── update.sh                   # Framework update
│       └── migrate.sh                  # One-time migration for existing projects
│
├── crates/                             # PROJECT-OWNED — never touched by update
│   ├── domain/                         # name = "domain"
│   │   ├── Cargo.toml                  # depends on: serde only
│   │   ├── src/                        # Business logic
│   │   └── tests/violations/           # Architecture violations (from template)
│   ├── infrastructure/                 # name = "infrastructure"
│   │   ├── Cargo.toml                  # depends on: ckeletin + project-specific deps
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports ckeletin + project modules
│   │       └── ...                     # Project-specific modules
│   └── cli/                            # name = "cli"
│       ├── Cargo.toml                  # [[bin]] name = "myproject"
│       ├── src/
│       └── tests/
│
├── Cargo.toml                          # workspace members: crates/*, .ckeletin/crate, .ckeletin/conform
├── Justfile                            # Project recipes, imports .ckeletin/Justfile
├── conformance-mapping.toml            # Project-owned
├── deny.toml                           # Project-owned (framework provides default on init)
├── AGENTS.md
├── CLAUDE.md
├── CHANGELOG.md
└── lefthook.yml
```

## Ownership Rules

| Location | Owner | Init | Update | Migrate |
|----------|-------|------|--------|---------|
| `.ckeletin/` | Framework | Untouched | Replaced wholesale | Created from upstream |
| `crates/` | Project | Demo stripped, names set | Never touched | Crates renamed, imports rewritten |
| Root config | Project | Templates created | Never touched | Untouched |
| `Cargo.toml` (workspace) | Project | Members set | Never touched | Members updated |
| `conformance-mapping.toml` | Project | Reset to deferred | Never touched | Untouched |

## Crate Naming

Framework crates carry the `ckeletin` prefix. Project crates are clean:

| Crate | Name | Role |
|-------|------|------|
| `.ckeletin/crate` | `ckeletin` | Framework library — output, config, logging, process |
| `.ckeletin/conform` | `ckeletin-conform` | Conformance generator tool |
| `crates/domain` | `domain` | Business logic — no I/O, no framework deps |
| `crates/infrastructure` | `infrastructure` | Re-exports ckeletin + project-specific I/O |
| `crates/cli` | `cli` | Commands, arg parsing. `[[bin]] name` = project name |

Project code never writes `use ckeletin::` directly. Infrastructure re-exports framework modules:

```rust
// crates/infrastructure/src/lib.rs
pub use ckeletin::config;
pub use ckeletin::logging;
pub use ckeletin::output;
pub use ckeletin::process;

// Project-specific modules below
```

CLI and domain import from `infrastructure` and `domain`:

```rust
use domain::ping;
use infrastructure::output::Output;
```

## Dependency Graph

```
domain          → serde                        (pure business logic)
infrastructure  → ckeletin, serde, figment...  (framework + project I/O)
cli             → domain, infrastructure, clap (convergence)
ckeletin        → serde, serde_json, figment, tracing, thiserror (framework deps)
ckeletin-conform → serde, serde_json, toml, ureq (standalone tool)
```

Compile-time enforcement: domain has no dependency on infrastructure or ckeletin. Any reverse import is a compile error. Violation tests (trybuild) verify this.

## Init Flow

`just init name=myproject` — run once after cloning.

1. Set `[[bin]] name` in `crates/cli/Cargo.toml` to `myproject`
2. Update workspace `Cargo.toml` metadata (repository, description)
3. Update root `Justfile` `binary_name` variable
4. Strip demo code: `crates/domain/src/ping.rs`, `crates/cli/src/ping.rs`, ping references in tests
5. Copy violation test templates from `.ckeletin/tests/violations/` to `crates/domain/tests/` and `crates/infrastructure/tests/`
6. Reset `CHANGELOG.md` to empty Keep a Changelog template
7. Reset `conformance-mapping.toml` — framework checks `met`, project-specific `deferred`
8. Reset git history: `git init`, initial commit, `v0.0.0` tag
9. Verify: `cargo check --workspace`

No crate renaming. No import rewriting. Five string replacements in known files.

## Update Flow

`just ckeletin-update` — run when framework has improvements.

1. Add `ckeletin-upstream` remote (first time): `git remote add ckeletin-upstream https://github.com/peiman/ckeletin-rust.git`
2. Fetch: `git fetch ckeletin-upstream` (or pinned: `version=v0.2.0`)
3. Replace: `git checkout ckeletin-upstream/main -- .ckeletin/`
4. Post-update migrations: run `.ckeletin/scripts/migrate.sh` if breaking changes exist between old and new VERSION
5. Verify: `cargo check --workspace` — if fails, rollback `.ckeletin/` and report error
6. Show changes: diff `.ckeletin/CHANGELOG.md` between old and new version
7. Commit: `git add .ckeletin/ && git commit -m "chore: update ckeletin framework to vX.Y.Z"`

Dry run available: `just ckeletin-update-dry-run` — shows what would change without applying.

No import rewriting. The crate name `ckeletin` is stable across all versions. `use ckeletin::output::Output` resolves to the new code automatically.

## Migration Flow (Existing Projects)

`just ckeletin-migrate prefix=workhorse` — one-time conversion for projects built from the old scaffold.

Example: workhorse has `workhorse-domain`, `workhorse-infrastructure`, `workhorse-cli`.

1. Create `.ckeletin/` from upstream (same as update step 1-3)
2. Rename crates in all `Cargo.toml` files:
   - `workhorse-domain` → `domain`
   - `workhorse-infrastructure` → `infrastructure`
   - `workhorse-cli` → `cli`
3. Rewrite imports in all `.rs` files:
   - `workhorse_domain` → `domain`
   - `workhorse_infrastructure` → `infrastructure`
4. Add `ckeletin` dependency to `crates/infrastructure/Cargo.toml`
5. Add re-exports to `crates/infrastructure/src/lib.rs`:
   - `pub use ckeletin::{config, logging, output, process};`
6. Remove framework code from `crates/infrastructure/src/` that is now provided by ckeletin (output.rs, config.rs, logging.rs, process.rs) — keep project-specific modules
7. Update workspace `Cargo.toml` members to include `.ckeletin/crate` and `.ckeletin/conform`
8. Regenerate violation test `.stderr` files (crate names changed → error messages changed)
9. Verify: `just check` (full suite)

After migration, `just ckeletin-update` works going forward. The migration is one-time.

## Justfile Structure

**`.ckeletin/Justfile`** — framework recipes, all `ckeletin-` prefixed:

- `ckeletin-update` / `ckeletin-update-dry-run` — framework update
- `ckeletin-health` — framework version and compile check
- `ckeletin-check` — fmt, clippy, deny (framework-provided quality gates)
- `conform` — run conformance generator

**Root `Justfile`** — project recipes, imports framework:

```just
import '.ckeletin/Justfile'

binary_name := "myproject"

check: ckeletin-check test
    @echo "All checks passed."

test:
    cargo nextest run --workspace 2>/dev/null || cargo test --workspace

# ... project-specific recipes
```

`deny.toml` stays at project root — project-owned. Framework provides a default during init. Projects evolve it as they add dependencies.

## Testing Strategy

**Framework tests:** Unit tests inside `.ckeletin/crate/` run as part of `cargo test --workspace`. These test output, config, logging, process in isolation.

**Violation test templates:** `.ckeletin/tests/violations/` contains the template `.rs` files. During init, these are copied to project `crates/*/tests/violations/` with correct crate names. After copy, they are project-owned — the `.stderr` files contain project-specific error messages.

**Update verification:** After every update, `cargo check --workspace` runs. Failure triggers rollback of `.ckeletin/` to the previous version.

**Migration verification:** After migration, `just check` (full suite: fmt, clippy, tests, deny) runs.

## Future: Publishing to crates.io

When the framework API stabilizes, `ckeletin` can be published to crates.io. Projects switch from:
```toml
ckeletin = { path = "../../.ckeletin/crate" }
```
to:
```toml
ckeletin = "1.0"
```

One line change. Updates via `cargo update`. The `.ckeletin/` directory would then only contain non-code files (Justfile, scripts, conform, violation templates). The library code moves to crates.io with semver protection.

This is a future step. The vendored model works now and gives us iteration speed.
