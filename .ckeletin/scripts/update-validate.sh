#!/usr/bin/env bash
# Tier 1 and tier 2 of ckeletin-update, after .ckeletin/ has been replaced.
# Sourced by the update recipe. The recipe that is already running is the one
# from before the replace, so a release's own update cannot see that release's
# advice. From 0.2.36 the recipe execs `just ckeletin-update-continue` in the
# imported tree, and that recipe sources this file.
#
# Expects OLD_VERSION or CKELETIN_UPDATE_OLD_VERSION, and the same for
# DISCARDED and REQ_COMMIT (true/false). The caller owns `set -o pipefail`.
# Do not enable strict mode here: this file is sourced.

json_escape() {
    local text=$1
    if [ "${#text}" -gt 8000 ]; then
        text="${text:0:8000}"$'\n'"…"
    fi
    # Cargo on CI colors its diagnostics. A raw ESC in `reason` is not valid JSON.
    if command -v python3 >/dev/null 2>&1; then
        printf '%s' "$text" | python3 -c '
import re, sys
text = sys.stdin.read()
text = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", text)
text = re.sub(r"\x1b\][^\x07]*(?:\x07|\x1b\\)", "", text)
text = text.replace("\\", "\\\\").replace("\"", "\\\"")
text = text.replace("\r", "").replace("\n", "\\n").replace("\t", "\\t")
text = re.sub(r"[\x00-\x08\x0b\x0c\x0e-\x1f]", "", text)
sys.stdout.write(text)
'
        return
    fi
    text=${text//\\/\\\\}
    text=${text//\"/\\\"}
    text=${text//$'\n'/\\n}
    text=${text//$'\t'/\\t}
    text=${text//$'\r'/}
    printf '%s' "$text"
}

run_update_validation() {
    OLD_VERSION=${OLD_VERSION:-${CKELETIN_UPDATE_OLD_VERSION:?}}
    DISCARDED=${DISCARDED:-${CKELETIN_UPDATE_DISCARDED:?}}
    REQ_COMMIT=${REQ_COMMIT:-${CKELETIN_UPDATE_COMMIT:?}}
    NEW_VERSION=$(tr -d '[:space:]' < .ckeletin/VERSION)

    compile_log=$(mktemp)
    if ! cargo check --workspace >"$compile_log" 2>&1; then
        cat "$compile_log"
        # Read the diagnostic while the imported script is still on disk.
        reason_text=$(bash .ckeletin/scripts/gate-failure-reason.sh "$compile_log" || true)
        rm -f "$compile_log"
        echo "Error: workspace does not compile after update. Rolling back."
        git restore --source=HEAD --staged --worktree -- .ckeletin/
        if [ -f .ckeletin/scripts/framework-lock.sh ]; then
            # shellcheck source=scripts/framework-lock.sh
            source .ckeletin/scripts/framework-lock.sh
            restore_framework_lock
        fi
        # The import record is written only after this compile succeeds.
        # A record from the previous update still names the tree rollback restored.
        reason_text=$(json_escape "$reason_text")
        printf 'CKELETIN_UPDATE_RESULT={"status":"compile_failed","from":"%s","to":"%s","committed":false,"rolled_back":true,"reason":"%s","discarded":false}\n' \
            "$OLD_VERSION" "$NEW_VERSION" "$reason_text"
        exit 1
    fi
    rm -f "$compile_log"

    # The compile succeeded, so this tree is what a later update compares against.
    if [ -n "${CKELETIN_UPDATE_REF:-}" ]; then
        git rev-parse "${CKELETIN_UPDATE_REF}^{commit}" > .ckeletin-imported
    fi

    check_log=$(mktemp)
    # Stream the gate. A redirected log looks stuck on a cold build, and the
    # exit status still comes from `just check` (pipefail is set by the caller).
    if ! just check 2>&1 | tee "$check_log"; then
        echo ""
        echo "Update to v$NEW_VERSION applied to the working tree, but 'just check' fails."
        consumer_edits=$(bash .ckeletin/scripts/changelog-consumer-edits.sh .ckeletin/CHANGELOG.md "$NEW_VERSION" || true)
        if [ -n "$consumer_edits" ]; then
            echo "These changelog entries for v$NEW_VERSION name a file or recipe this update does not edit:"
            printf '%s\n' "$consumer_edits"
        else
            echo "No changelog entry for v$NEW_VERSION names a file outside .ckeletin/. The failure above is in this repo."
        fi
        echo "Fix the violations above, then commit:"
        echo "  git add .ckeletin/ Cargo.lock .ckeletin-imported && git commit -m \"chore: update ckeletin framework from v$OLD_VERSION to v$NEW_VERSION\""
        echo "Left UNCOMMITTED (not rolled back) so you can fix forward."
        echo "This failure report is the one in the imported framework (v$NEW_VERSION). An update installs that report for the next update, not for itself, unless the previous framework already hands off to ckeletin-update-continue."
        reason_text=$(bash .ckeletin/scripts/gate-failure-reason.sh "$check_log" || true)
        rm -f "$check_log"
        reason_text=$(json_escape "$reason_text")
        printf 'CKELETIN_UPDATE_RESULT={"status":"check_failed","from":"%s","to":"%s","committed":false,"rolled_back":false,"reason":"%s","discarded":%s}\n' \
            "$OLD_VERSION" "$NEW_VERSION" "$reason_text" "$DISCARDED"
        exit 1
    fi
    rm -f "$check_log"
    echo ""
    echo "Updated ckeletin framework: v$OLD_VERSION → v$NEW_VERSION"
    echo ""
    git diff HEAD -- .ckeletin/CHANGELOG.md || true
    if [ "$REQ_COMMIT" = "true" ]; then
        git add -- .ckeletin/ Cargo.lock .ckeletin-imported ':(exclude).ckeletin/cache'
        git commit -m "chore: update ckeletin framework from v$OLD_VERSION to v$NEW_VERSION"
        COMMITTED=true
    else
        echo "Applied and left uncommitted."
        echo "  git add .ckeletin/ Cargo.lock .ckeletin-imported && git commit -m \"chore: update ckeletin framework from v$OLD_VERSION to v$NEW_VERSION\""
        COMMITTED=false
    fi
    printf 'CKELETIN_UPDATE_RESULT={"status":"updated","from":"%s","to":"%s","committed":%s,"rolled_back":false,"reason":null,"discarded":%s}\n' \
        "$OLD_VERSION" "$NEW_VERSION" "$COMMITTED" "$DISCARDED"
}
