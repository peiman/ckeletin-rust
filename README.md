# ckeletin-rust

Rust CLI scaffold implementing the [ckeletin specification](https://github.com/peiman/ckeletin). AI-first CLI framework with compile-time architecture enforcement.

## Architecture

```
crates/
├── domain/           serde only — business logic, no framework deps
├── infrastructure/   config, logging, output — no domain/cli deps
└── cli/              clap + domain + infrastructure — entry point
```

Directed dependencies enforced by Cargo.toml at compile time. If domain code imports clap → **compile error**. Not a lint. Not a convention. The compiler refuses.

## Quick Start

```bash
git clone https://github.com/peiman/ckeletin-rust
cd ckeletin-rust
just check    # fmt + clippy + test + deny
cargo run -p cli -- ping
cargo run -p cli -- --output json ping
```

## Spec Conformance

Implements [ckeletin spec](https://github.com/peiman/ckeletin) v0.4.0 — 35 requirements across 6 domains (all met):

| Domain | Requirements | Status |
|--------|-------------|--------|
| Architecture | 7 | All met (compile-time enforcement) |
| Enforcement | 7 | All met (`just conform` generator, CI-gated) |
| Testing | 4 | All met (85% coverage gated in CI) |
| Output | 5 | All met |
| Agent Readiness | 5 | All met |
| Changelog | 7 | All met |

See [CONFORMANCE.md](CONFORMANCE.md) for evidence per requirement.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
