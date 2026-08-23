#!/usr/bin/env python3

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import run_isolated_tests  # noqa: E402


class RunIsolatedTestsTests(unittest.TestCase):
    def test_environment_scrubs_host_state_and_sets_private_temp_root(self) -> None:
        with tempfile.TemporaryDirectory() as runtime:
            environment = run_isolated_tests._isolated_environment(Path(runtime))

        self.assertFalse(
            any(
                key.lower()
                in {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}
                for key in environment
            )
        )
        self.assertEqual(environment["TMPDIR"], f"{runtime}/tmp")
        self.assertEqual(environment["GIT_CEILING_DIRECTORIES"], f"{runtime}/tmp")
        self.assertEqual(environment["RUST_MIN_STACK"], "8388608")
        self.assertEqual(environment["NEXTEST_PROFILE"], "local")

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
            [["cargo", "build", "-p", "codex-rmcp-client", "--bin", "test_stdio_server"]],
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
                selected = run_isolated_tests._runtime_base(
                    {run_isolated_tests.RUNTIME_ROOT_ENV: str(contaminated)}
                )

        if Path("/dev/shm").is_dir():
            self.assertEqual(selected, Path("/dev/shm"))
        else:
            self.assertNotEqual(selected, contaminated)

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
