<div align="center">

![ckeletin-rust](logo/ckeletin-rust-banner.png)

**AI-first Rust development framework. Ship production CLIs that the compiler keeps honest.**

<!-- Row 1: Build Quality -->
[![Build Status](https://github.com/peiman/ckeletin-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/peiman/ckeletin-rust/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.96.0-orange)](rust-toolchain.toml)

<!-- Row 2: Project Metadata -->
[![Version](https://img.shields.io/github/v/release/peiman/ckeletin-rust)](https://github.com/peiman/ckeletin-rust/releases)
[![Last Commit](https://img.shields.io/github/last-commit/peiman/ckeletin-rust)](https://github.com/peiman/ckeletin-rust/commits/main)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](AGENTS.md)

<!-- Row 3: Community -->
[![GitHub stars](https://img.shields.io/github/stars/peiman/ckeletin-rust?style=social)](https://github.com/peiman/ckeletin-rust/stargazers)
[![Spec](https://img.shields.io/badge/spec-ckeletin-orange)](https://github.com/peiman/ckeletin)

</div>

---

## TL;DR

ckeletin-rust is an AI-first Rust CLI framework. The architecture is a Cargo workspace, so a layer that imports the wrong crate does not compile. You get production-ready infrastructure with an updatable framework layer, and your application crates stay yours.

- **The compiler is the architecture check** — domain may depend on serde only. Infrastructure re-exports the framework. Only the CLI crate has clap. A wrong import is a compile error, proved by trybuild
- **Updatable framework** — `.ckeletin/` updates through `just ckeletin-update`. Your crates are not replaced. A local edit under `.ckeletin/` stops the update
- **Agents read one guide** — `AGENTS.md` is the project guide. `CLAUDE.md` imports it and adds only Claude Code behavior. `just check` is the same gate in a hook and in CI
- **Ship with confidence** — 85% line coverage, `cargo deny` on the license allow list, and `just check` before every commit
- **One command setup** — `just init my-app` renames the scaffold. Start from a clone

```bash
git clone https://github.com/peiman/ckeletin-rust.git && cd ckeletin-rust
just init my-app
just build && ./target/release/my-app ping
```

---

## Who Is This For?

**You use AI coding agents and need them to produce correct code.**
`AGENTS.md`, hooks, and crate boundaries mean an agent cannot put clap in the domain crate and have the build succeed. That is what agent-ready looks like in Rust.

**You want to make your own codebase agent-ready.**
Study the pattern: `AGENTS.md`, then behavioral rules, then hooks, then a gate the compiler and `just check` both enforce.

**Your boss needs a CLI by next sprint. You've never built one.**
Clone, `just init`, and the workspace, logging, JSON output, and update mechanism are already there.

**You're a senior dev who's tired of rebuilding the same scaffolding.**
The framework updates on its own schedule. Your `crates/` stay. The next `just ckeletin-update` brings the framework forward and leaves your commands alone.

---

## Agent-Ready Architecture

Most scaffolds produce code that AI agents can write *in* but not write *well in*. Agents guess at module boundaries and drift. ckeletin-rust solves this with **enforcement by the compiler** — the dependency direction is Cargo.toml, and violation tests fail the build when that direction is broken.

### The AI Configuration Stack

```
AGENTS.md          → Universal project guide (any AI assistant)
CLAUDE.md          → Claude Code-specific behavioral rules
lefthook.yml       → fmt, clippy, and gitleaks on commit
just check         → Single gateway: fmt, clippy, test, deny, health
```

**`AGENTS.md`** is the project guide every agent reads: commands, where code goes, hard rules, and troubleshooting. Provider files do not restate it.

**`CLAUDE.md`** imports `AGENTS.md` and adds only Claude Code behavior.

**Hooks and enforcement** close the loop. Pre-commit runs format, clippy, and a secret scan. `just check` runs the same quality gates regardless of who wrote the code. A domain import of clap, tracing, figment, or the framework crate does not compile.

### Why This Matters

- **Determinism**: `just check` always runs the same recipes. Agents do not invent a different `cargo test` line
- **Architectural memory**: the spec and `AGENTS.md` say why a boundary exists, so an agent does not "simplify" it away
- **Automated enforcement**: trybuild snapshots fail when a forbidden import compiles. There is no honor system
- **Framework evolution**: `just ckeletin-update` replaces `.ckeletin/` and does not edit `crates/`

### Using With AI Agents

**Claude Code**: Reads `CLAUDE.md`, which imports `AGENTS.md`.

**Cursor / Copilot / Codex**: Read `AGENTS.md` directly.

**The pattern is reusable.** The guide, then hooks, then a machine check works in any codebase. ckeletin-rust is the Rust reference for the [ckeletin spec](https://github.com/peiman/ckeletin).

---

## What You Get

ckeletin-rust is both a **scaffold** (clone, `just init`, ship) and a **framework** (`.ckeletin/` keeps updating):

```
my-app/
├── .ckeletin/              # FRAMEWORK — `just ckeletin-update` replaces it
│   ├── crate/src/          # config, logging, output, catalog, build info
│   ├── conform/            # conformance generator (upstream only)
│   ├── scripts/            # init and update scripts
│   └── Justfile            # ckeletin-* recipes
│
├── crates/
│   ├── domain/             # YOUR logic — serde only
│   ├── infrastructure/     # re-exports the framework crate
│   └── cli/                # YOUR commands — clap, domain, infrastructure
├── ckeletin-project.toml   # YOUR defaults and layer allowlists
├── Justfile                # YOUR aliases: check, test, build, init
└── deny.toml               # YOUR license and advisory policy
```

**The scaffold** gets you started: clone, `just init my-app`, add a command under `crates/`, ship.

**The framework** keeps working: config, tracing, the JSON envelope, and the update guard, replaced independently of your crates.

`ckeletin-project.toml` lives outside `.ckeletin/` and survives an update. That file is where project defaults and the layer allowlists belong.

---

## Quick Start

1. **Clone:**
   ```bash
   git clone https://github.com/peiman/ckeletin-rust.git
   cd ckeletin-rust
   ```
   Rust comes from `rust-toolchain.toml` (1.96.0). Install [just](https://github.com/casey/just).

2. **Initialize with your project name:**
   ```bash
   just init my-app
   ```
   The name is lowercase letters, digits, and hyphens, at most 15 characters. `just init` rewrites the binary name, the crate paths, and the CI temp prefix. A name that is already initialized is refused. `just init my-app true` overrides that guard.

3. **Build and run:**
   ```bash
   just build
   ./target/release/my-app ping
   ```

On this repository, before you initialize it:

```bash
just check
cargo run -p cli -- ping
cargo run -p cli -- --output json catalog
```

---

## Architecture

Four crates, one direction, checked by Cargo:

- **Domain** — business types. serde only. A framework import does not compile
- **Infrastructure** — re-exports `.ckeletin/crate`. It does not import domain or clap
- **CLI** — clap, domain, and infrastructure. Commands stay thin: parse, call domain, render through `Output`
- **Framework** — `.ckeletin/crate`: config, logging, output, catalog, build identity

Other decisions that travel with the scaffold:

- **Dependency injection over mocking** — pass a writer. No mock framework
- **Three streams** — stdout for data, stderr for status, and an audit log file
- **JSON mode** — `--output json` on every command. The default envelope is the total shape from the spec
- **License allow list** — `cargo deny` allows MIT, Apache-2.0, BSD, and ISC. A license that is not on the list fails `just check`
- **Coverage** — `just coverage` fails under 85% line coverage

Conformance against the spec is recorded in **[CONFORMANCE.md](CONFORMANCE.md)**. That file is the count and the evidence. This README does not repeat them.

---

## Key Features

- **Compile-time layers**: the workspace graph is the architecture
- **Structured logging**: tracing, with an audit file on by default
- **Typed configuration**: figment. Flags, then environment, then the config file, then `[defaults]` in `ckeletin-project.toml`
- **Command catalog**: `catalog` walks the live clap tree, so the catalog cannot drift from the commands
- **Task runner**: one Justfile for check, test, fmt, build, and the framework recipes
- **Update guard**: uncommitted or locally committed framework edits stop `just ckeletin-update`
- **High test coverage**: 85% enforced in CI

---

## Getting Started

### Prerequisites

- **Rust**: the channel in `rust-toolchain.toml` (1.96.0). rustup installs it
- **just**: [casey/just](https://github.com/casey/just)
- **Git**: for version control and the update fetch

### Build from Source

```bash
git clone https://github.com/peiman/ckeletin-rust.git
cd ckeletin-rust
just build
```

### Using the Scaffold

```bash
just init my-app
just check
just build
./target/release/my-app --version
```

`just init` starts a new git history, commits it, and tags `v0.0.0`. The commit uses `ckeletin-init` unless `GIT_AUTHOR_NAME` is already set. It does not write a name into the new repository.

---

## Configuration

Figment loads configuration in this order, highest first: CLI flags, environment variables prefixed with `CKELETIN_`, the config file, `[defaults]` in `ckeletin-project.toml`, then framework defaults. `just init` renames the `CKELETIN_` prefix to your project.

```toml
# ckeletin-project.toml
[defaults]
log_level = "debug"
```

```bash
export CKELETIN_LOG_LEVEL=debug
cargo run -p cli -- --output json ping
```

Project defaults never override a flag, an environment variable, or a config file.

---

## Commands

Built-in commands: `ping` (the worked example), `version` (build identity), and `catalog` (the machine-readable command tree). Global flags include `--output text|json`, `--config`, `--verbose`, and `--no-audit`.

```bash
cargo run -p cli -- ping
cargo run -p cli -- version
cargo run -p cli -- --output json catalog
just ckeletin-doctor
```

A new command touches the domain type, the CLI handler, the clap enum, and an integration test. `AGENTS.md` lists the files. Skipping one is a compile error.

---

## Development Workflow

Recipes in the root `Justfile` are the same ones CI runs:

```bash
just check     # fmt, clippy, test, deny, health (before every commit)
just test      # cargo test --workspace
just fmt       # cargo fmt --all
just build     # cargo build --release
just coverage  # 85% line coverage
just ckeletin-doctor
```

CI also runs conformance, a secret scan, and the scaffold init smoke test. `just check` does not include the init smoke test. That test is slow and upstream-only.

---

## Framework Updates

Your crates and the framework layer are independent:

```bash
just ckeletin-update-dry-run
just ckeletin-update-check-compatibility
just ckeletin-update
```

| Framework (`.ckeletin/`) | Project (yours) |
|--------------------------|-----------------|
| `crate/` — config, logging, output | `crates/domain` — your types |
| `Justfile` — `ckeletin-*` recipes | `Justfile` — check, test, build, init |
| `scripts/` — init and update | `crates/cli` — your commands |
| `VERSION` — framework version | `ckeletin-project.toml` — defaults and allowlists |

An update replaces `.ckeletin/` only. It stops when that tree has uncommitted changes, or a committed difference from the recorded version. `just ckeletin-update discard_local=true` discards a committed difference and says so. It does not discard uncommitted work.

The framework version and the application release tag are different sequences. `just ckeletin-update v0.3.0` follows an application tag that already exists. It does not look up `.ckeletin/VERSION`.

---

## Contributing

1. Fork and create a feature branch
2. Make the change and run `just check`
3. Commit with a [Conventional Commits](https://www.conventionalcommits.org/) subject
4. Open a PR against `main`

`AGENTS.md` is the guide for where the change belongs.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
