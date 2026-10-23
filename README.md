# ckeletin-rust

Rust CLI scaffold implementing the [ckeletin specification](https://github.com/peiman/ckeletin). AI-first CLI framework with compile-time architecture enforcement.

## Architecture

```
.ckeletin/          vendored framework — `just ckeletin-update` replaces it and stops if you changed it
├── crate/src/      config, logging, output, catalog, build_info, process
└── conform/        conformance generator binary

crates/
├── domain/         serde only — business logic, no framework deps
├── infrastructure/ re-export shim — exposes .ckeletin/crate to cli
└── cli/            clap + domain + infrastructure — entry point
    └── src/
        ├── main.rs     bootstrap only: parse, config, logging init, dispatch, error rendering
        ├── root.rs     Cli struct, Commands enum
        ├── ping.rs     example command
        ├── version.rs  build identity command
        └── catalog.rs  machine-readable command catalog (CKSPEC-AGENT-006)
```

Directed dependencies enforced by Cargo.toml at compile time. If domain code imports clap → **compile error**. Not a lint. Not a convention. The compiler refuses.

## Quick Start

```bash
git clone https://github.com/peiman/ckeletin-rust
cd ckeletin-rust
just check    # fmt + clippy + test + deny + health

# Template workflow: initialize a new derived project
just init my-app
```

> **Already-initialized guard:** `just init` detects if the project has already
> been initialized (name in `Cargo.toml` no longer matches the scaffold slug) and
> exits with an explanation. Pass `true` as the second positional argument to override: `just init my-app true`.

## Updating the framework

`just ckeletin-update` replaces `.ckeletin/` and leaves the rest of the project in place.

Framework 0.2.30 wrote an update-check stamp at `.ckeletin/cache/`. That update stops and reports uncommitted changes. The directory is not one of your changes. Delete it and run the update again:

```bash
rm -rf .ckeletin/cache
just ckeletin-update
```

From framework 0.2.31 on, the update ignores that directory. `just check` prints `.ckeletin/UPDATE_NOTE` from upstream when a newer framework version is available. That note is the step to take before you update.

The 0.2.30 update also re-resolves the whole `Cargo.lock`. That command cannot print the fix. After it finishes, restore the lock and refresh only the framework crate:

```bash
git checkout HEAD -- Cargo.lock
cargo update -p ckeletin
```

```bash
# Run the scaffold commands
cargo run -p cli -- ping
cargo run -p cli -- --output json ping

# Machine-readable command catalog (CKSPEC-AGENT-006)
cargo run -p cli -- catalog
cargo run -p cli -- --output json catalog
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
