#!/usr/bin/env bash
# Print changelog bullets for one framework version that name a file or recipe
# an update does not edit. The update replaces `.ckeletin/` only.
# Usage: changelog-consumer-edits.sh CHANGELOG VERSION
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "usage: changelog-consumer-edits.sh CHANGELOG VERSION" >&2
    exit 2
fi

file=$1
version=$2

awk -v ver="$version" '
function flush() {
    if (bullet ~ /CI_TEMP_PREFIX|ckeletin-deny-history|secret-scan|ckeletin-project\.toml|\.github\/|LogConfig|public_imports_cli/) {
        printf "%s", bullet
    }
    bullet = ""
}
$0 ~ "^## \\[" ver "\\]" { in_section = 1; next }
in_section && /^## / { flush(); exit }
in_section && /^- / {
    flush()
    bullet = $0 "\n"
    next
}
in_section && bullet != "" && /^  / {
    bullet = bullet $0 "\n"
    next
}
in_section && bullet != "" { flush() }
END { if (in_section) flush() }
' "$file"
