#!/usr/bin/env python3
"""Give workspace tests a private home and temp folder.

Cargo test executables live in deps/. Direct invocation of a test binary must
use this launcher too (`--test` is the explicit form). Ordinary `cargo run`
executables keep the caller's environment.

The directory is removed when the child exits. The launcher therefore cannot
exec the test: it has to stay alive, forward signals, and delete the folder.
If this process is killed first, the directory can remain; the CI job temp
root is the backstop for that case.
"""

import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile

# A Unix socket path must be shorter than sun_path (104 bytes on macOS,
# including the trailing NUL on some kernels). Fixtures append their own
# names, so the private temp folder has to leave 70 bytes.
_SOCKET_LIMIT = 104
_SOCKET_ROOM = 70


def _leaves_socket_room(directory):
    sample = os.path.join(directory, "s" * _SOCKET_ROOM)
    return len(os.fsencode(sample)) < _SOCKET_LIMIT


def _short_directory():
    """A real directory whose path can hold a Unix socket.

    A symlink would be short enough, and a no-follow credential check would
    then report the target as unreadable. This directory is removed when the
    launcher exits. Set TMPDIR to a short path such as /tmp to keep the
    private folder in the caller's own temp directory instead.
    """
    try:
        # /tmp is a symlink on macOS. A no-follow open or a path comparison
        # then sees the link, not the directory. Create the folder on the
        # resolved path.
        path = tempfile.mkdtemp(prefix="s-", dir=os.path.realpath("/tmp"))
    except OSError as error:
        sys.stderr.write(
            "The temporary folder is too long for a Unix socket, and a "
            f"short folder under /tmp could not be created ({error}). "
            "Set TMPDIR to a short path such as /tmp and run the tests again.\n"
        )
        sys.exit(1)
    if _leaves_socket_room(path):
        return path
    shutil.rmtree(path, ignore_errors=True)
    sys.stderr.write(
        "The temporary folder is too long for a Unix socket. "
        "Set TMPDIR to a short path such as /tmp and run the tests again.\n"
    )
    sys.exit(1)


args = sys.argv[1:]
forced_test = bool(args) and args[0] == "--test"
if forced_test:
    args = args[1:]
if not args:
    sys.stderr.write("usage: test-runner.py [--test] <executable> [args...]\n")
    sys.exit(2)

if forced_test or os.path.basename(os.path.dirname(args[0])) == "deps":
    for name in list(os.environ):
        if name.upper().startswith("GIT_"):
            del os.environ[name]
    # Nested cargo (the health check, any test that builds) must keep using
    # the toolchain that is already installed. setdefault preserves a caller
    # who pointed these somewhere else. Read the home before replacing it.
    original_home = str(Path.home())
    os.environ.setdefault("CARGO_HOME", os.path.join(original_home, ".cargo"))
    os.environ.setdefault("RUSTUP_HOME", os.path.join(original_home, ".rustup"))
    # One home per test process, inherited by every child that does not set
    # its own. TMPDIR is the root itself, not root/tmp: Unix socket fixtures
    # have a path limit, and a killed child cannot drop its own TempDir.
    with tempfile.TemporaryDirectory(prefix="t-") as root:
        visible = root
        remove_visible = False
        if not _leaves_socket_room(root):
            visible = _short_directory()
            remove_visible = True
        try:
            for name, suffix in (
                ("HOME", "home"),
                ("XDG_CONFIG_HOME", "config"),
                ("XDG_STATE_HOME", "state"),
                ("XDG_CACHE_HOME", "cache"),
            ):
                path = os.path.join(visible, suffix)
                os.mkdir(path)
                os.environ[name] = path
            os.environ["TMPDIR"] = visible
            os.environ["TMP"] = visible
            os.environ["TEMP"] = visible
            gitconfig = os.path.join(visible, "home", ".gitconfig")
            with open(gitconfig, "w", encoding="utf-8") as handle:
                handle.write(
                    "[user]\n"
                    "\tname = ckeletin-test\n"
                    "\temail = ckeletin-test@example.invalid\n"
                )
            child = subprocess.Popen(args)

            def forward_signal(signum, _frame):
                try:
                    child.send_signal(signum)
                except ProcessLookupError:
                    # The child may exit before a pending signal reaches us.
                    pass

            for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
                signal.signal(sig, forward_signal)
            status = child.wait()
        finally:
            if remove_visible:
                shutil.rmtree(visible, ignore_errors=True)
    sys.exit(status if status >= 0 else 128 - status)

os.execvp(args[0], args)
