# ckeletin-rust — Project Guide for AI Agents

## About This Project

**ckeletin-rust** is a Rust CLI scaffold implementing the [ckeletin spec](https://github.com/peiman/ckeletin) (spec version tracked in CONFORMANCE.md / `conformance-mapping.toml`, not hardcoded here). It enforces four-layer architecture at compile time through a Cargo workspace with separate crates.

Key characteristics:
- **Workspace with 3 crates:** `domain` (business logic), `infrastructure` (config, logging, output), `cli` (entry + commands)
- **Compile-time architecture enforcement:** Crate boundaries in Cargo.toml prevent reverse dependencies. Violation tests prove it (CKSPEC-ENF-006)
- **Three-stream output:** stdout (data), stderr (status), log file (audit)
- **JSON mode:** `--output json` flag for machine-readable output on every command
- **Shadow logging:** every output logged to an audit file (on by default, at `~/.config/<app>/logs/`; `--no-audit` or the `log_location` config to change)
- **Layered configuration:** defaults → TOML file → environment variables → CLI flags
- **TDD:** Tests first, always. 85% minimum coverage
- **Dependency injection over mocking** — writer injection pattern, no mock frameworks

## Architecture

```
crates/
├── domain/           # Business logic — serde ONLY, no framework deps
│   └── src/
│       ├── lib.rs
│       └── ping.rs   # Example: pure function, returns typed result
├── infrastructure/   # Shared services — NO domain or cli deps
│   └── src/
│       ├── lib.rs
│       ├── config.rs   # figment layered config
│       ├── logging.rs  # tracing: stderr + file layers
│       └── output.rs   # Envelope, human/JSON rendering, shadow log
└── cli/              # Entry + commands — depends on domain + infrastructure
    └── src/
        ├── main.rs     # Bootstrap only (~20 lines)
        ├── root.rs     # clap derive: Cli struct, Commands enum
        └── ping.rs     # Handler: calls domain, renders via infrastructure
```

**Dependency direction (compile-time enforced):**
- `domain` → serde only. Cannot import clap, figment, tracing, infrastructure.
- `infrastructure` → Cannot import domain or cli.
- `cli` → Imports both domain and infrastructure. Only crate with clap.

**Violation tests:** `crates/domain/tests/architecture_violations.rs` and `crates/infrastructure/tests/architecture_violations.rs` use `trybuild` to verify that violating a boundary produces a compile error.

## Commands

| Scenario | Command |
|----------|---------|
| Run all checks | `just check` |
| Run tests only | `just test` |
| Format code | `just fmt` |
| Check formatting | `just ckeletin-fmt-check` |
| Run clippy | `just ckeletin-clippy` |
| License/advisory check | `just ckeletin-deny` |
| Coverage (85% min) | `just coverage` |
| Build release binary | `just build` |
| Run single crate tests | `cargo test -p domain` |
| Run specific test | `cargo test -p ckeletin --lib output::tests::envelope_success` |

**`just check` is the single gateway.** It runs fmt, clippy, test, deny — the same checks in CI and locally (SSOT). Run it before every commit.

## Adding a New Command

1. **Domain logic** (`crates/domain/src/mycommand.rs`):
   - Pure function, returns a typed result struct
   - `#[derive(Serialize)]` + `impl Display` on the result
   - Unit tests in the same file
   - No framework imports — only `serde` and `std`

2. **CLI handler** (`crates/cli/src/mycommand.rs`):
   - Calls domain function, passes result to `Output::success()`
   - Takes `&Output` as parameter for format selection
   - For a "no-data-to-report" success path (e.g. "no recorded
     history yet", "no pending actions"), call `Output::message()`
     not `Output::success()` with a `&format!("...")` String. The
     helper produces a stable JSON shape (`data: {"message":
     "..."}`) that downstream consumers can rely on; passing a
     bare String to `success` wraps it as a raw string blob in the
     envelope's `data` slot. See `output.rs` for the contract.

3. **Wire into root** (`crates/cli/src/root.rs`):
   - Add variant to `Commands` enum
   - Add match arm in `run_inner()` in `main.rs`

4. **Integration test** (`crates/cli/tests/cli.rs`):
   - Test human mode and JSON mode output

5. **Commit atomically:** Test + implementation in one commit (CKSPEC-TEST-004)

> **Common Mistake: Discovery logic in infrastructure.**
> The natural instinct is to put system discovery (running external processes, querying
> system state) in infrastructure because it uses infrastructure tools like process
> runners. But if that discovery code returns domain types, it creates an
> infrastructure -> domain dependency, violating CKSPEC-ARCH-005. The correct pattern:
> infrastructure provides generic tools (e.g., `process::run_capture`), and the **CLI
> layer** uses those tools to run commands and construct domain types from the results.
> Infrastructure never imports domain.

> **Domain types without business logic is valid.**
> Sometimes a command's domain layer is just typed data structures with
> `#[derive(Serialize)]` + `impl Display` — no computation, no validation, just
> structured output types. That is fine. The "logic" is orchestration in the CLI layer:
> calling infrastructure tools, building domain types from results, and passing them to
> `Output`. Not every domain module needs algorithms; sometimes its value is giving the
> pipeline a typed contract instead of raw strings.

> **Consuming a framework primitive: the `version` command.**
> `ping` shows a command built on your OWN domain type. `version`
> (`crates/cli/src/version.rs`) shows the other case: a command built on a
> FRAMEWORK primitive — `ckeletin::build_info::BuildInfo`. The build identity is
> baked at compile time by `crates/cli/build.rs` (one atomic `git describe
> --dirty`, degrading to `unknown` on any git failure) and read explicitly via
> `option_env!` in `version::current()` — the `env!` wiring is deliberately not
> hidden behind a macro so you can see it. `--version` is wired to the same
> `BuildInfo::version_line()` formatter in `main::parse_args`. Keep, customize,
> or delete it like `ping`.

## Coding Conventions

- **Domain has zero framework deps.** If you need logging in domain, return data and let the CLI layer log it.
- **All output through `Output` struct.** Never `println!` or `eprintln!` in domain or infrastructure. The output system handles stream routing and shadow logging.
- **Domain types handed to `Output::success` must implement both `Serialize` and `Display`.** `Output::success<T: Serialize + Display>` renders via `Display` in human mode and serializes via `Serialize` in JSON mode. One value, two outputs — presentation lives on the type. Implementing only `Serialize` means the type doesn't compile into a `success()` call; implementing only `Display` means JSON mode silently renders a string blob. See `crates/cli/src/ping.rs` for a worked example.
- **No-data success paths use `Output::message()`, not `Output::success()` with a `&format!("...")` string.** The `message` helper (added in ckeletin 0.2.2) writes a human sentence in text mode and an envelope with `data: {"message": msg}` in JSON mode — a stable, structured shape instead of a raw string blob.
- **Error envelopes must identify the failing subcommand.** Capture the command name from `&cli.command` *before* moving `cli` into `run_inner`, thread it into `Output::error`. Use an exhaustive `match` (not a default arm) so new subcommands are a compile error until they declare their own name — no silent `"init"` fallback. See `crates/cli/src/main.rs::subcommand_name`.
- **Typed configuration.** Add fields to `Config` struct in `config.rs`. figment deserializes at startup — no runtime type assertions.
- **Error handling:** `thiserror` for typed errors, `Box<dyn Error>` at application boundary.
- **Conventional commits:** `feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `ci:`, `chore:`. Enforced by lefthook commit-msg hook.

## Testing

- **Unit tests:** `#[cfg(test)] mod tests` in each source file
- **Violation tests:** `trybuild` compile-fail tests in `crates/*/tests/`
- **Integration tests:** `assert_cmd` in `crates/cli/tests/cli.rs`
- **Coverage:** `just coverage` (85% minimum, CKSPEC-TEST-002)
- **No mock frameworks.** Use writer injection (pass `&mut dyn Write`) or simple test doubles.

## Patterns for data-driven plug points

When a CKSPEC-compliant CLI grows to support **multiple backends,
runtimes, or providers**, the common pattern is a set of `const`s —
one per plugin — all matching the same struct shape (binary name,
signal strings, templates, keywords). This is a powerful pattern
but it has two specific failure modes that earn their own discipline.

### Capture-before-declare

Constants representing external systems (e.g. TUI ready signals,
CLI flag names, API response markers) MUST be picked from captured
evidence of the real system — never from docs, memory, or a
related implementation. External reality drifts; pinned constants
picked from intuition drift silently. The symptom is the pipeline
mis-classifying state weeks after the constant landed, with green
tests the whole time because the tests were written against the
same incorrect values.

**Discipline:** for every new plug-point constant:

1. Launch the real external system under your wrapper.
2. Capture its output/state in every distinct mode
   (pre-ready, ready, post-invocation, completion, failure).
3. Pick constant values from strings that appear *only* in the
   state they identify. Avoid substrings of text that appears in
   adjacent states.
4. Pin the captures as literals in regression tests that assert
   the picked constants appear in the right state and not the
   wrong ones. When the external system changes, these tests fail
   loudly — not silently at runtime.
5. Commit cites the capture source (file path or transcript).

This discipline was earned the hard way — three separate incidents of
constants picked from intuition drifting silently against the real system,
each with green tests the whole time, before it was written down.

### Cross-plug-point alias tests

When two plug-point constants share a shape, it's easy for one to
accidentally pick a signal that's a substring of another's.
Example: if plugin A declares `ready_signal = "Ready"` and plugin
B declares `completion_signal = "Not ready for input"`, A's signal
false-matches B's pane content.

**Discipline:** add a zero-cost invariant test that, for every
pair of plug-points (A, B) where A ≠ B, asserts no signal in A is
a substring of any signal in B. The test iterates the plug-point
registry, so adding a new plug-point automatically gets guarded
without per-plugin test code.

The invariant to assert: for every pair of plug-points (A, B) with A ≠ B, no
signal string in A is a substring of any signal string in B — iterated over the
plug-point registry so a new plug-point is guarded automatically.

### When these patterns apply (and when they don't)

These patterns apply when the CLI has multiple pluggable backends
represented as data (constants or config). They don't apply when
the CLI has a single runtime, a single protocol, or pure
business logic. Add them when the second plug-point lands — not
speculatively in a single-plugin CLI.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `just check` fails on fmt | `just fmt` then retry |
| Clippy pedantic warning | Fix it or add targeted `#[allow]` with justification |
| Violation test fails after adding dependency | You probably added a framework dep to domain — remove it |
| `cargo deny check` fails (advisory) | See "Advisory DB floats by design" below |
| `just conform` fails after a spec bump | `just conform-refresh` to pull the new spec, review the diff, reconcile `conformance-mapping.toml` |
| Integration test can't find binary | `cargo build` first, or run via `cargo test -p cli` |

### Advisory DB floats by design

The Rust **toolchain is pinned** (`rust-toolchain.toml` + CI) for reproducible
builds and stable trybuild snapshots. The RustSec **advisory database that
`cargo deny` consults is deliberately NOT pinned** — it floats so new CVEs
against transitive deps surface immediately. So CI can go red with zero commits
here when a fresh advisory lands (a weekly scheduled job catches this even on an
idle repo). That is the security system working. Remediation order:

1. Read the `RUSTSEC-…` id in the cargo-deny output.
2. `cargo update` (or `cargo update -p <crate>`), re-run `just ckeletin-deny`. A
   stale `Cargo.lock` is the usual cause; a patched release clears it.
3. If still flagged with no fix available, add a time-boxed entry to
   `[advisories] ignore` in `deny.toml` with the id, a reason, and a revisit
   date. Remove it once a fix ships.

Do **not** "fix" this by pinning the advisory DB — that hides future CVEs.
