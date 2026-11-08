"""Offline checks for the explicit beta payload and graphical AI setup helper."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
HELPER = Path(os.environ.get("DWA_BETA_TEST_HELPER", "build/beta/DungeonWorldAI.exe"))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PayloadPolicy(unittest.TestCase):
    def test_private_data_cannot_be_added_by_scanning_the_project(self):
        source = (ROOT / "scripts/build-beta.py").read_text(encoding="utf-8")
        self.assertNotIn("ROOT.rglob", source)
        self.assertNotIn('files["launch-settings.json"]', source)
        self.assertNotIn("Players/", source)

    def test_manifest_has_only_public_runtime_roots_and_contains_no_source_paths(self):
        manifest = os.environ.get("DWA_BETA_MANIFEST")
        if not manifest:
            self.skipTest("Set DWA_BETA_MANIFEST to the built package-manifest.json")
        pack = json.loads(Path(manifest).read_text(encoding="utf-8"))
        self.assertEqual(pack["product"], "ai.dungeonworld.beta")
        names = [f["path"] for f in pack["files"]]
        self.assertEqual(len(names), len(set(n.lower() for n in names)))
        self.assertIn("DungeonWorldAI.exe", names)
        self.assertIn("app/dungeon-world-ai-desktop.exe", names)
        self.assertIn("app/ai/llama-server.exe", names)
        self.assertTrue(any(n.endswith("/ScryingPane.exe") for n in names))
        for f in pack["files"]:
            self.assertNotIn("source", f)
            self.assertNotIn("..", Path(f["path"]).parts)
            self.assertFalse(any(s in f["path"].lower() for s in (".sqlite", "campaigns/", "launch-settings", "tokens/", "setup.log")))
        self.assertEqual(sum(f["size"] for f in pack["files"]), pack["expandedBytes"])


class SetupChoices(unittest.TestCase):
    def setUp(self):
        self.art = load("beta_art_setup", ROOT / "apps/beta-setup/setup_art.py")

    def test_no_nvidia_uses_portable_cpu_packages(self):
        with patch.object(self.art.shutil, "which", return_value=None), patch.object(self.art.Path, "is_file", return_value=False):
            self.assertEqual(self.art.torch_channel(), "cpu")

    def test_compatible_nvidia_uses_pinned_cuda_channel(self):
        with patch.object(self.art.shutil, "which", return_value="nvidia-smi.exe"), patch.object(self.art.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "581.42\n", "")):
            self.assertEqual(self.art.torch_channel(), "cu130")

    def test_old_driver_uses_cpu_and_never_modifies_drivers(self):
        with patch.object(self.art.shutil, "which", return_value="nvidia-smi.exe"), patch.object(self.art.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "551.00\n", "")) as run:
            self.assertEqual(self.art.torch_channel(), "cpu")
            self.assertEqual(run.call_count, 1)

    @unittest.skipUnless(os.name == "nt" and HELPER.is_file(), "Set DWA_BETA_TEST_HELPER to the compiled helper")
    def test_native_paths_hashing_and_atomic_state_changes(self):
        parent = ROOT / "build/beta-helper-checks"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as directory:
            result = subprocess.run([str(HELPER.resolve()), "/self-test"], env=dict(os.environ, DWA_SETUP_TEST_ROOT=directory), capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))


class MsiTables(unittest.TestCase):
    def setUp(self):
        self.sdk = load("beta_msi_sdk", ROOT / "scripts/msi-sdk.py")
        self.pack = {"productCode": "{00000000-0000-0000-0000-000000000001}",
                     "files": [{"path": "DungeonWorldAI.exe", "size": 3},
                               {"path": "app/ai/vcomp140.dll", "size": 4}]}

    def test_all_file_components_and_shortcut_registry_keys_are_consistent(self):
        tables = self.sdk.prepare_tables(self.pack)
        files = {r[0] for r in tables["File"][3:]}
        registries = {r[0] for r in tables["Registry"][3:]}
        self.assertEqual(len(files), 2)
        for row in tables["Component"][3:]:
            self.assertIn(row[5], registries if int(row[3]) & 4 else files)
        desktop = next(r for r in tables["Component"][3:] if r[0] == "DesktopLink")
        self.assertEqual(desktop[4], "CREATE_DESKTOP = 1")
        launch = next(r for r in tables["CustomAction"][3:] if r[0] == "LaunchAI")
        self.assertIn(launch[2], files)
        self.assertEqual(tables["Media"][-1][1], "2")

    def test_unsafe_or_duplicate_file_names_are_rejected_before_packaging(self):
        self.pack["files"].append({"path": "../outside", "size": 0})
        with self.assertRaises(ValueError):
            self.sdk.prepare_tables(self.pack)
        self.pack["files"][-1]["path"] = "DUNGEONWORLDAI.EXE"
        with self.assertRaises(ValueError):
            self.sdk.prepare_tables(self.pack)


if __name__ == "__main__":
    unittest.main(verbosity=2)
