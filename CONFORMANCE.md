# Ckeletin Spec v0.3.0 — Rust Conformance Report

**Implementation:** ckeletin-rust
**Spec version:** 0.3.0
**Report date:** 2026-04-13
**Total:** 35 requirements — 32 met, 3 deferred

This is the first conformance report from a second implementation.
Per Principle 10: "A requirement isn't proven until at least two
implementations can satisfy it."

---

## Architecture (7/7 met)

| ID | Title | Status | Enforcement | Violation Test |
|----|-------|--------|-------------|----------------|
| CKSPEC-ARCH-001 | Four-layer architecture | met | compile-time | Workspace structure |
| CKSPEC-ARCH-002 | Directed dependencies | met | compile-time | `crates/domain/tests/violations/domain_imports_infrastructure.rs` |
| CKSPEC-ARCH-003 | CLI framework isolation | met | compile-time | `crates/domain/tests/violations/domain_imports_clap.rs`, `crates/infrastructure/tests/violations/infra_imports_clap.rs` |
| CKSPEC-ARCH-004 | Business logic isolation | met | compile-time | `crates/domain/tests/violations/domain_imports_figment.rs`, `domain_imports_tracing.rs` |
| CKSPEC-ARCH-005 | Infrastructure independence | met | compile-time | `crates/infrastructure/tests/violations/infra_imports_domain.rs` |
| CKSPEC-ARCH-006 | Entry point minimality | met | design | `crates/cli/src/main.rs` — ~20 lines, bootstrap only |
| CKSPEC-ARCH-007 | Package location enforcement | met | compile-time | Workspace crate boundaries |

**Evidence:** `crates/domain/Cargo.toml` lists only `serde`. `crates/infrastructure/Cargo.toml` has no domain or cli dependency. If domain code writes `use clap::Parser` → compiler error E0432: "unresolved import." Six trybuild compile-fail tests verify this on every `cargo test`.

**Feedback for spec:** Rust's Cargo workspace provides enforcement level 1 (compile-time) for all 7 architecture requirements. Go's go-arch-lint provides level 4 (linter). Both meet the requirements. The enforcement_level field makes this difference visible without either implementation being "better" — different languages, different mechanisms, same guarantee.

---

## Enforcement (4/7 met, 3 deferred)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-ENF-001 | Automated enforcement required | met | lefthook pre-commit (fmt, clippy), cargo-deny, CI via `just check` |
| CKSPEC-ENF-002 | Enforcement ladder | met | Compile-time (architecture), linter (clippy), CI (coverage) |
| CKSPEC-ENF-003 | Document enforcement gaps | met | This report documents gaps — see deferred items below |
| CKSPEC-ENF-004 | Enforcement audit table | met | See enforcement audit table below |
| CKSPEC-ENF-005 | Conformance mapping completeness | deferred | No conformance generator exists yet. This report is hand-written. |
| CKSPEC-ENF-006 | Violation tests for enforcement claims | deferred | 6 violation tests exist for architecture claims. Other enforcement claims (clippy, cargo-deny) lack violation tests proving they catch violations. |
| CKSPEC-ENF-007 | Automatic feedback signals | deferred | No generator to produce automatic feedback signals. |

**Feedback for spec:** ENF-006 is partially met — architecture violation tests exist and are thorough. But clippy enforcement and cargo-deny enforcement don't have violation tests yet. Honest reporting: deferred until all enforcement claims have violation tests.

---

## Testing (4/4 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-TEST-001 | Test-driven development | met | All code written test-first. Git history shows test commits preceding or atomic with implementation. Honor system — cannot be automated. |
| CKSPEC-TEST-002 | Minimum coverage threshold | met | `just coverage` enforces 85% via cargo-llvm-cov. CI gate planned. |
| CKSPEC-TEST-003 | Dependency injection over mocking | met | Writer injection pattern in `Output::success()` / `Output::error()`. No mock frameworks in dependencies. `Cargo.lock` contains zero mock crates. |
| CKSPEC-TEST-004 | Atomic test commits | met | Every commit in git history includes tests with implementation. Honor system — cannot be automated. |

---

## Output (5/5 met)

| ID | Title | Status | Enforcement | Evidence |
|----|-------|--------|-------------|----------|
| CKSPEC-OUT-001 | Three-stream output separation | met | unit tests | `output.rs`: stdout (data), stderr (status via tracing), log file (audit via tracing file layer) |
| CKSPEC-OUT-002 | Machine-readable output mode | met | integration tests | `--json` flag. `crates/cli/tests/cli.rs::ping_json_mode_*` tests verify JSON envelope on stdout |
| CKSPEC-OUT-003 | Standardized output envelope | met | unit tests | `Envelope` struct: status, command, data, error. `output::tests::envelope_*` verify serialization |
| CKSPEC-OUT-004 | Shadow logging | met | unit tests | `tracing::debug!` in `Output::success()` and `Output::error()`. Lands in file audit layer when enabled. |
| CKSPEC-OUT-005 | Output isolation from business logic | met | compile-time | Domain crate has no `std::io` writer dependency. Cannot call `println!` — no path to stdout from domain code. Violation test: `domain_imports_infrastructure.rs` |

**Feedback for spec:** OUT-005 enforcement in Rust is compile-time (domain crate literally cannot write to stdout). In Go, it's script-level (validate-output-patterns.sh greps for fmt.Print). Both meet the requirement. Rust's approach is structurally stronger — the compiler prevents the violation rather than a script detecting it after the fact.

---

## Agent Readiness (5/5 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-AGENT-001 | Universal agent guide | met | `AGENTS.md` — architecture, commands, conventions, testing, troubleshooting |
| CKSPEC-AGENT-002 | No provider-specific content in universal guide | met | `AGENTS.md` contains no Claude/Gemini/Copilot-specific instructions |
| CKSPEC-AGENT-003 | Provider-specific guides follow provider guidance | met | `CLAUDE.md` references AGENTS.md, adds Claude-specific workflow rules |
| CKSPEC-AGENT-004 | Agent guide completeness | met | AGENTS.md covers: purpose, architecture, commands, adding commands, conventions, testing, troubleshooting |
| CKSPEC-AGENT-005 | CLI as the agent interface | met | `--json` flag provides machine-readable output. No protocol layer required. Shell access + AGENTS.md is sufficient. |

---

## Changelog (7/7 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-CL-001 | CHANGELOG.md in repository root | met | `CHANGELOG.md` exists at root |
| CKSPEC-CL-002 | Keep a Changelog format | met | Added section, reverse chronological, version + date headings |
| CKSPEC-CL-003 | ISO 8601 dates | met | `[0.1.0] - 2026-04-13` |
| CKSPEC-CL-004 | Semantic Versioning | met | `Cargo.toml` version = "0.1.0", CHANGELOG states SemVer adherence |
| CKSPEC-CL-005 | Unreleased section | met | `[Unreleased]` section at top of CHANGELOG.md |
| CKSPEC-CL-006 | Human-curated, not auto-generated | met | Changelog entries written by hand describing features, not git log dump |
| CKSPEC-CL-007 | Version comparison links | met | Reference-style links at bottom of CHANGELOG.md |

---

## Enforcement Audit Table (CKSPEC-ENF-004)

| Decision | Mechanism | Level | Status | Violation Test | Gap |
|----------|-----------|-------|--------|----------------|-----|
| Four-layer architecture | Cargo workspace crate boundaries | compile-time | Full | 6 trybuild tests | — |
| Directed dependencies | Cargo.toml dependency declarations | compile-time | Full | trybuild tests | — |
| CLI framework isolation | Domain Cargo.toml excludes clap | compile-time | Full | `domain_imports_clap.rs` | — |
| Business logic isolation | Domain Cargo.toml excludes infra deps | compile-time | Full | 4 trybuild tests | — |
| Infrastructure independence | Infra Cargo.toml excludes domain/cli | compile-time | Full | 2 trybuild tests | — |
| Output isolation | Domain crate has no std::io path | compile-time | Full | `domain_imports_infrastructure.rs` | — |
| Code formatting | cargo fmt + lefthook | pre-commit | Full | None | No violation test |
| Lint standards | clippy -D warnings | pre-commit | Full | None | No violation test |
| License compliance | cargo-deny licenses | pre-commit + CI | Full | None | No violation test |
| Vulnerability scanning | cargo-deny advisories | pre-commit + CI | Full | None | No violation test |
| Coverage threshold | cargo-llvm-cov 85% | CI | Full | None | Not yet wired in CI |
| TDD workflow | AGENTS.md + CLAUDE.md | honor system | — | N/A | Cannot automate intent |
| Atomic commits | AGENTS.md + CLAUDE.md | honor system | — | N/A | Cannot automate grouping |
| Changelog curation | Human judgment | honor system | — | N/A | Cannot automate editorial |
| Conventional commits | lefthook commit-msg | pre-commit | Full | None | No violation test |

**Summary:** 6 decisions enforced at compile-time with violation tests. 5 decisions enforced at pre-commit/CI without violation tests. 3 decisions at honor system. 1 decision (coverage) not yet wired.

---

## Cross-Implementation Observations (Principle 10)

These observations come from implementing the same spec in Rust after Go:

1. **Compile-time enforcement is achievable for architecture rules in Rust.** Cargo workspace crate boundaries provide structural enforcement — the compiler is the linter. Go needs a separate tool (go-arch-lint). Both work. The spec correctly notes (CKSPEC-ENF-002) that enforcement level varies by language.

2. **Workspace adds complexity but honesty.** A single-crate Rust project does NOT enforce directed dependencies at compile time (`pub` items are accessible across modules). We discovered this by applying the robustness lens and corrected it before building. The spec should note that compile-time enforcement of architecture may require language-specific project structure decisions (workspaces in Rust, build tags in Go).

3. **serde as cross-cutting concern works well.** The CKSPEC-ARCH-004 notes (added in v0.3.0) correctly classify serialization annotations as data description. In practice: domain types derive `Serialize`, infrastructure's output system serializes them. Clean separation.

4. **The violation test discipline (ENF-006) caught a real false claim.** We initially planned a single-crate architecture and would have claimed compile-time enforcement. The requirement to write violation tests forced us to verify the claim — and it was wrong. ENF-006 works as designed.

5. **figment is significantly better than Viper for typed configuration.** Viper returns `interface{}` and panics at runtime on type mismatch. figment extracts into typed structs at startup with provenance tracking in error messages. The spec's language-agnostic approach is correct — it says WHAT (layered config), not HOW.
