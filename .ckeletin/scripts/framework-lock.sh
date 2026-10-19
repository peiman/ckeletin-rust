#!/usr/bin/env bash
# Lockfile helpers for the framework update recipes.
#
# Sourced library: deliberately no `set -euo pipefail` here. The sourcing
# recipe owns strict mode, and options set here would leak into it.
#
#   source .ckeletin/scripts/framework-lock.sh
#   refresh_framework_lock   # after .ckeletin/ has been replaced
#   restore_framework_lock   # put Cargo.lock back to the committed bytes

# cargo generate-lockfile re-resolves every dependency in the consumer
# workspace. An existing lockfile updates only the framework package.
# A project with no lockfile yet still needs one created.
refresh_framework_lock() {
    if [ -f Cargo.lock ]; then
        cargo update -p ckeletin
    else
        cargo generate-lockfile
    fi
}

# Regenerating the lock on the way out does not restore it. The committed
# bytes are the previous resolution, including packages this update did not touch.
restore_framework_lock() {
    if git cat-file -e HEAD:Cargo.lock > /dev/null 2>&1; then
        git restore --source=HEAD --staged --worktree -- Cargo.lock
    else
        rm -f Cargo.lock
    fi
}
