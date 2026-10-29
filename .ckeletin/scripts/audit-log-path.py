#!/usr/bin/env python3
"""Print the audit log path the framework binary would write.

The application name is the Cargo ``[[bin]]`` name. That is the value
``logging::resolve_audit_path`` receives as ``CARGO_BIN_NAME``. A relative
log path is anchored under the config home and that name. ``log_file_path``
and ``log_location`` come from the environment, then ``config.toml``, then
``[defaults]`` in ``ckeletin-project.toml``, then the framework defaults.

The environment prefix is the string literal passed as the second argument
of every ``Config::load`` call outside ``.ckeletin/``. ``just init`` writes
that literal into the consumer's cli. With no call, the scaffold prefix
``CKELETIN_`` remains. Calls that disagree, or that pass a non-literal,
are an error: a guessed prefix would report a log the application does
not write.
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


def rust_files(root):
    skip = {".git", "target", ".ckeletin"}
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in skip]
        for name in filenames:
            if name.endswith(".rs"):
                found.append(os.path.join(dirpath, name))
    found.sort()
    return found


def _skip_quoted(text, i):
    quote = text[i]
    i += 1
    while i < len(text):
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == quote:
            return i + 1
        i += 1
    return i


def _raw_string_end(text, i):
    """Return the index after a raw string starting at ``r``, or None."""
    if i >= len(text) or text[i] != "r":
        return None
    hashes = 0
    j = i + 1
    while j < len(text) and text[j] == "#":
        hashes += 1
        j += 1
    if j >= len(text) or text[j] != '"':
        return None
    closer = '"' + ("#" * hashes)
    end = text.find(closer, j + 1)
    if end < 0:
        return len(text)
    return end + len(closer)


def _skip_space_and_comments(text, i):
    while i < len(text):
        if text[i] in " \t\r\n":
            i += 1
            continue
        if text.startswith("//", i):
            newline = text.find("\n", i)
            i = len(text) if newline < 0 else newline + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = len(text) if end < 0 else end + 2
            continue
        break
    return i


def _ident_char(ch):
    return ch.isalnum() or ch == "_"


def _string_literal(arg):
    arg = arg.strip()
    if arg.startswith("r"):
        end = _raw_string_end(arg, 0)
        if end != len(arg):
            return None
        hashes = 0
        j = 1
        while j < len(arg) and arg[j] == "#":
            hashes += 1
            j += 1
        return arg[j + 1 : len(arg) - 1 - hashes]
    if len(arg) < 2 or arg[0] != '"' or arg[-1] != '"':
        return None
    body = arg[1:-1]
    out = []
    i = 0
    while i < len(body):
        if body[i] == "\\":
            if i + 1 >= len(body) or body[i + 1] not in '\\"':
                return None
            out.append(body[i + 1])
            i += 2
            continue
        out.append(body[i])
        i += 1
    return "".join(out)


def _call_args(text, open_paren):
    """Split the argument source of a call whose ``(`` is at ``open_paren``."""
    depth = 1
    args = [""]
    i = open_paren + 1
    while i < len(text):
        if text.startswith("//", i):
            newline = text.find("\n", i)
            i = len(text) if newline < 0 else newline + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = len(text) if end < 0 else end + 2
            continue
        raw_end = _raw_string_end(text, i)
        if raw_end is not None:
            args[-1] += text[i:raw_end]
            i = raw_end
            continue
        if text[i] in "\"'":
            end = _skip_quoted(text, i)
            args[-1] += text[i:end]
            i = end
            continue
        ch = text[i]
        if ch == "(":
            depth += 1
            args[-1] += ch
            i += 1
            continue
        if ch == ")":
            depth -= 1
            if depth == 0:
                return [arg for arg in args if arg.strip()], i + 1
            args[-1] += ch
            i += 1
            continue
        if ch == "," and depth == 1:
            args.append("")
            i += 1
            continue
        args[-1] += ch
        i += 1
    return None, i


def load_prefixes(text):
    """Second-argument string literals of ``Config::load`` in ``text``."""
    prefixes = []
    i = 0
    needle = "Config::load"
    while i < len(text):
        if text.startswith("//", i):
            newline = text.find("\n", i)
            i = len(text) if newline < 0 else newline + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = len(text) if end < 0 else end + 2
            continue
        raw_end = _raw_string_end(text, i)
        if raw_end is not None:
            i = raw_end
            continue
        if text[i] in "\"'":
            i = _skip_quoted(text, i)
            continue
        if text.startswith(needle, i) and (i == 0 or not _ident_char(text[i - 1])):
            after = i + len(needle)
            if after < len(text) and _ident_char(text[after]):
                i += 1
                continue
            cursor = _skip_space_and_comments(text, after)
            if cursor < len(text) and text[cursor] == "(":
                args, end = _call_args(text, cursor)
                if args is None or len(args) < 2:
                    return None
                prefix = _string_literal(args[1])
                if prefix is None:
                    return None
                prefixes.append(prefix)
                i = end
                continue
        i += 1
    return prefixes


def env_prefix(root):
    """The consumer's prefix, or ``CKELETIN_`` when no call exists."""
    found = []
    for path in rust_files(root):
        with open(path, encoding="utf-8") as handle:
            prefixes = load_prefixes(handle.read())
        if prefixes is None:
            sys.stderr.write(
                f"{path}: Config::load's second argument must be a string literal, "
                "the environment prefix\n"
            )
            sys.exit(1)
        for prefix in prefixes:
            found.append((path, prefix))
    if not found:
        return "CKELETIN_"
    unique = {prefix for _, prefix in found}
    if len(unique) != 1:
        sys.stderr.write("Config::load env prefixes disagree:\n")
        for path, prefix in found:
            sys.stderr.write(f"{path}: {prefix}\n")
        sys.exit(1)
    return found[0][1]


def main():
    root = os.getcwd()
    names = bin_names(root)
    app = names[0] if names else os.path.basename(os.path.abspath(root))
    prefix = env_prefix(root)

    path = os.environ.get(prefix + "LOG_FILE_PATH")
    if not path:
        path = read_key(os.path.join(root, "config.toml"), "log_file_path")
    if not path:
        path = read_key(
            os.path.join(root, "ckeletin-project.toml"), "log_file_path", "defaults"
        )
    if not path:
        path = "logs/app.log"

    location = os.environ.get(prefix + "LOG_LOCATION")
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
