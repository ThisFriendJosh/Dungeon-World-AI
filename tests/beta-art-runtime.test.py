"""Offline integration tests using the actual packaged base Python, including PEP 668.

Set DWA_BETA_ART_BASE_PYTHON to a staged base's python.exe. All installation is
confined to a disposable workspace virtual environment and uses a local wheel.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parent.parent
BASE = Path(os.environ.get("DWA_BETA_ART_BASE_PYTHON", "build/beta-art-base/python.exe"))
SCRIPT = ROOT / "apps/beta-setup/setup_art.py"


@unittest.skipUnless(os.name == "nt" and BASE.is_file(), "Set DWA_BETA_ART_BASE_PYTHON to the staged packaged base")
class PrivateArtEnvironment(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        parent = ROOT / "build/beta-art-regression"
        parent.mkdir(parents=True, exist_ok=True)
        cls.temp = tempfile.TemporaryDirectory(prefix="case-", dir=parent)
        cls.folder = Path(cls.temp.name)
        cls.environment = cls.folder / "Environment With Spaces"
        cls.python = cls.environment / "Scripts/python.exe"
        scratch = cls.folder / "temp"
        scratch.mkdir()
        cls.env = dict(os.environ, TEMP=str(scratch), TMP=str(scratch), PYTHONDONTWRITEBYTECODE="1")
        cls.marker = BASE.parent / "Lib/EXTERNALLY-MANAGED"
        cls.marker_before = cls.marker.read_bytes()
        cls.run_command([str(BASE.resolve()), "-I", "-X", "utf8", "-m", "venv", "--without-pip", str(cls.environment)])
        cls.run_command([str(cls.python), "-I", "-X", "utf8", "-m", "ensurepip", "--upgrade", "--default-pip"])
        cls.wheel = cls.folder / "dwa_beta_offline_probe-0.0.1-py3-none-any.whl"
        with zipfile.ZipFile(cls.wheel, "w") as wheel:
            wheel.writestr("dwa_beta_offline_probe.py", "VALUE = 'private-art-environment'\n")
            wheel.writestr("dwa_beta_offline_probe-0.0.1.dist-info/METADATA", "Metadata-Version: 2.1\nName: dwa-beta-offline-probe\nVersion: 0.0.1\n")
            wheel.writestr("dwa_beta_offline_probe-0.0.1.dist-info/WHEEL", "Wheel-Version: 1.0\nGenerator: DWA offline regression\nRoot-Is-Purelib: true\nTag: py3-none-any\n")
            wheel.writestr("dwa_beta_offline_probe-0.0.1.dist-info/RECORD", "")

    @classmethod
    def run_command(cls, command, expected=0):
        result = subprocess.run(command, env=cls.env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace",
                                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0), timeout=60)
        if result.returncode != expected:
            raise AssertionError(result.stdout)
        return result.stdout

    @classmethod
    def tearDownClass(cls):
        if cls.marker.read_bytes() != cls.marker_before:
            raise AssertionError("The managed base marker changed")
        cls.temp.cleanup()

    def test_real_virtual_environment_is_separate_from_managed_base(self):
        value = self.run_command([str(self.python), "-I", "-c", "import sys,json; print(json.dumps([sys.prefix,sys.base_prefix]))"])
        prefix, base = json.loads(value.strip())
        self.assertNotEqual(prefix, base)
        self.assertEqual(Path(prefix), self.environment)
        self.assertEqual(self.marker.read_bytes(), self.marker_before)

    def test_actual_package_install_succeeds_offline_and_keeps_output_without_a_console(self):
        code = ("import runpy; m=runpy.run_path(" + repr(str(SCRIPT)) + "); "
                "m['pip']('--no-index','--no-cache-dir'," + repr(str(self.wheel)) + "); "
                "import dwa_beta_offline_probe; print(dwa_beta_offline_probe.VALUE)")
        output = self.run_command([str(self.python), "-I", "-X", "utf8", "-u", "-c", code])
        self.assertIn("Successfully installed dwa-beta-offline-probe", output)
        self.assertIn("private-art-environment", output)

    def test_real_pip_error_reaches_the_setup_error_message(self):
        code = ("import runpy; m=runpy.run_path(" + repr(str(SCRIPT)) + ")\n"
                "try: m['pip']('--no-index','--no-cache-dir','dwa-missing-offline-fixture==0')\n"
                "except m['SetupError'] as e:\n print('DWA_ERROR '+str(e)); raise SystemExit(1)\n")
        output = self.run_command([str(self.python), "-I", "-X", "utf8", "-u", "-c", code], expected=1)
        self.assertIn("No matching distribution found", output)
        self.assertIn("DWA_ERROR AI art package setup failed:", output)

    def test_managed_base_is_rejected_before_installing_any_package(self):
        code = ("import runpy; m=runpy.run_path(" + repr(str(SCRIPT)) + ")\n"
                "try: m['pip']('--no-index'," + repr(str(self.wheel)) + ")\n"
                "except m['SetupError'] as e:\n print(str(e)); raise SystemExit(1)\n")
        output = self.run_command([str(BASE.resolve()), "-I", "-X", "utf8", "-c", code], expected=1)
        self.assertIn("private AI art environment is not initialized", output)
        self.assertEqual(self.marker.read_bytes(), self.marker_before)


if __name__ == "__main__":
    unittest.main(verbosity=2)
