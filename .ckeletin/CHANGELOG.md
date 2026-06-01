# ckeletin Framework Changelog

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
