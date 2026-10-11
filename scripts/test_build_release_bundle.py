import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

import build_release_bundle as bundle


class BuildArtifactsTests(unittest.TestCase):
    def build(self, target, paths):
        spec = bundle.BundleSpec("1.0.0-beta.3", bundle.SUPPORTED_TARGETS[target], "sysop")
        messages = [{"reason": "build-finished", "success": True}]
        messages.extend(
            {"reason": "compiler-artifact", "target": {"name": name}, "executable": path}
            for name, path in paths.items()
        )
        output = "\n".join(json.dumps(message) for message in messages)
        result = subprocess.CompletedProcess([], 0, stdout=output, stderr="")
        with patch.object(bundle, "detect_host_target", return_value=target), patch.object(
            bundle, "run", return_value=result
        ) as run:
            binaries = bundle.build_binaries(spec)
        self.assertIn("--message-format=json-render-diagnostics", run.call_args.args[0])
        return binaries

    def test_uses_reported_slot_paths_and_ignores_other_binaries(self):
        paths = {name: f"/cargo-slot/0/release/{name}" for name in ("nc-door", "nc-sysop")}
        result = self.build("x86_64-unknown-linux-gnu", {**paths, "nc-game": "/other/nc-game"})
        self.assertEqual(result, {name: Path(path) for name, path in paths.items()})

    def test_uses_reported_windows_executables(self):
        paths = {name: f"C:/build/release/{name}.exe" for name in ("nc-door", "nc-sysop")}
        self.assertEqual(
            self.build("x86_64-pc-windows-msvc", paths),
            {name: Path(path) for name, path in paths.items()},
        )

    def test_missing_artifact_fails_instead_of_using_stale_target(self):
        with self.assertRaisesRegex(SystemExit, "nc-door, nc-sysop"):
            self.build("x86_64-unknown-linux-gnu", {})

    def test_windows_x86_builds_std_and_records_the_compiler_actually_used(self):
        for target in ("i686-pc-windows-msvc", "i686-win7-windows-msvc"):
            with self.subTest(target=target):
                spec = bundle.BundleSpec("1.0.0-beta.3", bundle.SUPPORTED_TARGETS[target], "sysop")
                command = bundle.build_command(spec, ("nc-game", "nc-sysop"))
                self.assertEqual(command[:5], ["cargo", "+nightly", "build", "-Z", "build-std=std,panic_abort"])
                with patch.object(bundle, "capture", side_effect=["commit", "short", "rustc nightly"]) as capture:
                    info = bundle.build_info_text(spec)
                capture.assert_called_with(["rustc", "+nightly", "-V"])
                self.assertIn("rustc=rustc nightly", info)
                self.assertIn("build_std=std,panic_abort", info)

    def test_windows_x64_keeps_normal_build(self):
        spec = bundle.BundleSpec(
            "1.0.0-beta.3", bundle.SUPPORTED_TARGETS["x86_64-pc-windows-msvc"], "sysop"
        )
        command = bundle.build_command(spec, ("nc-game", "nc-sysop"))
        self.assertEqual(command[:3], ["cargo", "build", "--release"])
        self.assertNotIn("-Z", command)


if __name__ == "__main__":
    unittest.main()
