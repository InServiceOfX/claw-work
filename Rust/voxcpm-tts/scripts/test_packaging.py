"""Installation boundaries using existing debug binaries; never touches home installs."""
from pathlib import Path
import hashlib
import os
import plistlib
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.uname().sysname == "Darwin", "macOS packaging")
class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="voxcpm package ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.prefix = self.base / "cli with spaces"
        self.apps = self.base / "apps with spaces"
        self.args = [str(ROOT / "scripts/install.sh"), "--skip-build", "--debug", "--offline",
                     "--prefix", str(self.prefix), "--app-dir", str(self.apps)]

    def run_install(self, *extra):
        return subprocess.run(self.args + list(extra), capture_output=True, text=True, timeout=30)

    def test_existing_cli_is_preserved_without_force(self):
        cli = self.prefix / "bin/voxcpm-tts"
        cli.parent.mkdir(parents=True)
        cli.write_text("existing CLI sentinel")
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Refusing to overwrite", result.stderr)
        self.assertEqual(cli.read_text(), "existing CLI sentinel")
        self.assertFalse(self.apps.exists())

    def test_existing_app_is_preserved_before_cli_install(self):
        bundle = self.apps / "VoxCPM TTS.app"
        bundle.mkdir(parents=True)
        sentinel = bundle / "keep.txt"
        sentinel.write_text("existing app sentinel")
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sentinel.read_text(), "existing app sentinel")
        self.assertFalse((self.prefix / "bin/voxcpm-tts").exists())

    def test_offline_fetch_fails_before_any_write(self):
        result = self.run_install("--fetch")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be combined", result.stderr)
        self.assertFalse(self.prefix.exists())
        self.assertFalse(self.apps.exists())

    def test_bundle_signature_allowlist_and_force(self):
        if not (ROOT / "target/debug/voxcpm-tts-app").is_file():
            self.skipTest("build debug binaries with --features gui first")
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        bundle = self.apps / "VoxCPM TTS.app"
        relative_files = {str(p.relative_to(bundle)) for p in bundle.rglob("*") if p.is_file()}
        self.assertEqual(relative_files, {"Contents/Info.plist", "Contents/MacOS/voxcpm-tts-app",
                                         "Contents/_CodeSignature/CodeResources"})
        metadata = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
        self.assertEqual(metadata["CFBundleExecutable"], "voxcpm-tts-app")
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(bundle)], check=True)
        cli = self.prefix / "bin/voxcpm-tts"
        digest = hashlib.sha256(cli.read_bytes()).digest()
        refused = self.run_install()
        self.assertNotEqual(refused.returncode, 0)
        self.assertEqual(hashlib.sha256(cli.read_bytes()).digest(), digest)
        forced = self.run_install("--force")
        self.assertEqual(forced.returncode, 0, forced.stderr)
        self.assertEqual(hashlib.sha256(cli.read_bytes()).digest(), digest)
        self.assertFalse(list(self.apps.glob(".voxcpm-*")))
        self.assertFalse(list(cli.parent.glob(".voxcpm-*")))


if __name__ == "__main__":
    unittest.main()
