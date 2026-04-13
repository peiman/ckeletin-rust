# ckeletin-rust — Project Guide for AI Agents

## About This Project

**ckeletin-rust** is a Rust CLI scaffold implementing the [ckeletin spec](https://github.com/peiman/ckeletin) v0.3.0. It enforces four-layer architecture at compile time through a Cargo workspace with separate crates.

Key characteristics:
- **Workspace with 3 crates:** `domain` (business logic), `infrastructure` (config, logging, output), `cli` (entry + commands)
- **Compile-time architecture enforcement:** Crate boundaries in Cargo.toml prevent reverse dependencies. Violation tests prove it (CKSPEC-ENF-006)
- **Three-stream output:** stdout (data), stderr (status), log file (audit)
- **JSON mode:** `--json` flag for machine-readable output on every command
- **Shadow logging:** Every output operation logged to audit stream
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
| Check formatting | `just fmt-check` |
| Run clippy | `just clippy` |
| License/advisory check | `just deny` |
| Coverage (85% min) | `just coverage` |
| Build release binary | `just build` |
| Run single crate tests | `cargo test -p ckeletin-domain` |
| Run specific test | `cargo test -p ckeletin-infrastructure --lib output::tests::envelope_success` |

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

3. **Wire into root** (`crates/cli/src/root.rs`):
   - Add variant to `Commands` enum
   - Add match arm in `run_inner()` in `main.rs`

4. **Integration test** (`crates/cli/tests/cli.rs`):
   - Test human mode and JSON mode output

5. **Commit atomically:** Test + implementation in one commit (CKSPEC-TEST-004)

## Coding Conventions

- **Domain has zero framework deps.** If you need logging in domain, return data and let the CLI layer log it.
- **All output through `Output` struct.** Never `println!` or `eprintln!` in domain or infrastructure. The output system handles stream routing and shadow logging.
- **Typed configuration.** Add fields to `Config` struct in `config.rs`. figment deserializes at startup — no runtime type assertions.
- **Error handling:** `thiserror` for typed errors, `Box<dyn Error>` at application boundary.
- **Conventional commits:** `feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `ci:`, `chore:`. Enforced by lefthook commit-msg hook.

## Testing

- **Unit tests:** `#[cfg(test)] mod tests` in each source file
- **Violation tests:** `trybuild` compile-fail tests in `crates/*/tests/`
- **Integration tests:** `assert_cmd` in `crates/cli/tests/cli.rs`
- **Coverage:** `just coverage` (85% minimum, CKSPEC-TEST-002)
- **No mock frameworks.** Use writer injection (pass `&mut dyn Write`) or simple test doubles.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `just check` fails on fmt | `just fmt` then retry |
| Clippy pedantic warning | Fix it or add targeted `#[allow]` with justification |
| Violation test fails after adding dependency | You probably added a framework dep to domain — remove it |
| `cargo deny check` fails | Check `deny.toml` allowlist or update advisory database |
| Integration test can't find binary | `cargo build` first, or run via `cargo test -p ckeletin-cli` |
