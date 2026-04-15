#!/usr/bin/env bash
set -euo pipefail

NAME="${1:?Usage: just init name=<project-name>}"

# Validate name (lowercase, hyphens, no spaces)
if [[ ! "$NAME" =~ ^[a-z][a-z0-9-]*$ ]]; then
    echo "Error: name must be lowercase alphanumeric with hyphens (e.g., 'my-project')"
    exit 1
fi

# Pre-flight: warn about uncommitted changes
if [ -d .git ] && ! git diff --quiet 2>/dev/null; then
    echo "Warning: uncommitted changes exist. Init resets git history — uncommitted work will be lost."
    read -p "Continue? (y/N) " confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Aborted."
        exit 0
    fi
fi

echo "Initializing scaffold as: $NAME"

# 1. Set binary name
sed -i '' "s/name = \"ckeletin-rust\"/name = \"$NAME\"/" crates/cli/Cargo.toml
sed -i '' "s/name = \"ckeletin-rust\"/name = \"$NAME\"/" crates/cli/src/root.rs

# 2. Update workspace metadata
sed -i '' "s|peiman/ckeletin-rust|peiman/$NAME|g" Cargo.toml

# 3. Update Justfile binary name
sed -i '' "s/binary_name := \"ckeletin-rust\"/binary_name := \"$NAME\"/" Justfile

# 4. Update ping message to use new name
sed -i '' "s/ckeletin-rust is alive/$NAME is alive/g" crates/domain/src/ping.rs
sed -i '' "s/ckeletin-rust/$NAME/g" crates/cli/tests/cli.rs

# 5. Strip demo code
rm -f crates/domain/src/ping.rs
sed -i '' '/pub mod ping;/d' crates/domain/src/lib.rs

rm -f crates/cli/src/ping.rs
sed -i '' '/mod ping;/d' crates/cli/src/main.rs
sed -i '' '/Ping,/d' crates/cli/src/root.rs
sed -i '' '/Check connectivity/d' crates/cli/src/root.rs
sed -i '' '/Commands::Ping/d' crates/cli/src/main.rs

# Remove ping-related integration tests
sed -i '' '/ping/Id' crates/cli/tests/cli.rs

# 6. Reset CHANGELOG.md
cat > CHANGELOG.md << 'CHANGELOG'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
CHANGELOG

# 7. Verify
echo "Verifying..."
if cargo check --workspace -q; then
    echo "Workspace compiles."
else
    echo "Error: workspace does not compile after init. Something went wrong."
    exit 1
fi

# 8. Reset git history
CKELETIN_VERSION=$(cat .ckeletin/VERSION)
rm -rf .git
git init
git add -A
git commit -m "Initial scaffold from ckeletin-rust v$CKELETIN_VERSION"
git tag -a "v0.0.0" -m "Initial scaffold"

echo ""
echo "Done! $NAME is ready."
echo "  Binary: cargo run -p cli"
echo "  Tests:  just check"
