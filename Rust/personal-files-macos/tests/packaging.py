"""Synthetic packaging failures; never build, sign, install or launch a user's app."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


PUBLISHER = str(Path(sys.argv.pop(1)).resolve())
BUILD_SCRIPT = Path(sys.argv.pop(1)).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parents[1] / "build.sh"


def tree(path):
    return {str(p.relative_to(path)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in path.rglob("*") if p.is_file()}


class Packaging(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="personal-files packaging ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.project = self.root / "project with spaces"
        (self.project / "assets").mkdir(parents=True)
        shutil.copy2(BUILD_SCRIPT, self.project / "build.sh")
        shutil.copy2(Path(__file__).resolve().parents[1] / "assets/Info.plist", self.project / "assets/Info.plist")
        self.home = self.root / "home with spaces"
        self.cache = self.home / "Library/Caches/personal-files-macos"
        self.app = self.cache / "Personal Files.app"
        (self.app / "Contents/MacOS").mkdir(parents=True)
        (self.app / "Contents/MacOS/old").write_text("working old app")
        (self.app / "old-only-resource").write_text("must not merge")
        self.before = tree(self.app)
        self.tools = self.root / "tools"
        self.tools.mkdir()
        self.env = dict(os.environ, HOME=str(self.home),
                        CARGO_TARGET_DIR=str(self.root / "target with spaces"),
                        PATH=str(self.tools) + ":" + os.environ["PATH"],
                        TEST_REAL_PUBLISHER=PUBLISHER, TEST_APP=str(self.app),
                        TEST_LOG=str(self.root / "log"))
        self.env.pop("PERSONAL_FILES_CACHE", None)
        self.env.pop("TEST_FAIL", None)
        self.script("rustc", "#!/bin/bash\nprintf 'host: aarch64-apple-darwin\\n'\n")
        self.script("cargo", '''#!/usr/bin/env python3
import os, pathlib, sys
if os.environ.get("TEST_FAIL") == "build": sys.exit(42)
binary=pathlib.Path(os.environ["CARGO_TARGET_DIR"])/"aarch64-apple-darwin/release/personal-files-macos"
binary.parent.mkdir(parents=True,exist_ok=True)
binary.write_text(''' + repr('''#!/bin/bash
if [[ "$1" == --install-bundle && "$2" == *"/Personal Files.app" && "$2" != "$TEST_APP" ]]; then
  if [[ "${TEST_FAIL:-}" == publish ]]; then exit 42; fi
  if [[ "${TEST_FAIL:-}" == concurrent ]]; then
    mkdir -p "$3"
    printf 'concurrent app' > "$3/concurrent"
    exit 42
  fi
fi
exec "$TEST_REAL_PUBLISHER" "$@"
''') + ''')
binary.chmod(0o755)
''')
        self.script("cp", '''#!/usr/bin/env python3
import os,sys
if os.environ.get("TEST_FAIL") == "copy" and sys.argv[1].endswith("Info.plist"): sys.exit(42)
os.execv("/bin/cp", ["cp", *sys.argv[1:]])
''')
        self.script("plutil", '''#!/usr/bin/env python3
import os,sys
if os.environ.get("TEST_FAIL") == "plist": sys.exit(42)
os.execv("/usr/bin/plutil", ["plutil", *sys.argv[1:]])
''')
        self.script("codesign", '''#!/usr/bin/env python3
import os,sys
operation="verify" if "--verify" in sys.argv else "sign"
with open(os.environ["TEST_LOG"],"a") as log: log.write(operation+"\\n")
if os.environ.get("TEST_FAIL") == operation: sys.exit(42)
''')

    def script(self, name, content):
        path = self.tools / name
        path.write_text(content)
        path.chmod(0o755)

    def build(self, failure=None):
        if failure:
            self.env["TEST_FAIL"] = failure
        return subprocess.run(["/bin/bash", str(self.project / "build.sh")], env=self.env,
                              capture_output=True, text=True, timeout=30)

    def test_success_replaces_bundle_without_merging_and_checks_signature_first(self):
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / "Contents/MacOS/personal-files-macos").is_file())
        self.assertFalse((self.app / "old-only-resource").exists())
        self.assertEqual((self.root / "log").read_text(), "sign\nverify\n")
        self.assertFalse(list(self.cache.glob(".bundle.*")))

    def test_build_failure_preserves_previous_app(self):
        self.assert_failure_preserves("build")

    def test_copy_failure_preserves_previous_app(self):
        self.assert_failure_preserves("copy")

    def test_plist_failure_preserves_previous_app(self):
        self.assert_failure_preserves("plist")

    def test_sign_failure_preserves_previous_app(self):
        self.assert_failure_preserves("sign")

    def test_signature_verification_failure_preserves_previous_app(self):
        self.assert_failure_preserves("verify")

    def test_publication_failure_restores_previous_app(self):
        self.assert_failure_preserves("publish")

    def assert_failure_preserves(self, failure):
        result = self.build(failure)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(tree(self.app), self.before, result.stderr)

    def test_concurrent_app_is_not_overwritten_and_previous_app_is_recoverable(self):
        result = self.build("concurrent")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.app / "concurrent").read_text(), "concurrent app")
        previous = list(self.cache.glob(".bundle.*/previous.app"))
        self.assertEqual(len(previous), 1, result.stderr)
        self.assertEqual(tree(previous[0]), self.before)
        self.assertIn("Previous cached app retained for recovery:", result.stderr)

    def test_cache_override_and_first_build(self):
        cache = self.root / "custom cache with spaces"
        self.env["PERSONAL_FILES_CACHE"] = str(cache)
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((cache / "Personal Files.app/Contents/MacOS/personal-files-macos").is_file())
        self.assertEqual(tree(self.app), self.before)


if __name__ == "__main__":
    unittest.main()
