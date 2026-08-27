#!/usr/bin/env python3

import hashlib
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import run_isolated_tests  # noqa: E402


class RunIsolatedTestsTests(unittest.TestCase):
    def test_cached_rusty_v8_environment_requires_verified_pair(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            cache_base = Path(root)
            cache_dir = (
                cache_base
                / f"rusty-v8-v{run_isolated_tests.RUSTY_V8_VERSION}"
                / run_isolated_tests.RUSTY_V8_TARGET
            )
            cache_dir.mkdir(parents=True)
            profile = run_isolated_tests.RUSTY_V8_PROFILE
            target = run_isolated_tests.RUSTY_V8_TARGET
            archive_name = f"librusty_v8_{profile}_{target}.a.gz"
            binding_name = f"src_binding_{profile}_{target}.rs"
            checksum_name = f"rusty_v8_{profile}_{target}.sha256"
            artifacts = {archive_name: b"archive", binding_name: b"binding"}
            for name, content in artifacts.items():
                (cache_dir / name).write_bytes(content)
            (cache_dir / checksum_name).write_text(
                "".join(
                    f"{hashlib.sha256(content).hexdigest()}  {name}\n"
                    for name, content in artifacts.items()
                ),
                encoding="utf-8",
            )

            environment = run_isolated_tests._cached_rusty_v8_environment(cache_base)
            self.assertEqual(
                environment["RUSTY_V8_ARCHIVE"], str(cache_dir / archive_name)
            )
            self.assertEqual(
                environment["RUSTY_V8_SRC_BINDING_PATH"],
                str(cache_dir / binding_name),
            )
            (cache_dir / archive_name).write_bytes(b"tampered")
            self.assertEqual(
                run_isolated_tests._cached_rusty_v8_environment(cache_base),
                {},
            )

    def test_environment_scrubs_host_state_and_sets_private_temp_root(self) -> None:
        with tempfile.TemporaryDirectory() as runtime:
            environment = run_isolated_tests._isolated_environment(Path(runtime))

        self.assertFalse(
            any(
                key.lower() in {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}
                for key in environment
            )
        )
        self.assertEqual(environment["TMPDIR"], f"{runtime}/tmp")
        self.assertEqual(environment["GIT_CEILING_DIRECTORIES"], f"{runtime}/tmp")
        self.assertEqual(environment["RUST_MIN_STACK"], "8388608")
        self.assertEqual(environment["NEXTEST_PROFILE"], "local")

    def test_environment_preserves_explicit_rusty_v8_assets(self) -> None:
        with (
            tempfile.TemporaryDirectory() as runtime,
            patch.dict(
                os.environ,
                {
                    "RUSTY_V8_ARCHIVE": "/explicit/archive",
                    "RUSTY_V8_SRC_BINDING_PATH": "/explicit/binding",
                },
            ),
            patch.object(
                run_isolated_tests,
                "_cached_rusty_v8_environment",
                return_value={
                    "RUSTY_V8_ARCHIVE": "/cached/archive",
                    "RUSTY_V8_SRC_BINDING_PATH": "/cached/binding",
                },
            ),
        ):
            environment = run_isolated_tests._isolated_environment(Path(runtime))

        self.assertEqual(environment["RUSTY_V8_ARCHIVE"], "/explicit/archive")
        self.assertEqual(environment["RUSTY_V8_SRC_BINDING_PATH"], "/explicit/binding")

    def test_command_preserves_explicit_nextest_arguments(self) -> None:
        self.assertEqual(
            run_isolated_tests._command(["-p", "codex-core", "--lib"]),
            [
                "cargo",
                "nextest",
                "run",
                "--no-fail-fast",
                "-p",
                "codex-core",
                "--lib",
            ],
        )

    def test_core_scope_builds_stdio_runtime_helper(self) -> None:
        self.assertEqual(
            run_isolated_tests._runtime_helper_commands(["-p", "codex-core"]),
            [
                [
                    "cargo",
                    "build",
                    "-p",
                    "codex-rmcp-client",
                    "--bin",
                    "test_stdio_server",
                ],
                ["cargo", "build", "-p", "codex-code-mode-host"],
            ],
        )

    def test_app_server_scope_builds_code_mode_host(self) -> None:
        self.assertEqual(
            run_isolated_tests._runtime_helper_commands(["-p", "codex-app-server"]),
            [["cargo", "build", "-p", "codex-code-mode-host"]],
        )

    def test_non_core_scope_skips_stdio_runtime_helper(self) -> None:
        self.assertEqual(
            run_isolated_tests._runtime_helper_commands(
                ["--package=codex-login", "--package", "codex-protocol"]
            ),
            [],
        )

    def test_runtime_base_rejects_contaminated_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            contaminated = Path(root) / "contaminated"
            contaminated.mkdir()
            (contaminated / ".git").mkdir()
            with patch.object(
                run_isolated_tests,
                "_has_workspace_markers",
                side_effect=lambda path: path == contaminated,
            ):
                with self.assertRaisesRegex(
                    RuntimeError,
                    run_isolated_tests.RUNTIME_ROOT_ENV,
                ):
                    run_isolated_tests._runtime_base(
                        {run_isolated_tests.RUNTIME_ROOT_ENV: str(contaminated)}
                    )

    @unittest.skipUnless(Path("/var/tmp").is_dir(), "/var/tmp is unavailable")
    def test_runtime_base_prefers_sandbox_visible_posix_temp_root(self) -> None:
        with patch.object(
            run_isolated_tests,
            "_has_workspace_markers",
            return_value=False,
        ):
            selected = run_isolated_tests._runtime_base({})

        self.assertEqual(selected, Path("/var/tmp"))

    def test_runtime_base_honors_safe_explicit_directory(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            safe = Path(root)
            with patch.object(
                run_isolated_tests,
                "_has_workspace_markers",
                return_value=False,
            ):
                selected = run_isolated_tests._runtime_base(
                    {run_isolated_tests.RUNTIME_ROOT_ENV: root}
                )

        self.assertEqual(selected, safe.resolve())

    def test_main_requires_an_explicit_scope(self) -> None:
        self.assertEqual(run_isolated_tests.main([]), 2)


if __name__ == "__main__":
    unittest.main()
