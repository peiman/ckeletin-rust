# ckeletin Framework Changelog

## [0.2.10] - 2026-06-04

### Added
- **SBOM generation + vulnerability scanning (supply-chain readiness).**
  - `ckeletin-sbom` recipe — generates `sbom.cdx.json`, a CycloneDX 1.5 SBOM of
    the CLI binary's full dependency graph, using
    [cargo-cyclonedx](https://github.com/CycloneDX/cyclonedx-rust-cargo) (the
    official OWASP CycloneDX cargo plugin; stable toolchain, no nightly).
  - `ckeletin-sbom-scan` recipe — generates then scans the SBOM with
    [grype](https://github.com/anchore/grype), failing on High severity or above.
  - Both standalone (external tools, not in `just check`). `ckeletin-doctor`
    reports cargo-cyclonedx + grype presence. Generated `*.cdx.json` are
    gitignored.
  - Worked example (project-owned): a `sbom` CI job that generates + scans and
    uploads the SBOM as a build artifact for compliance/consumers.
  Chose the Rust-native OWASP generator over syft for a leaner footprint (one
  external binary) while keeping grype for parity with ckeletin-go's scanner.

## [0.2.9] - 2026-06-04

### Added
- **Secret scanning with gitleaks (CKSPEC-ENF-001).** Detects hardcoded
  credentials committed to the repo, using the industry-standard
  [gitleaks](https://github.com/gitleaks/gitleaks) (MIT, single static binary).
  - `ckeletin-secrets` recipe — scans the working tree. Standalone, not part of
    `just check` (gitleaks is an external non-cargo tool, so a missing gitleaks
    never blocks the cargo gate).
  - `.ckeletin/configs/gitleaks.toml` — framework default config (extends the
    built-in ruleset, excludes `target/`); override via a root `.gitleaks.toml`.
  - `ckeletin-doctor` now reports gitleaks presence.
  - Worked examples (project-owned, kept/replaced by adopters): a lefthook
    pre-commit staged scan that skips cleanly when gitleaks is absent but fails
    on a real secret, and a `secret-scan` CI job that scans full git history via
    the gitleaks **CLI** (not the commercial gitleaks-action).
  Mirrors ckeletin-go's secret scanning.

## [0.2.8] - 2026-06-04

### Added
- **`ckeletin-doctor` recipe.** Reports the development environment — framework
  version, pinned toolchain + MSRV (read from `rust-toolchain.toml` / `Cargo.toml`,
  so it stays SSOT) and installed `rustc`, plus presence of the tools the
  framework depends on (`cargo-deny`, `cargo-llvm-cov`, optional `cargo-nextest`,
  `just`, and the rustfmt/clippy components). Informational only — always exits 0,
  so it is intentionally not part of `just check`. Mirrors ckeletin-go's
  `task doctor`. Smoke test: `.ckeletin/crate/tests/doctor.rs`.
- **`ckeletin-version` recipe.** Prints the framework version (parity with
  ckeletin-go's `task version`).

### Notes
- Remaining ckeletin-go tasks are deliberately not ported. The `validate:*`
  ADR-enforcement suite is already achieved at compile time (trybuild violation
  tests + `framework_purity`) and by `conform`; the `check:*`/`test:*`/`build:*`
  variants collapse into the single `check` gateway and standard cargo; and
  GoReleaser/`generate:config:*`/`tidy` are Go-toolchain specific. Heavier
  capabilities (secret scanning, SAST, SBOM, fuzzing, benchmarks, `setup`) remain
  open decisions rather than silent external-tool dependencies.

## [0.2.7] - 2026-06-04

### Added
- **`ckeletin-update-check-compatibility` recipe.** Applies the upstream
  `.ckeletin/` to the working tree, runs `cargo check --workspace`, then
  restores the committed framework via a trap (interrupt-safe) — letting an
  adopter confirm an update compiles against their code without keeping it.
  Brings the Rust framework to parity with ckeletin-go's
  `task ckeletin:update:check-compatibility`. No import rewriting is needed
  (Rust references crates by name, not an embedded module path).
- **Upstream self-guard on the update recipes.** `ckeletin-update`,
  `ckeletin-update-dry-run`, and `ckeletin-update-check-compatibility` now
  short-circuit (exit 0 with a message) when run inside the ckeletin-rust
  upstream repo itself, detected via the workspace `repository` slug in the
  root `Cargo.toml` (`just init` rewrites it for derived projects). Mirrors
  ckeletin-go's go.mod module-path guard. Regression test:
  `.ckeletin/crate/tests/update_guard.rs`.

### Changed
- The upstream remote URL and identity slug are now SSOT `just` variables
  (`ckeletin_upstream_url`, `ckeletin_upstream_slug`) instead of being
  hardcoded across the update recipes.

## [0.2.6] - 2026-06-04

### Added
- **Anchored conformance evidence (CKSPEC-ENF-008).** `just conform` now
  exits non-zero on any `met` requirement that has no automated check, no
  violation test, and no written `violation_evidence` — an unbacked "met"
  can no longer pass the gate or reach the published report. The gate
  (`lacks_anchor`) runs after the completeness check; unit tests
  `anchored_met_passes` / `unanchored_met_is_rejected` prove it.
- **Published machine-readable conformance report (CKSPEC-ENF-010).** The
  generator projects `conformance-mapping.toml` into a deterministic
  `conformance-report.json` at the repo root — sorted requirement keys,
  alphabetical fields, **no timestamp** — so it is byte-stable and a spec
  repo can aggregate it instead of hand-authoring (the aggregator stamps
  the fetch date). `just conform` regenerates it in memory and **fails on
  drift** (sync-check); `just conform-report` rewrites it. Schema mirrors
  ckeletin-go's report (`implementation`, `requirements`, `spec_version`,
  `summary`). Unit tests `report_projection_is_deterministic` /
  `sync_check_detects_drift`.
- `conform-report` recipe in `.ckeletin/Justfile` — regenerate the
  published report after editing the mapping.

### Notes
- CKSPEC-ENF-009 (conformance gate on release) is wired at the project
  level, not the framework level: a tag-triggered `release.yml` gates its
  publish job on the `conform` job, and a scheduled `spec-drift.yml`
  watches the live upstream spec. These ship as worked examples adopters
  keep or replace, like `ci.yml`.

## [0.2.5] - 2026-06-03

### Added
- **Build identity (`build_info::BuildInfo`).** A prefix-agnostic framework
  primitive that surfaces the git provenance baked into a binary at compile
  time — version + commit + date + dirty — rendered by `version_line()`
  (mirrors ckeletin-go's `--version`: `"<version>, commit <commit>, built
  <date> (dirty)"`). The scaffold ships the worked example of consuming it:
  `crates/cli/build.rs` bakes the identity (one atomic `git describe --dirty`,
  so there is no false-clean gap; degrades to `unknown` on any git failure) and
  a `version` command renders it in human + JSON, with `--version` wired to the
  same formatter. Build-identity surfacing only; runtime staleness checking is
  left to the adopter (out of the shared cross-language contract). First
  consumer: workhorse (SH-004). Implements CKSPEC-OUT-006.

## [0.2.4] - 2026-05-31

### Changed
- The audit log (CKSPEC-OUT-004) now defaults to a stable per-user location
  instead of `./logs/` relative to the working directory. A relative
  `log_file_path` is anchored under `~/.config/<app>/` by default (XDG-style,
  uniform on every platform including macOS). New `log_location` config field:
  `"config"` (default) or `"platform"` (the OS-native app-data dir, e.g.
  `~/Library/Application Support/<app>` on macOS). An absolute `log_file_path`
  still overrides entirely. Resolution is dependency-free (env vars only — no
  new crates, no copyleft). The first-run notice prints the resolved path.

## [0.2.3] - 2026-05-29

### Changed
- Audit logging (CKSPEC-OUT-004) is now **on by default**
  (`Config.log_file_enabled` defaults to `true`), and
  `Output::success`/`message`/`error` shadow-log the *rendered data*, not
  just the command name — so the audit stream contains what the user saw.
  Downstream projects receive this on `just ckeletin-update` and will start
  writing `logs/app.log` by default; opt out with `log_file_enabled = false`
  (or the `--no-audit` flag if the consumer wires it into its CLI).

### Fixed
- `just init <name>` produced a non-compiling, un-committed project.
  The strip-demo step deleted `ping` (the only subcommand), leaving an
  empty `Commands` enum the entry point could not match exhaustively,
  and a `sed '/ping/Id'` line delete mangled the integration-test file
  into invalid Rust. init now keeps `ping` as the renamed worked
  example (as the ckeletin-go scaffold does) and verifies with
  `cargo check --all-targets`. The `init_smoke` test now builds and
  tests the initialized project, and CI gates it (upstream-only).
  Fixes #1.

### Security
- Bumped `rustls-webpki` to 0.103.13 (RUSTSEC-2026-0104: reachable
  panic parsing certificate revocation lists).

## [0.2.2] - 2026-04-22

### Added
- `Output::message(command, msg, writer)` — emit a human-addressed
  success response with no structured data. Human mode writes the
  message with a trailing newline; JSON mode wraps it in an
  envelope with `data: {"message": msg}` (structured, not a raw
  string blob in the data slot). Replaces the common wart of
  passing `&format!("...")` to `Output::success` for "no data to
  report" success paths.

### Spec alignment
- Neither CKSPEC-OUT-003 nor CKSPEC-OUT-005 forbade the prior
  pattern — it produced structurally valid envelopes — but the
  structure was inconsistent. `Output::message` formalizes the
  no-data-success shape so downstream consumers can rely on
  `data.message` always being a string.

## [0.2.0] - 2026-04-14

### Added
- Extracted framework library into `.ckeletin/crate/`
- Output, config, logging, process modules from infrastructure
- Framework update mechanism (`just ckeletin-update`)
- Init flow (`just init name=<name>`)
- Violation test templates in `.ckeletin/tests/violations/`
- Two-level Justfile: framework tasks in `.ckeletin/Justfile`
