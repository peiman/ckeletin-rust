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

Implements the [ckeletin spec](https://github.com/peiman/ckeletin) across six
domains — Architecture, Enforcement, Testing, Output, Agent Readiness, and
Changelog. Conformance is validated in CI by `just conform` against
`conformance-mapping.toml`.

See **[CONFORMANCE.md](CONFORMANCE.md)** for the exact spec version, requirement
count, and per-requirement evidence — kept there as the single source of truth
rather than duplicated here, where it would drift.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
