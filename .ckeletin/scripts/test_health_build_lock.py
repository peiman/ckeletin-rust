#!/usr/bin/env python3
"""Run the health test while the parent Cargo build lock is held.

A green `cargo test` does not prove the nested check is isolated: the parent
often drops `target/debug/.cargo-lock` before the test starts. This holds
that lock and fails if `health_exits_zero_on_clean_tree` is still waiting.
The nested check builds a tiny fixture crate. The timeout catches a test
that is still waiting on this lock.
"""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def timeout_seconds():
    """How long the nested health build may run.

    The nested check is a tiny crate. Set CKELETIN_HEALTH_LOCK_TIMEOUT to a
    whole number of seconds when the machine is slow. A missing or unusable
    value keeps the 180 second default.
    """
    raw = os.environ.get("CKELETIN_HEALTH_LOCK_TIMEOUT", "180")
    try:
        value = int(raw)
    except (TypeError, ValueError):
        return 180
    if value <= 0:
        return 180
    return value


TIMEOUT_SECONDS = timeout_seconds()


def main():
    env = os.environ.copy()
    target = Path(env.get("CARGO_TARGET_DIR", ROOT / "target"))
    build = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "ckeletin",
            "--test",
            "health",
            "--no-run",
            "--message-format=json",
        ],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=TIMEOUT_SECONDS,
        check=False,
    )
    if build.returncode != 0:
        sys.stderr.write(build.stderr)
        return build.returncode

    executable = None
    for line in build.stdout.splitlines():
        if not line.startswith("{"):
            continue
        artifact = json.loads(line)
        target_name = artifact.get("target", {}).get("name")
        if artifact.get("executable") and target_name == "health":
            executable = artifact["executable"]
    if not executable:
        sys.stderr.write("cargo test --no-run did not name the health executable\n")
        return 1

    lock_path = target / "debug" / ".cargo-lock"
    if not lock_path.is_file():
        sys.stderr.write(f"parent build lock not found: {lock_path}\n")
        return 1

    import fcntl

    with lock_path.open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        process = subprocess.Popen(
            [executable, "--exact", "health_exits_zero_on_clean_tree"],
            cwd=ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        try:
            output, _ = process.communicate(timeout=TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            output, _ = process.communicate()
            sys.stderr.write(
                "health test waited on its parent's Cargo build lock:\n" + output
            )
            return 1
    if process.returncode != 0:
        sys.stderr.write(output)
        return process.returncode or 1
    if "1 passed; 0 failed" not in output:
        sys.stderr.write(output)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
