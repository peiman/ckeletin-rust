# ckeletin-rust task runner

# Single gateway — all checks (CKSPEC-ENF-001)
check: fmt-check clippy test deny
    @echo "All checks passed."

# Run tests
test:
    cargo nextest run --workspace 2>/dev/null || cargo test --workspace

# Run tests with coverage (CKSPEC-TEST-002: 85% minimum)
coverage:
    cargo llvm-cov --workspace --fail-under-lines 85

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Run clippy (strict)
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run cargo-deny checks (licenses, advisories)
deny:
    cargo deny check

# Build release binary
build:
    cargo build --release

# Run conformance check (CKSPEC-ENF-005/006/007)
conform *ARGS:
    cargo run -p ckeletin-conform -q -- {{ARGS}}
