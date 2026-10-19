#!/usr/bin/env bash
# .ckeletin/scripts/check-framework-update.sh
#
# Compare .ckeletin/VERSION with upstream. An update exists only when the
# upstream framework version is a newer semver. A commit that does not change
# that version is not an update. A local version ahead of upstream is not one
# either. One script serves the notice and `just ckeletin-check-update`.
#
#   (no args)       notice: throttled, skipped in CI, opt-out, always exits 0
#   check           explicit text check; fetches every time; always exits 0
#   check json      explicit JSON check; fetches every time; always exits 0
#
# When a comparison is made the JSON object has current, latest, and
# update_available. When the check does not apply it has applicable: false.

set -o pipefail

mode=notice
format=text
if [ "${1:-}" = "check" ]; then
    mode=check
    if [ "${2:-}" = "json" ]; then
        format=json
    fi
fi

json_escape() {
    local v=$1
    if [ -z "$v" ]; then
        printf 'null'
        return
    fi
    case "$v" in
        *\"* | *\\* | *[[:cntrl:]]*) printf 'null' ;;
        *) printf '"%s"' "$v" ;;
    esac
}

semver_parse() {
    local raw=$1
    raw=${raw#v}
    raw=${raw#V}
    local core=${raw%%+*}
    PARSED_PRE=""
    if [[ "$core" == *-* ]]; then
        PARSED_PRE=${core#*-}
        core=${core%%-*}
    fi
    if [[ ! "$core" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
        return 1
    fi
    PARSED_MAJ=${BASH_REMATCH[1]}
    PARSED_MIN=${BASH_REMATCH[2]}
    PARSED_PAT=${BASH_REMATCH[3]}
    return 0
}

semver_is_newer() {
    semver_parse "$1" || return 1
    local up_maj=$PARSED_MAJ up_min=$PARSED_MIN up_pat=$PARSED_PAT up_pre=$PARSED_PRE
    semver_parse "$2" || return 1
    local loc_maj=$PARSED_MAJ loc_min=$PARSED_MIN loc_pat=$PARSED_PAT loc_pre=$PARSED_PRE
    if ((10#$up_maj > 10#$loc_maj)); then return 0; fi
    if ((10#$up_maj < 10#$loc_maj)); then return 1; fi
    if ((10#$up_min > 10#$loc_min)); then return 0; fi
    if ((10#$up_min < 10#$loc_min)); then return 1; fi
    if ((10#$up_pat > 10#$loc_pat)); then return 0; fi
    if ((10#$up_pat < 10#$loc_pat)); then return 1; fi
    if [ -z "$up_pre" ] && [ -n "$loc_pre" ]; then
        return 0
    fi
    return 1
}

read_local_version() {
    tr -d '[:space:]' < .ckeletin/VERSION 2> /dev/null || true
}

emit_json_comparison() {
    local current=$1 latest=$2 available=$3
    printf '{"current":%s,"latest":%s,"update_available":%s}\n' \
        "$(json_escape "$current")" "$(json_escape "$latest")" "$available"
}

finish_exempt() {
    local reason=$1
    local text=$2
    if [ "$format" = "json" ]; then
        printf '{"applicable":false,"reason":"%s"}\n' "$reason"
    elif [ "$mode" = "check" ]; then
        printf '%s\n' "$text"
    fi
    exit 0
}

if [ "$mode" = "notice" ]; then
    if [ -n "${CI:-}" ]; then exit 0; fi
    if [ "${CKELETIN_SKIP_UPDATE_CHECK:-}" = "1" ]; then exit 0; fi
fi

# The workspace repository slug is the upstream fingerprint. `just init`
# rewrites it in the root Cargo.toml, so a manifest that still names it is
# this repository. Split so a tree-wide rewrite does not retarget the guard.
UPSTREAM_SLUG="peiman/ckeletin-""rust"
if [ -f Cargo.toml ] && grep -q "$UPSTREAM_SLUG" Cargo.toml; then
    finish_exempt "upstream repository" "This is the ${UPSTREAM_SLUG} upstream repository itself — framework update check does not apply here."
fi

if ! git remote get-url ckeletin-upstream > /dev/null 2>&1; then
    if [ "$format" = "json" ]; then
        printf '{"applicable":false,"reason":"no_upstream_remote"}\n'
    elif [ "$mode" = "check" ]; then
        echo "No upstream remote. Run: git remote add ckeletin-upstream https://github.com/peiman/ckeletin-rust.git"
    fi
    exit 0
fi

should_fetch=1
if [ "$mode" = "notice" ]; then
    # Outside the framework tree. A stamp inside .ckeletin/ is a dirty
    # worktree, and the update then refuses to run.
    STAMP=$(git rev-parse --git-path ckeletin-last-update-check 2> /dev/null || true)
    TTL_SECONDS="${CKELETIN_UPDATE_CHECK_TTL:-86400}"
    case "$TTL_SECONDS" in
        '' | *[!0-9]*) TTL_SECONDS=86400 ;;
    esac
    NOW=$(date +%s 2> /dev/null || echo 0)
    if [ -f "$STAMP" ]; then
        LAST=$(cat "$STAMP" 2> /dev/null || echo 0)
        case "$LAST" in
            '' | *[!0-9]*) LAST=0 ;;
        esac
        if [ "$NOW" -ne 0 ] && [ $((NOW - LAST)) -lt "$TTL_SECONDS" ]; then
            should_fetch=0
        fi
    fi
fi

fetched=0
if [ "$should_fetch" -eq 1 ]; then
    TIMEOUT=""
    if command -v timeout > /dev/null 2>&1; then
        TIMEOUT="timeout 5"
    elif command -v gtimeout > /dev/null 2>&1; then
        TIMEOUT="gtimeout 5"
    fi
    # shellcheck disable=SC2086
    if $TIMEOUT git fetch ckeletin-upstream main --quiet > /dev/null 2>&1; then
        fetched=1
    fi
    if [ "$mode" = "notice" ]; then
        if [ -n "$STAMP" ]; then
            mkdir -p "$(dirname "$STAMP")" 2> /dev/null || true
            echo "${NOW:-0}" > "$STAMP" 2> /dev/null || true
        fi
    fi
elif git rev-parse --verify --quiet ckeletin-upstream/main > /dev/null 2>&1; then
    fetched=1
fi

LOCAL=$(read_local_version)
if [ "$fetched" -ne 1 ]; then
    if [ "$format" = "json" ]; then
        emit_json_comparison "$LOCAL" "" false
    elif [ "$mode" = "check" ]; then
        echo "Framework update status is unknown (upstream unreachable)."
    fi
    exit 0
fi

REMOTE=$(git show "ckeletin-upstream/main:.ckeletin/VERSION" 2> /dev/null | tr -d '[:space:]' || true)
available=false
if semver_is_newer "$REMOTE" "$LOCAL"; then
    available=true
fi

if [ "$format" = "json" ]; then
    emit_json_comparison "$LOCAL" "$REMOTE" "$available"
    exit 0
fi

if [ "$available" = "true" ]; then
    echo "Update available: v${LOCAL} → v${REMOTE}"
    echo "Run: just ckeletin-update"
    exit 0
fi

if [ "$mode" = "check" ]; then
    if [ "$LOCAL" = "$REMOTE" ]; then
        echo "Framework is up to date (v${LOCAL})"
    else
        echo "Framework v${LOCAL} is not behind upstream v${REMOTE}."
    fi
fi
exit 0
