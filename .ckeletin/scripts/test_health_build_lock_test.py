#!/usr/bin/env python3
"""The health-lock timeout is configured, and the default stays 180."""

import importlib.util
import os
from pathlib import Path
import unittest


def load():
    path = Path(__file__).with_name("test_health_build_lock.py")
    spec = importlib.util.spec_from_file_location("health_lock", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class HealthLockTimeout(unittest.TestCase):
    def test_default_is_180(self):
        os.environ.pop("CKELETIN_HEALTH_LOCK_TIMEOUT", None)
        self.assertEqual(load().timeout_seconds(), 180)

    def test_environment_overrides_the_default(self):
        os.environ["CKELETIN_HEALTH_LOCK_TIMEOUT"] = "600"
        try:
            self.assertEqual(load().timeout_seconds(), 600)
        finally:
            os.environ.pop("CKELETIN_HEALTH_LOCK_TIMEOUT", None)

    def test_unusable_values_keep_the_default(self):
        for raw in ("0", "-5", "soon", ""):
            os.environ["CKELETIN_HEALTH_LOCK_TIMEOUT"] = raw
            try:
                self.assertEqual(load().timeout_seconds(), 180, raw)
            finally:
                os.environ.pop("CKELETIN_HEALTH_LOCK_TIMEOUT", None)


if __name__ == "__main__":
    unittest.main()
