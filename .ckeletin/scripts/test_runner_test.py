#!/usr/bin/env python3
"""Check private test homes without moving the installed Rust tool caches."""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest


def _credential_walk_allows(path):
    """True when a no-follow walk can open this directory.

    The worker-token check refuses any symlink except the root-owned
    platform aliases /tmp and /var. A link anywhere else makes a missing
    token look unreadable.
    """
    current = Path(path).anchor or "/"
    for part in Path(path).parts[1:]:
        current = str(Path(current) / part)
        if not os.path.islink(current):
            continue
        if current not in ("/tmp", "/var") or os.lstat(current).st_uid != 0:
            return False
    return True


class TestRunner(unittest.TestCase):
    def test_private_tmp_keeps_unix_socket_paths_usable(self):
        # This script is also run inside the workspace launcher. A second
        # launcher must still leave room for a fixture's Unix socket path.
        launcher = Path(__file__).with_name("test-runner.py")
        result = subprocess.run(
            [str(launcher), "--test", "python3", "-c",
             "import pathlib,socket,tempfile; "
             "root=pathlib.Path(tempfile.mkdtemp()); "
             "listener=socket.socket(socket.AF_UNIX); "
             "listener.bind(str(root/'sock'))"],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_ci_root_and_launcher_leave_sixty_bytes_for_sockets(self):
        # Eight random characters are used by mktemp in CI. Test the real
        # launcher and bind a long fixture socket to catch added nesting.
        with tempfile.TemporaryDirectory(prefix="ck-ci.", dir="/tmp") as root:
            env = os.environ.copy()
            env.update(TMPDIR=root, TMP=root, TEMP=root)
            launcher = Path(__file__).with_name("test-runner.py")
            probe = r"""
import json, os, pathlib, socket, tempfile
root = pathlib.Path(tempfile.mkdtemp(prefix=".tmp"))
path = root / ("s" * 58)
listener = socket.socket(socket.AF_UNIX)
listener.bind(str(path))
print(json.dumps({"tmp": os.environ["TMPDIR"], "socket": str(path)}))
"""
            result = subprocess.run(
                [str(launcher), "--test", "python3", "-c", probe],
                env=env, capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            got = json.loads(result.stdout)
            self.assertLessEqual(len(os.fsencode(got["tmp"])) + 1 + 70, 104)
            self.assertLess(len(os.fsencode(got["socket"])), 104)

    def test_long_caller_temp_still_binds_a_socket_and_is_removed(self):
        # A nested worker folder is already past the Unix socket limit. The
        # launcher must hand the test a real directory short enough to bind.
        # A symlink is long enough on paper, and the worker-token check then
        # reports a missing token as unreadable because it will not follow
        # the link. Remove the directory when the test exits.
        launcher = Path(__file__).with_name("test-runner.py")
        with tempfile.TemporaryDirectory(prefix="w" * 80, dir="/tmp") as inherited:
            nested = os.path.join(inherited, "t-" + "n" * 8, "s" * 70)
            self.assertGreaterEqual(len(os.fsencode(nested)), 104)
            env = os.environ.copy()
            env.update(TMPDIR=inherited, TMP=inherited, TEMP=inherited)
            probe = r"""
import json, os, pathlib, socket, tempfile
root = pathlib.Path(tempfile.mkdtemp(prefix=".tmp"))
path = root / ("s" * 58)
listener = socket.socket(socket.AF_UNIX)
listener.bind(str(path))
print(json.dumps({
    "tmp": os.environ["TMPDIR"],
    "tmp_is_link": pathlib.Path(os.environ["TMPDIR"]).is_symlink(),
    "tmp_real": str(pathlib.Path(os.environ["TMPDIR"]).resolve()),
    "socket": str(path),
}))
"""
            result = subprocess.run(
                [str(launcher), "--test", "python3", "-c", probe],
                env=env, capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            got = json.loads(result.stdout)
            self.assertLessEqual(len(os.fsencode(got["tmp"])) + 1 + 70, 104)
            self.assertLess(len(os.fsencode(got["socket"])), 104)
            self.assertFalse(got["tmp_is_link"])
            self.assertTrue(_credential_walk_allows(got["tmp"]))
            current = Path(got["tmp"]).anchor or "/"
            for part in Path(got["tmp"]).parts[1:]:
                current = str(Path(current) / part)
                self.assertFalse(
                    os.path.islink(current),
                    f"{current} is a symlink; the short temp folder must be a real path",
                )
            self.assertFalse(
                str(Path(got["tmp_real"])).startswith(str(Path(inherited).resolve()))
            )
            self.assertFalse(Path(got["tmp"]).exists())
            self.assertFalse(Path(got["tmp_real"]).exists())

    def test_private_tmp_is_removed_after_child_exit(self):
        self.check_private_tmp(False)

    def test_private_tmp_is_removed_after_child_is_killed(self):
        self.check_private_tmp(True)

    def check_private_tmp(self, kill_child):
        # Deliberately leave a folder behind, as a killed Rust TempDir owner
        # would. Keep the inherited temp location different from the home.
        probe = r'''
import json, os, signal, tempfile
from pathlib import Path
leaked = Path(tempfile.mkdtemp(prefix="leaked-test-"))
(leaked / "file").write_text("left behind")
print(json.dumps({k: os.environ.get(k) for k in
                  ["HOME", "TMPDIR", "TMP", "TEMP"]} |
                 {"leaked": str(leaked), "pid": os.getpid(),
                  "tmp_is_link": Path(os.environ["TMPDIR"]).is_symlink(),
                  "tmp_real": str(Path(os.environ["TMPDIR"]).resolve())}),
      flush=True)
if os.environ["KILL_CHILD"] == "1":
    while True:
        signal.pause()
'''
        with tempfile.TemporaryDirectory() as inherited:
            env = os.environ.copy()
            env.update(TMPDIR=inherited, TMP=inherited, TEMP=inherited,
                       KILL_CHILD=str(int(kill_child)))
            launcher = Path(__file__).with_name("test-runner.py")
            with subprocess.Popen(
                [str(launcher), "--test", "python3", "-c", probe],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True,
            ) as process:
                got = json.loads(process.stdout.readline())
                if kill_child:
                    os.kill(got["pid"], signal.SIGKILL)
                _, stderr = process.communicate(timeout=10)
                self.assertEqual(process.returncode,
                                 128 + signal.SIGKILL if kill_child else 0,
                                 stderr)
            root = Path(got["HOME"]).parent
            # A path that fits stays in the caller's folder. A path that does
            # not is a real short directory, never a symlink: credential
            # checks refuse to follow one.
            self.assertFalse(got["tmp_is_link"])
            self.assertTrue(_credential_walk_allows(got["TMPDIR"]))
            if Path(got["tmp_real"]).parent == Path(inherited).resolve():
                self.assertEqual(root.parent, Path(inherited))
            else:
                self.assertLessEqual(
                    len(os.fsencode(got["TMPDIR"])) + 1 + 70, 104
                )
            tmp = Path(got["TMPDIR"])
            self.assertEqual(tmp, root)
            self.assertNotEqual(tmp, Path(got["HOME"]))
            self.assertEqual(got["TMP"], str(tmp))
            self.assertEqual(got["TEMP"], str(tmp))
            self.assertEqual(Path(got["leaked"]).parent, tmp)
            self.assertFalse(root.exists())
            self.assertFalse(tmp.exists())
            self.assertFalse(Path(got["leaked"]).exists())

    def test_signal_after_child_exit_keeps_status_and_cleans_home(self):
        self.check_signal_after_child_exit("ProcessLookupError", 7)

    def test_signal_forwarding_does_not_hide_other_errors(self):
        self.check_signal_after_child_exit("PermissionError", 1)

    def check_signal_after_child_exit(self, error, expected_status):
        # Hold the launcher in wait() AFTER the real child is reaped. CPython
        # versions can suppress ESRCH inside Popen.send_signal themselves; use
        # the OS call directly to exercise the launcher's own handling.
        wrapper = r'''
import json, os, runpy, subprocess, sys
original = subprocess.Popen
error = sys.argv[1]
class ExitedChild(original):
    def wait(self, *args, **kwargs):
        status = super().wait(*args, **kwargs)
        print(json.dumps({"home": os.environ["HOME"], "status": status}), flush=True)
        sys.stdin.readline()
        return status
    def send_signal(self, signum):
        if error == "PermissionError":
            raise PermissionError("forwarding denied")
        os.kill(self.pid, signum)
subprocess.Popen = ExitedChild
sys.argv = [sys.argv[2], "--test", sys.executable, "-c", "raise SystemExit(7)"]
runpy.run_path(sys.argv[0], run_name="__main__")
'''
        launcher = Path(__file__).with_name("test-runner.py")
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            with self.subTest(signal=sig, error=error):
                with subprocess.Popen(
                    ["python3", "-c", wrapper, error, str(launcher)],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, text=True,
                ) as process:
                    ready = json.loads(process.stdout.readline())
                    self.assertEqual(ready["status"], 7)
                    home = Path(ready["home"])
                    self.assertTrue(home.exists())
                    process.send_signal(sig)
                    _, stderr = process.communicate("\n", timeout=10)
                    self.assertFalse(home.parent.exists(), stderr)
                    self.assertEqual(process.returncode, expected_status, stderr)
                    if error == "ProcessLookupError":
                        self.assertEqual(stderr, "")
                    else:
                        self.assertIn(error, stderr)

    def test_private_home_keeps_original_rust_tool_locations(self):
        with tempfile.TemporaryDirectory() as root:
            env = os.environ.copy()
            env.update(HOME=root)
            env.pop("CARGO_HOME", None)
            env.pop("RUSTUP_HOME", None)
            launcher = Path(__file__).with_name("test-runner.py")
            result = subprocess.run(
                [str(launcher), "--test", "python3", "-c",
                 "import json,os; print(json.dumps({k:os.environ.get(k) for k in ['HOME','CARGO_HOME','RUSTUP_HOME']}))"],
                env=env, capture_output=True, text=True, check=True,
            )
            got = json.loads(result.stdout)
            self.assertNotEqual(got["HOME"], root)
            self.assertEqual(got["CARGO_HOME"], str(Path(root) / ".cargo"))
            self.assertEqual(got["RUSTUP_HOME"], str(Path(root) / ".rustup"))
            self.assertFalse(Path(got["HOME"]).exists())

    def test_private_home_has_a_fixed_git_identity(self):
        # Tests that commit must not depend on the machine's own git identity:
        # a CI VM has none, and macOS may guess one.
        with tempfile.TemporaryDirectory() as root:
            env = os.environ.copy()
            env.update(HOME=root)
            env.pop("GIT_CONFIG_GLOBAL", None)
            launcher = Path(__file__).with_name("test-runner.py")
            result = subprocess.run(
                [str(launcher), "--test", "sh", "-c",
                 "git config --global user.name; git config --global user.email"],
                env=env, capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.split(),
                             ["ckeletin-test", "ckeletin-test@example.invalid"])

    def test_non_test_executable_keeps_the_caller_environment(self):
        env = os.environ.copy()
        env["HOME"] = "/tmp"
        launcher = Path(__file__).with_name("test-runner.py")
        result = subprocess.run(
            [str(launcher), "python3", "-c", "import os; print(os.environ['HOME'])"],
            env=env, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "/tmp")


if __name__ == "__main__":
    unittest.main()
