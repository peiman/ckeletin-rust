#!/usr/bin/env bash
# just ckeletin-update replaces .ckeletin/ and does not edit lefthook.yml.
# Say so when that file is missing the gitleaks pre-commit command.
set -euo pipefail

if [ ! -f lefthook.yml ] || ! grep -q 'gitleaks' lefthook.yml; then
    echo "lefthook.yml has no gitleaks pre-commit. Merge that command from the ckeletin-rust lefthook.yml. just ckeletin-update does not change this file."
fi
