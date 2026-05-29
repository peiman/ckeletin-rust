# Ckeletin Spec v0.4.0 — Rust Conformance Report

**Implementation:** ckeletin-rust
**Spec version:** 0.4.0
**Report date:** 2026-05-29
**Total:** 35 requirements — 35 met

This report is reconciled with `conformance-mapping.toml` (the machine-readable
source of truth) and is validated by `just conform` (`.ckeletin/conform/`),
which runs in CI. When prose and mapping disagree, the mapping wins and this
file is corrected to match.

Per Principle 10, this is a conformance report from a second implementation —
a retrospective, not an audit. Cross-implementation feedback with ckeletin-go
continues to refine the spec.

> **Changed since the 2026-04-13 report (spec v0.3.0):** the conformance
> generator now exists, so CKSPEC-ENF-005/006/007 moved from *deferred* to
> *met*; coverage (CKSPEC-TEST-002) is now gated in CI; agent-facing command
> docs were corrected (CKSPEC-AGENT-004/005); CKSPEC-OUT-004 shadow logging
> was completed (rendered data is logged; audit on by default, `--no-audit`
> opts out) and is now *met*; and CKSPEC-ARCH-006/007 enforcement claims were
> corrected to match reality.

---

## Architecture (7/7 met)

| ID | Title | Status | Enforcement | Violation Test / Evidence |
|----|-------|--------|-------------|----------------|
| CKSPEC-ARCH-001 | Four-layer architecture | met | compile-time | Workspace structure; `crates/domain/tests/violations/domain_imports_infrastructure.rs` |
| CKSPEC-ARCH-002 | Directed dependencies | met | compile-time | `domain_imports_infrastructure.rs`, `infra_imports_domain.rs` |
| CKSPEC-ARCH-003 | CLI framework isolation | met | compile-time | `domain_imports_clap.rs`, `infra_imports_clap.rs` |
| CKSPEC-ARCH-004 | Business logic isolation | met | compile-time | `domain_imports_figment.rs`, `domain_imports_tracing.rs` |
| CKSPEC-ARCH-005 | Infrastructure independence | met | compile-time | `infra_imports_domain.rs` |
| CKSPEC-ARCH-006 | Entry point minimality | met | design | `crates/cli/src/main.rs` — bootstrap only (see note) |
| CKSPEC-ARCH-007 | Package location enforcement | met | design | Structural (see note) |

**Evidence:** `crates/domain/Cargo.toml` lists only `serde`. `crates/infrastructure/Cargo.toml` has no domain or cli dependency. Writing `use clap::Parser` in domain → compiler error E0432. Seven trybuild compile-fail tests (5 domain + 2 infra) verify the boundaries on every `cargo test`.

**ARCH-006 note (corrected):** `main.rs` is the bootstrap entry — argument parsing, config *loading*, logging init, and dispatch — **102 lines**, 100% line-covered. (The earlier report's "~20 lines" figure was wrong.) Feature logic lives in domain; the entry contains no business logic.

**ARCH-007 note (corrected):** file placement is enforced *structurally* by the Cargo workspace layout — a stray `.rs` file at the workspace root belongs to no crate and is not built — not by a dedicated file-placement linter. Enforcement level is therefore `design`, not `compile-time` as previously claimed.

---

## Enforcement (7/7 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-ENF-001 | Automated enforcement required | met | lefthook pre-commit (fmt, clippy), cargo-deny, CI via `just check` |
| CKSPEC-ENF-002 | Enforcement ladder | met | compile-time (architecture), linter (clippy), CI (coverage, conform, init-smoke) |
| CKSPEC-ENF-003 | Document enforcement gaps | met | This report + the audit table below document all gaps, including the OUT-004 partial and the feedback signals |
| CKSPEC-ENF-004 | Enforcement audit table | met | See audit table below; reconciled with `conformance-mapping.toml` |
| CKSPEC-ENF-005 | Conformance mapping completeness | met | `just conform` loads the spec requirement IDs and **exits non-zero on any unmapped requirement** before reporting (`.ckeletin/conform/src/main.rs`) |
| CKSPEC-ENF-006 | Violation tests for enforcement claims | met | 7 trybuild violation tests; `just conform` verifies each declared violation-test file exists and flags unproven above-honor claims |
| CKSPEC-ENF-007 | Automatic feedback signals | met | `just conform` emits feedback signals in its report summary (`feedback_signals`) |

**Generator (supersedes the prior "deferred"):** `just conform` runs the `ckeletin-conform` crate (`.ckeletin/conform/`). It loads the spec requirement set (live from the spec repo, with a committed `conformance/requirements.json` fallback so it works offline), fails on unmapped requirements (ENF-005), runs each mapped check, verifies declared violation-test files exist (ENF-006), and emits feedback signals (ENF-007). It is gated by the CI `conform` job.

**Documented gap (ENF-003):** 13 requirements declare an above-honor-system enforcement level backed by positive checks (e.g. `test -f AGENTS.md`, integration tests) rather than dedicated *violation* tests. `just conform` surfaces these as ENF-007 feedback signals. Adding violation tests — or honestly reclassifying to honor-system — is tracked future work; per ENF-006's guidance we prefer real violation tests over box-ticking `violation_evidence`.

---

## Testing (4/4 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-TEST-001 | Test-driven development | met | Honor system; git history shows test+impl atomicity |
| CKSPEC-TEST-002 | Minimum coverage threshold | met | `just coverage` enforces 85% (cargo-llvm-cov), **gated by the CI coverage job**. Workspace is ~99.8%; the build-time `.ckeletin/conform` generator is a documented exclusion |
| CKSPEC-TEST-003 | Dependency injection over mocking | met | Writer injection in `Output`; zero mock crates in `Cargo.lock` |
| CKSPEC-TEST-004 | Atomic test commits | met | Honor system; git history |

**Corrected:** the prior report deferred CI coverage gating ("CI gate planned"). It is now wired — coverage runs in CI and fails the build below 85%.

---

## Output (5/5 met)

| ID | Title | Status | Enforcement | Evidence |
|----|-------|--------|-------------|----------|
| CKSPEC-OUT-001 | Three-stream output separation | met | unit tests | stdout (data), stderr (status via tracing), file (audit) |
| CKSPEC-OUT-002 | Machine-readable output mode | met | integration tests | `--output json`; `crates/cli/tests/cli.rs` verifies the JSON envelope on stdout |
| CKSPEC-OUT-003 | Standardized output envelope | met | unit tests | `Envelope { status, command, data, error }`; `output::tests::envelope_*` |
| CKSPEC-OUT-004 | Shadow logging | met | unit + integration tests | Shadow-logs rendered data; audit on by default (`--no-audit` opts out) |
| CKSPEC-OUT-005 | Output isolation from business logic | met | compile-time | Domain crate has no `std::io` path; `domain_imports_infrastructure.rs` |

**OUT-004 (now met):** every `Output` method shadow-logs the rendered data — `success` logs it via `data = %data`, `message` logs the text, `error` logs the error message — so the audit log contains at least what the user saw, plus tracing metadata (timestamp, command, level). File audit logging is **on by default** (`Config.log_file_enabled` defaults to `true`), active in both human and JSON modes; users opt out with `--no-audit` (one run) or `log_file_enabled = false` (config). On first run the CLI prints a one-time stderr notice pointing at the log file and the off-switch. (This was first reported *partial* and then implemented, rather than left as a hedge.)

---

## Agent Readiness (5/5 met)

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| CKSPEC-AGENT-001 | Universal agent guide | met | `AGENTS.md` |
| CKSPEC-AGENT-002 | No provider-specific content in universal guide | met | `AGENTS.md` is provider-neutral |
| CKSPEC-AGENT-003 | Provider-specific guides follow provider guidance | met | `CLAUDE.md` references `AGENTS.md` |
| CKSPEC-AGENT-004 | Agent guide completeness | met | Covers purpose, architecture, commands, conventions, testing, troubleshooting — **with corrected commands** |
| CKSPEC-AGENT-005 | CLI as the agent interface | met | `--output json` machine-readable mode; no protocol layer required |

**Corrected (AGENT-004/005):** documented commands now match reality — `--output json` (not the non-existent `--json`), real package names (`cli`/`domain`/`infrastructure`/`ckeletin`, not `ckeletin-*`), and real `just` recipes (`ckeletin-fmt-check`/`ckeletin-clippy`/`ckeletin-deny`, plus a new `fmt`). An agent following the docs verbatim now succeeds.

---

## Changelog (7/7 met)

| ID | Title | Status |
|----|-------|--------|
| CKSPEC-CL-001 | CHANGELOG.md in repository root | met |
| CKSPEC-CL-002 | Keep a Changelog format | met |
| CKSPEC-CL-003 | ISO 8601 dates | met |
| CKSPEC-CL-004 | Semantic Versioning | met |
| CKSPEC-CL-005 | Unreleased section | met |
| CKSPEC-CL-006 | Human-curated, not auto-generated | met |
| CKSPEC-CL-007 | Version comparison links | met |

---

## Enforcement Audit Table (CKSPEC-ENF-004)

| Decision | Mechanism | Level | Status | Violation Test | Gap |
|----------|-----------|-------|--------|----------------|-----|
| Four-layer architecture | Cargo workspace boundaries | compile-time | Full | 7 trybuild tests | — |
| Directed dependencies | Cargo.toml dependency graph | compile-time | Full | trybuild tests | — |
| CLI framework isolation | domain/infra Cargo.toml exclude clap | compile-time | Full | `domain_imports_clap.rs` | — |
| Business logic isolation | domain Cargo.toml excludes infra deps | compile-time | Full | 4 trybuild tests | — |
| Infrastructure independence | infra Cargo.toml excludes domain/cli | compile-time | Full | 2 trybuild tests | — |
| Output isolation | domain has no std::io path | compile-time | Full | `domain_imports_infrastructure.rs` | — |
| Package location | workspace layout (structural) | design | Structural | — | No file-placement linter |
| Entry-point minimality | bootstrap-only `main.rs` | design | Structural | — | Not coverage-excluded (100% covered anyway) |
| Code formatting | cargo fmt + lefthook | pre-commit | Full | — | No violation test |
| Lint standards | clippy -D warnings | pre-commit | Full | — | No violation test |
| License + advisory scanning | cargo-deny | pre-commit + CI | Full | — | No violation test |
| Coverage threshold | cargo-llvm-cov 85% | CI | Full | — | conform generator excluded (documented) |
| Conformance completeness | `just conform` fail-on-unmapped | CI (script) | Full | — | — |
| Conformance violation proof | `just conform` checks test files | CI (script) | Full | — | 13 claims rely on checks, surfaced as feedback |
| Shadow logging | tracing events (data) + default-on audit | script | Full | output.rs + cli.rs audit tests | — |
| TDD / atomic commits / changelog curation | AGENTS.md + CLAUDE.md | honor system | — | N/A | Cannot automate intent |
| Conventional commits | lefthook commit-msg | pre-commit | Full | — | No violation test |
| Scaffold init flow | `init_smoke` test | CI (upstream-only) | Full | `init_smoke` | — |

---

## Cross-Implementation Observations (Principle 10)

1. **Compile-time enforcement of architecture is real in Rust.** Cargo workspace crate boundaries make the compiler the linter; Go needs go-arch-lint. Both satisfy the requirements; `enforcement_level` makes the difference visible.

2. **Conformance reporting rots faster than code.** This report drifted from the code (the generator existed but the prose said it didn't; the spec advanced from v0.3.0 to v0.4.0). The fix is structural: `conformance-mapping.toml` is the SSOT, `just conform` validates it, and CI gates it — so prose can no longer silently diverge.

3. **Honest partials beat false "met"s — then close them.** OUT-004 shadow logging was first reported `partial` (rather than claimed met-with-a-hedge), making the gap visible; it was then implemented properly — rendered data logged, audit on by default — and is now genuinely met. Truth-Seeking (Principle 1): surface the gap, don't bury it in a "when enabled" qualifier, then fix it.

4. **A scaffold's headline flow must be gated.** `just init` shipped broken (issue #1) because its guard test was `#[ignore]`d and never run in CI. The lesson for the spec: enforcement claims include the *scaffold's own* tooling, not just the generated project.
