# ckeletin Framework Changelog

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
