#!/usr/bin/env bash
# Print the lines that explain a failed compile or test gate.
# The tail of a log is the wrong selection: later passing tests push the
# failing assertion out of the last twenty lines.
# Usage: gate-failure-reason.sh LOGFILE
set -euo pipefail

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "usage: gate-failure-reason.sh LOGFILE" >&2
    exit 2
fi

awk '
function interesting(line) {
    return line ~ /^error(\[|:)/ || line ~ /FAILED/ || line ~ /panicked at/ || line ~ /^failures:/ || line ~ /assertion `/ || line ~ /^test result: FAILED/
}
{
    lines[NR] = $0
}
END {
    kept = 0
    for (i = 1; i <= NR; i++) {
        if (!interesting(lines[i])) {
            continue
        }
        end = i
        if (lines[i] ~ /panicked at/ || lines[i] ~ /^error/) {
            end = i + 6
            if (end > NR) {
                end = NR
            }
        }
        for (j = i; j <= end; j++) {
            seen[j] = 1
        }
        kept = 1
    }
    if (!kept) {
        for (i = 1; i <= NR; i++) {
            print lines[i]
        }
        exit
    }
    for (i = 1; i <= NR; i++) {
        if (seen[i]) {
            print lines[i]
        }
    }
}
' "$1"
