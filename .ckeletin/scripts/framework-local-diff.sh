#!/usr/bin/env bash
# .ckeletin/scripts/framework-local-diff.sh
#
# Compare this project's .ckeletin/ with upstream at the framework version the
# project records. `just ckeletin-update` applies no name rewrite — `just init`
# rewrites names outside this tree — so any difference is a local difference.
# If the update command starts re-applying an init rewrite inside .ckeletin/,
# exclude exactly those rewrites here and nowhere else.
#
# Usage:
#   framework-local-diff.sh <update|dry-run|compatibility> <discard_local> <upstream_url> <marker>
#
# On exit 0 the marker file is one word:
#   exempt  this checkout is the upstream repository; the caller exits 0
#   true    update is discarding a committed local difference
#   false   no committed local difference (dry-run may still have printed one)
# On exit 1 the caller must not change .ckeletin/. Stopped JSON is on stdout.
# Dry-run prints a local difference when one exists, changes nothing, and exits 0.
# Bash 3.2: no namerefs, no sort -V.

set -euo pipefail

export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat

mode=${1:?}
discard_local=${2:?}
upstream_url=${3:?}
marker=${4:?}

write_marker() {
    printf '%s\n' "$1" > "$marker"
}

# Split so a tree-wide replace of the upstream slug does not retarget the guard.
UPSTREAM_SLUG="peiman/ckeletin-""rust"
if [ -f Cargo.toml ] && grep -q "$UPSTREAM_SLUG" Cargo.toml; then
    echo "This is the ${UPSTREAM_SLUG} upstream repository itself — framework update does not apply here."
    write_marker exempt
    exit 0
fi

if [ ! -f .ckeletin/VERSION ]; then
    echo "Error: .ckeletin/VERSION is missing."
    printf 'CKELETIN_UPDATE_RESULT={"status":"stopped","reason":"unknown_version","from":"","to":null,"committed":false,"discarded":false}\n'
    exit 1
fi

# `from` is the committed framework version. The worktree can already hold
# the update under test, and reporting the version on disk hides that baseline.
FROM=$(git show HEAD:.ckeletin/VERSION 2> /dev/null | tr -d '[:space:]' || true)
if [ -z "$FROM" ]; then
    FROM=$(tr -d '[:space:]' < .ckeletin/VERSION)
fi

emit_stopped() {
    # reason is one of the fixed tokens uncommitted, unknown_version, local_difference.
    printf 'CKELETIN_UPDATE_RESULT={"status":"stopped","reason":"%s","from":"%s","to":null,"committed":false,"discarded":false}\n' \
        "$1" "$FROM"
}

# `.ckeletin/cache/` is a retired update-check stamp. It is not a local
# framework change, and a leftover file must not refuse the update.
framework_paths=(.ckeletin/ ':(exclude).ckeletin/cache')

show_worktree_diff() {
    git status --porcelain -- "${framework_paths[@]}" || true
    git --no-pager diff -- .ckeletin/ ':(exclude).ckeletin/cache' || true
    git --no-pager diff --cached -- .ckeletin/ ':(exclude).ckeletin/cache' || true
}

if [ -n "$(git status --porcelain -- "${framework_paths[@]}")" ]; then
    if [ "$mode" = "dry-run" ]; then
        echo "Uncommitted changes in .ckeletin/:"
    else
        echo "Error: .ckeletin/ has uncommitted changes. Commit or revert them before updating."
    fi
    show_worktree_diff
    if [ "$mode" != "dry-run" ]; then
        # discard_local does not apply to uncommitted changes.
        emit_stopped uncommitted
        exit 1
    fi
fi

if ! git remote | grep -qx ckeletin-upstream; then
    git remote add ckeletin-upstream "$upstream_url"
fi
git fetch ckeletin-upstream

if ! git rev-parse --verify --quiet ckeletin-upstream/main >/dev/null; then
    echo "Error: ckeletin-upstream/main is not available."
    if [ "$mode" = "dry-run" ]; then
        write_marker false
        exit 0
    fi
    emit_stopped unknown_version
    exit 1
fi

# Newest commit on upstream main whose .ckeletin/VERSION equals the local one.
find_baseline() {
    local commit ver
    while IFS= read -r commit; do
        [ -z "$commit" ] && continue
        ver=$(git show "${commit}:.ckeletin/VERSION" 2>/dev/null | tr -d '[:space:]' || true)
        if [ "$ver" = "$FROM" ]; then
            printf '%s\n' "$commit"
            return 0
        fi
    done < <(git log --format=%H ckeletin-upstream/main)
    return 0
}

baseline=$(find_baseline)
if [ -z "$baseline" ]; then
    echo "Error: no commit on ckeletin-upstream/main has .ckeletin/VERSION ${FROM}."
    if [ "$mode" = "dry-run" ]; then
        write_marker false
        exit 0
    fi
    emit_stopped unknown_version
    exit 1
fi

has_local_diff=false
if ! git --no-pager diff --quiet "$baseline" HEAD -- .ckeletin/ ':(exclude).ckeletin/cache'; then
    has_local_diff=true
fi

if [ "$has_local_diff" = "true" ]; then
    if [ "$mode" = "update" ] && [ "$discard_local" = "true" ]; then
        echo "Discarding local framework changes."
        git --no-pager diff --stat "$baseline" HEAD -- .ckeletin/ ':(exclude).ckeletin/cache' || true
        write_marker true
        exit 0
    fi
    echo "Local framework difference: .ckeletin/ does not match upstream ${FROM} (${baseline})."
    echo "Run: just ckeletin-update discard_local=true"
    git --no-pager diff --stat "$baseline" HEAD -- .ckeletin/ ':(exclude).ckeletin/cache' || true
    git --no-pager diff "$baseline" HEAD -- .ckeletin/ ':(exclude).ckeletin/cache' || true
    if [ "$mode" = "dry-run" ]; then
        write_marker false
        exit 0
    fi
    emit_stopped local_difference
    exit 1
fi

write_marker false
exit 0
