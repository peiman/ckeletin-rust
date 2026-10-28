#!/usr/bin/env python3
"""Report whether an audit log path can be created.

Uses lstat, which does not follow symlinks. A dangling symlink is not
writable, even when its parent directory is. This matches
ckeletin::logging::file_log_status.
"""

import json
import os
import stat
import sys


def writable(path: str) -> bool:
    current = path
    creating_child = False
    while True:
        try:
            info = os.lstat(current)
        except FileNotFoundError:
            parent = os.path.dirname(current)
            if parent == current:
                return False
            creating_child = True
            current = parent
            continue
        except OSError:
            return False
        if stat.S_ISLNK(info.st_mode):
            try:
                os.stat(current)
            except OSError:
                return False
        try:
            followed = os.stat(current)
        except OSError:
            return False
        if creating_child and not stat.S_ISDIR(followed.st_mode):
            return False
        return os.access(current, os.W_OK)


def main() -> None:
    path = sys.argv[1]
    json.dump({"path": path, "writable": writable(path)}, sys.stdout)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
