"""Local deployment contracts: no Docker/model/network required."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("lean_setup", ROOT / "scripts/setup.py")
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)
feedback = setup.feedback


class Deployment(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name).resolve()
        self.home = self.base / "home"
        self.home.mkdir()
        self.root = self.base / "checkout with spaces"
        for relative in ("target/release/claw-lean", "scripts/setup.py", "scripts/lean-container.py"):
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("#!/bin/sh\nexit 0\n")
            path.chmod(0o755)
        for skill in setup.SKILLS:
            source = ROOT / "skills" / skill
            import shutil
            shutil.copytree(source, self.root / "skills" / skill)
        self.mock_root = patch.object(setup, "ROOT", self.root)
        self.mock_root.start()

    def tearDown(self):
        self.mock_root.stop()
        self.temp.cleanup()

    def test_preview_has_no_writes_and_covers_five_harnesses(self):
        result = setup.install(self.home)
        self.assertEqual(list(self.home.iterdir()), [])
        self.assertEqual(len(result["harnesses"]), 5)
        self.assertFalse(result["applied"])

    def test_install_idempotent_and_references_self_contained(self):
        setup.install(self.home, True)
        second = setup.install(self.home, True)
        self.assertTrue(all(item["status"] == "unchanged" for item in second["entries"]))
        for root in setup.skill_roots(self.home):
            for skill in setup.SKILLS:
                manifest = root / skill / "references/manifest.json"
                self.assertEqual(json.loads(manifest.read_text())["schema_version"], 1)
                text = (root / skill / "SKILL.md").read_text()
                self.assertNotIn("../../", text)
        tool = self.home / ".local/bin/claw-lean"
        self.assertEqual(subprocess.run([str(tool)], check=False).returncode, 0)

    def test_modified_skill_conflict_prevents_all_writes(self):
        dest = self.home / ".hermes/skills/lean-review"
        dest.mkdir(parents=True)
        (dest / "SKILL.md").write_text("user edit")
        with self.assertRaisesRegex(ValueError, "not overwritten"):
            setup.install(self.home, True)
        self.assertEqual((dest / "SKILL.md").read_text(), "user edit")
        self.assertFalse((self.home / ".local").exists())
        self.assertFalse((self.home / ".agents").exists())

    def test_foreign_and_dangling_symlink_never_overwritten(self):
        dest = self.home / ".local/bin/claw-lean"
        dest.parent.mkdir(parents=True)
        dest.symlink_to(self.home / "absent")
        with self.assertRaisesRegex(ValueError, "Conflicting tool"):
            setup.install(self.home, True)
        self.assertTrue(dest.is_symlink())
        self.assertFalse((self.home / ".agents").exists())

    def test_symlink_parent_rejected(self):
        outside = self.base / "outside"
        outside.mkdir()
        (self.home / ".agents").symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "symlinked"):
            setup.install(self.home, True)
        self.assertEqual(list(outside.iterdir()), [])

    def test_partial_install_rolls_back_own_payloads(self):
        original = Path.symlink_to
        count = 0

        def injected(path, target, *args, **kwargs):
            nonlocal count
            count += 1
            if count == 2:
                raise OSError("injected failure")
            return original(path, target, *args, **kwargs)

        with patch.object(Path, "symlink_to", injected):
            with self.assertRaisesRegex(OSError, "injected"):
                setup.install(self.home, True)
        self.assertFalse(os.path.lexists(self.home / ".local/bin/claw-lean"))
        self.assertFalse((self.home / ".agents").exists())

    def test_unrelated_config_preserved(self):
        config = self.home / ".claude/settings.json"
        config.parent.mkdir()
        config.write_text('{"unchanged":true}')
        setup.install(self.home, True)
        self.assertEqual(config.read_text(), '{"unchanged":true}')


class SourceBuildPolicy(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.source = Path(self.temp.name).resolve() / "public-source"
        self.source.mkdir()
        for relative in ("CMakePresets.json", "stage0/src/CMakeLists.txt"):
            path = self.source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("synthetic fixture")
        self.run = subprocess.run
        self.run(["git", "init", "-q", str(self.source)], check=True)
        self.run(["git", "-C", str(self.source), "add", "."], check=True)
        self.run(["git", "-C", str(self.source), "-c", "user.name=Synthetic",
                  "-c", "user.email=synthetic@example.invalid", "commit", "-qm", "fixture"], check=True)

    def tearDown(self):
        self.temp.cleanup()

    def args(self):
        return argparse.Namespace(source=str(self.source), revision="HEAD", platform="linux/arm64", jobs=2)

    def test_only_git_archive_enters_named_context_and_is_cleaned(self):
        contexts = []

        def intercept(command, **kwargs):
            if command[0] != "docker":
                return self.run(command, **kwargs)
            self.assertIn("--platform=linux/arm64", command)
            context = Path(command[command.index("--build-context") + 1].split("=", 1)[1])
            contexts.append(context)
            files = {str(p.relative_to(context)) for p in context.rglob("*") if p.is_file()}
            self.assertEqual(files, {"CMakePresets.json", "stage0/src/CMakeLists.txt"})
            self.assertFalse((context / ".git").exists())
            self.assertNotIn(str(self.source), command)
            return subprocess.CompletedProcess(command, 0)

        with patch.object(feedback, "local_docker"), patch.object(setup.subprocess, "run", side_effect=intercept):
            setup.build_image(self.args())
        self.assertEqual(len(contexts), 1)
        self.assertFalse(contexts[0].exists())

    def test_dirty_or_untracked_checkout_never_starts_build(self):
        (self.source / "private-canary.env").write_text("synthetic exclusion canary")
        def intercept(command, **kwargs):
            self.assertNotEqual(command[0], "docker", "Dirty source must never reach a build")
            return self.run(command, **kwargs)

        with patch.object(feedback, "local_docker"), patch.object(setup.subprocess, "run", side_effect=intercept):
            with self.assertRaisesRegex(ValueError, "clean"):
                setup.build_image(self.args())

    def test_stable_build_uses_explicit_native_platform_and_allowlisted_root(self):
        args = self.args()
        args.source = None
        with patch.object(feedback, "local_docker"), patch.object(setup.subprocess, "run") as build:
            setup.build_image(args)
            command = build.call_args.args[0]
            self.assertIn("--platform=linux/arm64", command)
            self.assertEqual(command[-1], str(setup.ROOT))


class ContainerPolicy(unittest.TestCase):
    def args(self, **kwargs):
        values = dict(timeout_seconds=120, memory_mib=4096, cpus=2,
                      command="version", file=None, input_dir=None)
        values.update(kwargs)
        return argparse.Namespace(**values)

    def test_no_pull_network_or_privileges_and_immutable_identity(self):
        command = feedback.docker_args(self.args(), "sha256:" + "1" * 64,
                                       "linux/amd64", "synthetic")
        for flag in ("--pull=never", "--network=none", "--read-only", "--cap-drop=ALL",
                     "--security-opt=no-new-privileges", "--pids-limit=256", "--log-driver=none"):
            self.assertIn(flag, command)
        self.assertIn("--entrypoint=/usr/bin/timeout", command)
        self.assertNotIn("--privileged", command)
        self.assertNotIn("-t", command)
        self.assertNotIn("--mount", command)

    def test_invalid_mounts_filenames_and_resources(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp).resolve()
            (directory / "Good.lean").write_text("example : True := by trivial")
            good = dict(command="check", input_dir=str(directory), file="Good.lean")
            command = feedback.docker_args(self.args(**good), "immutable", "linux/amd64", "x")
            self.assertIn("/input/Good.lean", command)
            for change in ({"file": "../Good.lean"}, {"file": "-x.lean"},
                           {"input_dir": str(directory) + ",unsafe"},
                           {"input_dir": str(Path.home().resolve())},
                           {"timeout_seconds": 0}, {"memory_mib": 1}, {"cpus": 0}):
                with self.subTest(change=change), self.assertRaises(ValueError):
                    feedback.docker_args(self.args(**dict(good, **change)), "id", "linux/amd64", "x")

    def test_server_preserves_stdio_without_tty(self):
        with tempfile.TemporaryDirectory() as temp:
            command = feedback.docker_args(self.args(command="server", input_dir=str(Path(temp).resolve())),
                                           "id", "linux/amd64", "x")
        self.assertIn("-i", command)
        self.assertNotIn("-t", command)
        self.assertEqual(command[-1], "server")

    def test_remote_daemon_blocked_before_image_inspection(self):
        for env, endpoint in (("tcp://remote:2375", "unix:///local"),
                              ("unix:///local", "ssh://remote")):
            with self.subTest(env=env, endpoint=endpoint):
                with patch.dict(os.environ, {"DOCKER_HOST": env}), patch.object(feedback, "capture", return_value=endpoint):
                    with self.assertRaisesRegex(ValueError, "local Unix"):
                        feedback.resolve_image("image")

    def test_timeout_cleans_only_own_container_and_is_not_success(self):
        from unittest.mock import MagicMock
        child = MagicMock()
        child.wait.side_effect = [subprocess.TimeoutExpired("docker", 135), 0]
        child.poll.return_value = None
        with patch.object(feedback, "resolve_image", return_value=("id", "linux/amd64")), \
                patch.object(feedback.subprocess, "Popen", return_value=child), \
                patch.object(feedback.subprocess, "run") as cleanup:
            self.assertEqual(feedback.main(["version"]), 124)
            args = cleanup.call_args.args[0]
            self.assertEqual(args[:3], ["docker", "rm", "-f"])
            self.assertTrue(args[3].startswith("claw-lean-feedback-"))
            self.assertEqual(len(args), 4)
            child.kill.assert_called_once()


if __name__ == "__main__":
    unittest.main()
