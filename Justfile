# Project task runner
# Framework tasks imported from .ckeletin/Justfile

import '.ckeletin/Justfile'

binary_name := "ckeletin-rust"

# Single gateway — all checks (CKSPEC-ENF-001)
check: ckeletin-check test ckeletin-health
    @echo "All checks passed."

# Run tests
test:
    cargo nextest run --workspace 2>/dev/null || cargo test --workspace

# Run tests with coverage (CKSPEC-TEST-002: 85% minimum)
coverage:
    cargo llvm-cov --workspace --fail-under-lines 85

# Build release binary
build:
    cargo build --release

# Initialize scaffold for a new project (run once after clone)
init name:
    .ckeletin/scripts/init.sh {{name}}
