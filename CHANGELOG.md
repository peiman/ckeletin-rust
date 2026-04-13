# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
