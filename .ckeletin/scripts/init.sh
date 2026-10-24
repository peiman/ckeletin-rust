#!/usr/bin/env bash
set -euo pipefail

NAME="${1:?Usage: just init name=<project-name>}"

# Validate name (lowercase, hyphens, no spaces)
if [[ ! "$NAME" =~ ^[a-z][a-z0-9-]*$ ]]; then
    echo "Error: name must be lowercase alphanumeric with hyphens (e.g., 'my-project')"
    exit 1
fi

# The CI temp folder is /tmp/<name>-ci.XXXXXXXX. ci_temp.rs requires that
# template plus 71 bytes to fit in a 104-byte socket path, which leaves 15
# characters for the name. Reject a longer name before any file is rewritten.
CI_TEMPLATE="/tmp/${NAME}-ci.XXXXXXXX"
if [ $((${#CI_TEMPLATE} + 71)) -gt 104 ]; then
    echo "Error: name '$NAME' is too long for the CI temp folder."
    echo "Use 15 characters or fewer so the template plus 71 bytes fits in 104."
    exit 1
fi

# Guard: refuse to run on an already-initialized / consumer repo.
# Detection: the upstream slug "peiman/ckeletin-rust" is rewritten by THIS
# script (step 2 below) so if it's absent, this is a derived project.
# Pass `force=true` as the second argument to bypass (use with care).
FORCE="${2:-false}"
if ! grep -q "peiman/ckeletin-rust" Cargo.toml 2>/dev/null; then
    if [ "$FORCE" != "true" ]; then
        echo "Error: this repo no longer looks like the ckeletin-rust scaffold" \
             "(the upstream slug 'peiman/ckeletin-rust' is absent from Cargo.toml)."
        echo "If this is a fresh clone that was already init'd, do not run init again."
        echo "To force (e.g. re-init intentionally): just init name=$NAME force=true"
        exit 1
    fi
    echo "Warning: force=true — bypassing already-initialized guard."
fi

# Pre-flight: warn about uncommitted changes. Checks BOTH unstaged and staged
# changes so that `git add`-ed work is not silently destroyed by the git
# history reset in step 8. Automatable: set CKELETIN_ASSUME_YES=1 to proceed
# without a prompt (for agent/CI use). In a non-interactive shell without that
# var we REFUSE rather than silently discard uncommitted work.
if [ -d .git ] && { ! git diff --quiet 2>/dev/null || ! git diff --cached --quiet 2>/dev/null; }; then
    echo "Warning: uncommitted changes exist (working tree or staging area). Init resets git history — uncommitted work will be lost."
    if [ "${CKELETIN_ASSUME_YES:-}" = "1" ]; then
        echo "CKELETIN_ASSUME_YES=1 — proceeding without prompt."
    elif [ -t 0 ]; then
        read -p "Continue? (y/N) " confirm
        if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
            echo "Aborted."
            exit 0
        fi
    else
        echo "Error: non-interactive shell and CKELETIN_ASSUME_YES is not set —" \
             "refusing to discard uncommitted changes. Set CKELETIN_ASSUME_YES=1 to proceed."
        exit 1
    fi
fi

echo "Initializing scaffold as: $NAME"

# Portable sed -i (macOS uses BSD sed, Linux uses GNU sed)
sedi() {
    if sed --version 2>/dev/null | grep -q GNU; then
        sed -i "$@"
    else
        sed -i '' "$@"
    fi
}

# 1. Set binary name and replace all ckeletin-rust references in CLI crate
sedi "s/name = \"ckeletin-rust\"/name = \"$NAME\"/" crates/cli/Cargo.toml
sedi "s/ckeletin-rust/$NAME/g" crates/cli/src/root.rs

# 2. Update workspace metadata
sedi "s|peiman/ckeletin-rust|peiman/$NAME|g" Cargo.toml

# 3. Update env prefix in main.rs (CKELETIN_ → PROJECT_NAME_)
UPPER_NAME=$(echo "$NAME" | tr '[:lower:]-' '[:upper:]_')
sedi "s/\"CKELETIN_\"/\"${UPPER_NAME}_\"/" crates/cli/src/main.rs

# 4. Update ping message to use new name
sedi "s/ckeletin-rust is alive/$NAME is alive/g" crates/domain/src/ping.rs
sedi "s/ckeletin-rust/$NAME/g" crates/cli/tests/cli.rs

# 4b. Rename the baked CI temp prefix. The prepare step runs before checkout,
# so it cannot read .github/CI_TEMP_PREFIX. The workflows carry the same
# token. A line that gates a job on the upstream repository stays verbatim,
# or a derived project would start running upstream-only jobs.
printf '%s\n' "$NAME" > .github/CI_TEMP_PREFIX
for wf in .github/workflows/*.yml; do
    [ -f "$wf" ] || continue
    tmp=$(mktemp)
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            *"github.repository =="*) printf '%s\n' "$line" ;;
            *) printf '%s\n' "${line//ckeletin-rust/$NAME}" ;;
        esac
    done < "$wf" > "$tmp"
    cat "$tmp" > "$wf"
    rm -f "$tmp"
done

# NOTE: The `ping` command is intentionally KEPT as the worked example. The
# steps above already renamed it (domain logic, CLI handler, and both the
# human and JSON integration tests). Do NOT strip it. `ping` is the only
# subcommand; deleting it leaves an empty `Commands` enum that the entry point
# cannot match exhaustively, so the project would fail to compile — and init
# would abort before creating the git repo (see issue #1). ckeletin-go's
# scaffold keeps its demo command for the same reason. Replace or extend
# `ping` when you add your first real command — see AGENTS.md, "Adding a New
# Command". Ref: https://github.com/peiman/ckeletin-rust/issues/1

# 5. Reset CHANGELOG.md
cat > CHANGELOG.md << 'CHANGELOG'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
CHANGELOG

# 5b. Strip upstream-only artifacts (scaffold export boundary — spec issue #30,
#     FB-002). A derived project must not inherit ckeletin-rust's OWN conformance
#     evidence or framework working-docs as if they described THIS project:
#       - generated conformance evidence — regenerable, and `just conform`
#         no-ops in a consumer, so a stale inherited report is worse than none;
#       - upstream-only prose — code reviews, consumer/dogfooding feedback, and
#         the framework's own spec-feedback / plans / specs under docs/.
#     Framework-owned `.ckeletin/` and consumer-owned `ckeletin-project.toml` are
#     deliberately KEPT. `rm -f`/`rm -rf` are no-ops if an artifact is absent.
echo "Stripping upstream-only conformance evidence and framework working-docs..."
rm -f CONFORMANCE.md conformance-mapping.toml conformance-report.json
rm -rf conformance docs
rm -f code-review-*.md consumer-feedback-*.md dogfooding-feedback-*.md

# 6. Verify — compile ALL targets (lib, bin, AND tests) BEFORE resetting git
#    history. Destroying git history and then failing is far worse than failing
#    and leaving history intact. Checking only the default targets would miss a
#    broken integration-test file: a test that fails to compile does not surface
#    until the user's first `just check`.
echo "Verifying..."
if cargo check --workspace --all-targets -q; then
    echo "Workspace and tests compile."
else
    echo "Error: workspace does not compile after init. Something went wrong."
    exit 1
fi

# Process-local identity for the scaffold commit and the annotated tag.
# Nothing is written into the new repository, so the next commit uses the
# developer's own gitconfig. Callers who already exported GIT_AUTHOR_* keep it.
# This does not consult `git var`: a host that guesses an identity would hide
# a missing fallback, and the guess would be baked into the scaffold commit.
ckeletin_init_git_identity() {
    export GIT_AUTHOR_NAME="${GIT_AUTHOR_NAME:-ckeletin-init}"
    export GIT_AUTHOR_EMAIL="${GIT_AUTHOR_EMAIL:-ckeletin-init@example.invalid}"
    export GIT_COMMITTER_NAME="${GIT_COMMITTER_NAME:-$GIT_AUTHOR_NAME}"
    export GIT_COMMITTER_EMAIL="${GIT_COMMITTER_EMAIL:-$GIT_AUTHOR_EMAIL}"
}

# 7. Reset git history — AFTER compile verification above.
CKELETIN_VERSION=$(cat .ckeletin/VERSION)
rm -rf .git
git init
ckeletin_init_git_identity
git add -A
git commit -m "Initial scaffold from ckeletin-rust v$CKELETIN_VERSION"
git tag -a "v0.0.0" -m "Initial scaffold"

echo ""
echo "Done! $NAME is ready."
echo "  Binary: cargo run -p cli"
echo "  Tests:  just check"
echo ""
echo "Remember:"
echo "  - Update the copyright holder and year in LICENSE-MIT before distributing."
echo "  - Keep LICENSE-APACHE verbatim (Apache-2.0 §4d / appendix)."
echo "    Copyright notices for Apache-2.0 belong in a NOTICE file or source"
echo "    file headers, not in edits to the license text itself."
