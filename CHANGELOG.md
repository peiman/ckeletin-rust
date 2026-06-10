# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Machine-readable command catalog (CKSPEC-AGENT-006)** (#25/#26): `catalog`
  subcommand derives a JSON catalog directly from the live clap command tree —
  the catalog and the parser share one tree so structural drift is impossible.
  Spec advanced to v0.8.0 adding this requirement; implementation immediately
  met it. Framework bump to 0.2.16.
- **Update compatibility check** (#16): `just ckeletin-update-check-compatibility`
  applies a candidate framework update, runs the full `just check` gate, and
  restores the original framework via a trap (interrupt-safe). Mirrors
  ckeletin-go's `task ckeletin:update:check-compatibility`. Upstream self-guard
  prevents the update recipes from running on the upstream repo itself.
- **Doctor and version recipes** (#17): `just ckeletin-doctor [json]`
  reports the dev environment (framework version, toolchain, tool presence)
  and accepts `json` (positional) for machine-readable output: `just ckeletin-doctor json`.
  `just ckeletin-version` mirrors ckeletin-go's `task version`.
- **Secret scanning with gitleaks** (#18): `just ckeletin-secrets` scans the
  working tree for hardcoded secrets. Gated via the `secret-scan` CI job and
  the lefthook pre-commit hook; excluded from `just check` so a missing
  gitleaks binary never blocks the build gate.
- **SBOM generation and vulnerability scanning** (#19): `just ckeletin-sbom`
  emits a CycloneDX 1.5 SBOM (`sbom.cdx.json`) via cargo-cyclonedx;
  `just ckeletin-sbom-scan` scans it with grype, failing on High severity.
  Both are advisory/standalone — not part of `just check`.
- **SAST: clippy hardening and cargo-geiger** (#20): additional clippy lints
  deny'd workspace-wide (`cast_possible_truncation`, `lossy_float_literal`,
  `dbg_macro`, `todo`, etc.); `float_cmp` scoped to `--lib --bins` only
  (test assertions against 0.0/1.0 are idiomatic and should not be denied).
  `just ckeletin-geiger` reports the `unsafe` surface (advisory, never gates).
- **Fuzzing worked example with bolero** (#21): `fuzz_ping` target under
  `crates/domain/tests/`; exercises bolero's generative testing on stable
  toolchain via `just ckeletin-fuzz`. Pedagogical template — not a meaningful
  guard for the trivial shipped type.
- **Agent-drivable update/diagnostic surface** (#23): `just ckeletin-check-update`
  accepts `json` as a positional argument (`just ckeletin-check-update json`) and emits
  `{"current","latest","update_available"}` for autonomous agents. `just ckeletin-update`
  now emits a machine-readable `CKELETIN_UPDATE_RESULT={"status","from","to","committed","rolled_back"}`
  verdict on every exit path.
- **Conformance brought up to spec v0.7.0** (#14, framework 0.2.6): 39
  requirements all met, adopting the published-report aggregation model.
  Four new requirements: CKSPEC-OUT-006 (build identity), CKSPEC-ENF-008
  (anchored evidence), CKSPEC-ENF-009 (release gate), CKSPEC-ENF-010
  (published machine-readable report). `conformance-report.json` is now a
  deterministic artifact the spec-repo can aggregate; `just conform` sync-checks
  it and fails on drift.
- `Output::message(command, msg, writer)` — framework helper for
  no-structured-data success paths (e.g. "no recorded history
  yet"). Human mode writes the sentence + newline; JSON mode wraps
  in `data: {"message": msg}` so downstream JSON consumers always
  find a structured object in the `data` slot (never a raw string
  blob). Framework version bumped to 0.2.2. Regression tests in
  `.ckeletin/crate/src/output.rs`.
- `--no-audit` global flag and on-by-default audit logging
  (CKSPEC-OUT-004). The audit log (`logs/app.log`) is written by
  default and shadow-logs the rendered data; `Output::success`/`message`/`error`
  all emit it. A one-time first-run notice on stderr (human mode only) names
  the log directory and the filename pattern (stem + date suffix), and points
  at the off-switches. `logs/` is gitignored.

### Changed
- **`just ckeletin-update` now enforces two-tier validation** (#22/#30):
  Tier 1 (`cargo check`) rolls back fully on compile failure. Tier 2
  (`just check`) leaves the tree dirty on failure so you can fix forward —
  previously, auto-committing a red `just check` was a real failure mode.
  `CKELETIN_UPDATE_RESULT` verdict emitted on every exit path (updated/
  compile_failed/check_failed with committed/rolled_back booleans).
- **`ckeletin-health` exits non-zero on broken workspace** (#30): previously
  `just check` could succeed on an uncompilable workspace if `ckeletin-health`
  ran last but the check step didn't fail the gate. Now health explicitly
  propagates a `cargo check --workspace` failure.
- **`just conform` no-ops in consumer repos** (#24): previously the recipe
  errored when `conformance/requirements.json` was absent (consumer repos have
  no spec). Now it exits 0 with an explanatory message. Same guard added to
  `conform-refresh` and `conform-report`.
- Audit log defaults to `~/.config/<app>/logs/` (XDG-style, uniform across
  platforms) instead of `./logs/` relative to the working directory. A new
  `log_location` config field selects `"config"` (default) or `"platform"`
  (OS-native app-data dir); an absolute `log_file_path` overrides both.
- Dependency maintenance (Dependabot batch, #15): bumped `toml` 0.8 → 1 in
  the conformance generator; patch bumps for `serde_json`, `clap`,
  `assert_cmd`, `tracing-appender`; CI actions: `actions/checkout` v4 → v6,
  `actions/cache` v4 → v5.
- Pinned the Rust toolchain to 1.96.0 via `rust-toolchain.toml`.
- Conformance reporting reconciled to spec v0.4.0 (35/35 met, was 32/3
  deferred against a stale v0.3.0 snapshot).

### Fixed
- **Audit-stream integrity and output-mode SSOT** (#31): output mode was
  computed independently in two places, producing a mixed-mode binary
  (success = JSON envelope, error = human stderr) when `--output json` was
  activated via config/env. Now computed once in `main.rs::resolve_output_mode`
  (SSOT). The `LogGuard` is now carried in `RunError::PostConfig` so the audit
  worker stays alive through `Output::error`, ensuring error shadow-log events
  reach the audit file. `--output text` now correctly overrides `json=true`
  config in both directions.
- **Reachable panic in audit logging → typed error** (#31): `tracing_appender::
  rolling::daily` panicked (exit 101) when the audit file couldn't be created
  in an existing directory (e.g. after a `sudo` run left a root-owned log file).
  Replaced with `RollingFileAppender::builder()` (returns `Result`); all
  failures flow to the caller as a clean error envelope + exit 1.
- **Invalid log level strings are now startup errors** (#31): previously a
  misspelled `log_level` or `log_file_level` was silently ignored, potentially
  emptying the audit stream without warning. `logging::init` now validates the
  level string and returns `Err` on anything not in
  `trace|debug|info|warn|error|off`.
- **Audit log directory and files get restricted permissions** (#31, Unix):
  directory created with mode 0700; initial log file set to 0600. Audit
  contents are per-user and must not be world-readable.
- **`just ckeletin-update` apply step is true wholesale replacement** (#30):
  replaced `git checkout <ref> -- .ckeletin/` with `git restore --source=<ref>
  --staged --worktree -- .ckeletin/`, which deletes files absent from the
  source ref. The previous command left upstream-deleted files in place — a
  latent-broken "replaced wholesale" promise that would have surfaced on the
  first file deletion upstream.
- **Canonical Apache-2.0 license** (#29): previous `LICENSE-APACHE` was an
  abridged paraphrase missing the patent-litigation termination clause, §9
  indemnification condition, and APPENDIX. Replaced with the verbatim canonical
  text. A `license_integrity` test pins the SHA-256 so the file cannot silently
  drift again.
- **Framework crate version sync** (#29): framework crate `version` in
  `.ckeletin/crate/Cargo.toml` now matches `.ckeletin/VERSION`. A
  `version_sync` test enforces the invariant going forward.
- **Build identity self-heals after `git init`** (#31, scaffold): `build.rs`
  is re-run after the initial commit during `just init`, so the first build
  bakes real identity rather than "unknown". Emits `cargo:warning=` when
  degrading to "unknown" so the degradation is visible at build time.
- Error envelope in JSON mode now correctly identifies the failing subcommand
  via a `subcommand_name` helper with an exhaustive `match` — no silent "init"
  fallback.
- `just init <name>` now produces a project that compiles and is a committed
  git repository. The init script previously stripped the `ping` demo command,
  leaving an empty `Commands` enum that `main.rs` could not match exhaustively.

### Security
- **Supply-chain hardening** (#27): all GitHub Actions in ci/release/spec-drift
  SHA-pinned to full commit SHAs with `# vX.Y.Z` comments; grype install
  replaced from `curl | sudo sh` with a checksummed download; GitHub token
  permissions scoped to least-privilege per workflow job.
- **Deny yanked crates** (#5): `yanked = "deny"` added to `[advisories]` in
  `deny.toml` so yanked transitive dependencies fail `just check`.
- Bumped `rustls-webpki` 0.103.12 → 0.103.13 (RUSTSEC-2026-0104: reachable
  panic parsing certificate revocation lists, pulled in transitively via
  `.ckeletin/conform` → `ureq` → `rustls`).

## [0.1.0] - 2026-04-13

### Added
- Cargo workspace with 3-crate architecture (domain, infrastructure, cli)
- Compile-time architecture enforcement via crate boundaries
- Violation tests proving crate boundaries hold (trybuild compile-fail tests)
- Output system with standardized JSON envelope and human-readable mode
- Shadow logging — every output operation logged to audit stream
- Three-stream output: stdout (data), stderr (status), log file (audit)
- Layered configuration via figment (defaults → TOML → env vars)
- tracing-based logging with stderr and optional JSON file layers
- Ping example command demonstrating full 4-layer pipeline
- Integration tests via assert_cmd
- Pre-commit hooks via lefthook (fmt, clippy, conventional commits)
- License and advisory checking via cargo-deny
- CI pipeline via GitHub Actions (calls `just check` — SSOT with local)
- AGENTS.md universal agent guide
- CLAUDE.md provider-specific guide

[Unreleased]: https://github.com/peiman/ckeletin-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/peiman/ckeletin-rust/releases/tag/v0.1.0
