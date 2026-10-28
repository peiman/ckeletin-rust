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

# 0.2.30 wrote the update-check stamp here. This script ignores the directory.
# Say so when it is still on disk. A project still running 0.2.30 never
# reaches this line; the README carries the same step for that jump.
if [ -d .ckeletin/cache ]; then
    echo ".ckeletin/cache/ is a retired update-check stamp. Delete it: rm -rf .ckeletin/cache"
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
git fetch --quiet --no-tags ckeletin-upstream main:refs/remotes/ckeletin-upstream/main

if ! git rev-parse --verify --quiet ckeletin-upstream/main >/dev/null; then
    echo "Error: ckeletin-upstream/main is not available."
    if [ "$mode" = "dry-run" ]; then
        write_marker false
        exit 0
    fi
    emit_stopped unknown_version
    exit 1
fi

# The baseline is the tree this project imported, not the newest upstream
# commit that still carries the same version. A later commit can keep the
# version while changing .ckeletin/ (the 0.2.34 update picked such a commit
# and reported a local difference that was not one).
#
# `.ckeletin-imported` is project-owned. The update writes the upstream commit
# it checked out. When that record is missing or not in this clone, use the
# first-parent commit that introduced this version: skip newer versions, then
# the oldest commit in the contiguous run that still has it.
IMPORTED_RECORD=".ckeletin-imported"

baseline_from_record() {
    local recorded
    [ -f "$IMPORTED_RECORD" ] || return 0
    recorded=$(tr -d '[:space:]' < "$IMPORTED_RECORD")
    [ -n "$recorded" ] || return 0
    if git cat-file -e "${recorded}^{commit}" 2>/dev/null; then
        printf '%s\n' "$recorded"
    fi
}

find_introduced_version() {
    local commit ver found=""
    while IFS= read -r commit; do
        [ -z "$commit" ] && continue
        ver=$(git show "${commit}:.ckeletin/VERSION" 2>/dev/null | tr -d '[:space:]' || true)
        if [ "$ver" = "$FROM" ]; then
            found=$commit
            continue
        fi
        if [ -n "$found" ]; then
            printf '%s\n' "$found"
            return 0
        fi
    done < <(git log --first-parent --format=%H ckeletin-upstream/main)
    if [ -n "$found" ]; then
        printf '%s\n' "$found"
    fi
}

baseline=$(baseline_from_record)
if [ -z "$baseline" ]; then
    baseline=$(find_introduced_version)
fi
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
