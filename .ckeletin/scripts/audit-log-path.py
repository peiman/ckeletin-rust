#!/usr/bin/env python3
"""Print the audit log path the framework binary would write.

The application name is the Cargo ``[[bin]]`` name. That is the value
``logging::resolve_audit_path`` receives as ``CARGO_BIN_NAME``. A relative
log path is anchored under the config home and that name. ``log_file_path``
and ``log_location`` come from the environment, then ``config.toml``, then
``[defaults]`` in ``ckeletin-project.toml``, then the framework defaults.
"""

import os
import re
import sys

SKIP = {".git", "target", ".ckeletin"}


def bin_names(root):
    names = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP]
        if "Cargo.toml" not in filenames:
            continue
        path = os.path.join(dirpath, "Cargo.toml")
        in_bin = False
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                stripped = line.strip()
                if stripped.startswith("[[") and stripped.endswith("]]"):
                    in_bin = stripped == "[[bin]]"
                    continue
                if in_bin and stripped.startswith("["):
                    in_bin = False
                if not in_bin:
                    continue
                match = re.match(r'name\s*=\s*"([^"]+)"', stripped)
                if match:
                    names.append(match.group(1))
                    in_bin = False
    return names


def unquote(value):
    value = value.strip().rstrip(",")
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def read_key(path, key, section=None):
    if not os.path.isfile(path):
        return None
    current = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if stripped.startswith("[") and stripped.endswith("]"):
                current = stripped[1:-1].strip()
                continue
            if section is not None and current != section:
                continue
            match = re.match(r"([A-Za-z0-9_]+)\s*=\s*(.*)$", stripped)
            if match and match.group(1) == key:
                return unquote(match.group(2))
    return None


def config_base(location):
    home = os.environ.get("HOME") or os.environ.get("USERPROFILE") or ""
    if location in ("platform", "native", "os"):
        if sys.platform == "darwin" and home:
            return os.path.join(home, "Library", "Application Support")
        if os.name == "nt":
            return os.environ.get("APPDATA") or ".config"
        data_home = os.environ.get("XDG_DATA_HOME")
        if data_home:
            return data_home
        if home:
            return os.path.join(home, ".local", "share")
        return ".config"
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if xdg:
        return xdg
    if home:
        return os.path.join(home, ".config")
    return ".config"


def main():
    root = os.getcwd()
    names = bin_names(root)
    app = names[0] if names else os.path.basename(os.path.abspath(root))

    path = os.environ.get("CKELETIN_LOG_FILE_PATH")
    if not path:
        path = read_key(os.path.join(root, "config.toml"), "log_file_path")
    if not path:
        path = read_key(
            os.path.join(root, "ckeletin-project.toml"), "log_file_path", "defaults"
        )
    if not path:
        path = "logs/app.log"

    location = os.environ.get("CKELETIN_LOG_LOCATION")
    if not location:
        location = read_key(os.path.join(root, "config.toml"), "log_location")
    if not location:
        location = read_key(
            os.path.join(root, "ckeletin-project.toml"), "log_location", "defaults"
        )
    if not location:
        location = "config"

    if os.path.isabs(path):
        print(path)
        return
    print(os.path.join(config_base(location), app, path))


if __name__ == "__main__":
    main()
