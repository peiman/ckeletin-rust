#!/usr/bin/env bash
# Print the gitleaks config to pass as --config.
#
# A root .gitleaks.toml is the project file. It should extend
# .ckeletin/configs/gitleaks.toml. The framework file is replaced on update,
# so project allowlist entries belong in the root file.
# When the root file is absent, the scan uses the framework config.
set -euo pipefail

if [ -f .gitleaks.toml ]; then
    printf '%s\n' .gitleaks.toml
else
    printf '%s\n' .ckeletin/configs/gitleaks.toml
fi
