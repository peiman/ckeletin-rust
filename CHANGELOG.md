# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `Output::message(command, msg, writer)` — framework helper for
  no-structured-data success paths (e.g. "no recorded history
  yet"). Human mode writes the sentence + newline; JSON mode wraps
  in `data: {"message": msg}` so downstream JSON consumers always
  find a structured object in the `data` slot (never a raw string
  blob). Framework version bumped to 0.2.2. Discovered during the
  workhorse replay build when `learn::execute` and `replay::execute`
  both ended up passing `&format!("...")` to `Output::success` for
  their empty-history paths, producing JSON envelopes with the
  message as a string-in-data slot. Regression tests in
  `.ckeletin/crate/src/output.rs`:
  `human_message_writes_msg_with_newline`,
  `json_message_wraps_text_in_structured_data_field`,
  `json_message_output_is_valid_parseable_json`,
  `json_message_envelope_carries_the_subcommand_name`.
- `--no-audit` global flag and on-by-default audit logging
  (CKSPEC-OUT-004). The audit log (`logs/app.log`) is now written by
  default and shadow-logs the *rendered data* (not just the command
  name); `Output::success`/`message`/`error` all emit it. A one-time
  first-run notice on stderr (human mode) points at the log file and the
  off-switches: `--no-audit` for one run, or `log_file_enabled = false`
  in config. `logs/` is gitignored.

### Changed
- Conformance reporting brought in line with the code and ckeletin
  spec v0.4.0. `CONFORMANCE.md` is now reconciled with
  `conformance-mapping.toml` (the machine source of truth) and
  validated by `just conform`, which runs in CI:
  CKSPEC-ENF-005/006/007 move from "deferred" to "met" (the
  `just conform` generator exists and is CI-gated); CKSPEC-TEST-002
  coverage is gated by a CI job (85%, with the build-time conformance
  generator a documented exclusion; the rest of the workspace is
  ~99.8%); CKSPEC-OUT-004 shadow logging was completed (rendered data is
  shadow-logged; audit logging on by default, `--no-audit` opts out) and
  is met; and the CKSPEC-ARCH-006/007 enforcement claims were corrected
  (entry point is 102 lines not "~20"; package location is structural,
  not compile-time). The vendored spec snapshot
  (`conformance/requirements.json`) is now committed so `just conform`
  works offline. Net: 35 requirements — all 35 met (previously reported
  32 met / 3 deferred against a stale v0.3.0 snapshot).

### Fixed
- Error envelope in JSON mode now identifies the failing subcommand
  via its `command` field (CKSPEC-OUT-003). Prior versions hardcoded
  `"command": "init"` in the error path of `crates/cli/src/main.rs`,
  so every subcommand that failed produced an envelope claiming the
  init command, regardless of which one was running. A
  `subcommand_name(&Commands)` helper now maps the parsed `Commands`
  variant to its CLI-visible name, captured before `cli` moves into
  `run_inner`. The match in `subcommand_name` is exhaustive — adding
  a new subcommand is a compile error until the name is assigned,
  eliminating the silent-fallback class of bug entirely. Regression
  test: `json_mode_error_envelope_identifies_failing_subcommand` in
  `crates/cli/tests/cli.rs`.
- `just init <name>` now produces a project that compiles and is a
  committed git repository. The init script previously stripped the
  `ping` demo command — the only subcommand — leaving an empty
  `Commands` enum that `crates/cli/src/main.rs` could not match
  exhaustively (`error[E0004]`), so `cargo check` failed at init's
  verify step and the script exited before `git init`. It also used a
  case-insensitive line delete (`sed '/ping/Id'`) that mangled
  `crates/cli/tests/cli.rs` into invalid Rust. init now keeps `ping`
  as the renamed worked example (matching the ckeletin-go scaffold),
  verifies with `cargo check --all-targets` so a broken test file can
  no longer slip through, and the previously `#[ignore]`d `init_smoke`
  test is gated in CI (upstream-only). Fixes #1.

### Security
- Bumped `rustls-webpki` 0.103.12 → 0.103.13 (RUSTSEC-2026-0104:
  reachable panic parsing certificate revocation lists), pulled in
  transitively via `.ckeletin/conform` → `ureq` → `rustls`. The
  committed `Cargo.lock` previously failed `cargo deny check` (and
  therefore `just check`) on a clean clone.

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
