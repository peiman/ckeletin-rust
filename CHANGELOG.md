# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
