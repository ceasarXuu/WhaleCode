#!/usr/bin/env python3

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import qualify_candidate  # noqa: E402


class QualifyCandidateTests(unittest.TestCase):
    def test_candidate_target_is_independent_from_overlay_target(self) -> None:
        self.assertEqual(
            qualify_candidate.CANDIDATE_TARGET,
            "78c290807ce710180111df227df3b7a4fe845452",
        )

    def test_candidate_uses_codex_sandboxed_v8_artifacts(self) -> None:
        self.assertEqual(qualify_candidate.RUSTY_V8_VERSION, "150.4.0")
        self.assertEqual(
            qualify_candidate.RUSTY_V8_TARGET,
            "x86_64-unknown-linux-gnu",
        )
        self.assertEqual(
            qualify_candidate.RUSTY_V8_PROFILE,
            "ptrcomp_sandbox_release",
        )

    def test_environment_scrubs_proxy_variants(self) -> None:
        environment = qualify_candidate._qualification_environment(
            {
                "HTTP_PROXY": "upper",
                "https_proxy": "lower",
                "All_Proxy": "mixed",
                "NO_PROXY": "localhost",
                "CODEX_SANDBOX_NETWORK_DISABLED": "1",
                "UNRELATED": "preserved",
            }
        )

        self.assertEqual(environment["UNRELATED"], "preserved")
        self.assertFalse(
            any(
                key.lower() in qualify_candidate.PROXY_ENVIRONMENT_KEYS
                for key in environment
            )
        )
        self.assertNotIn("CODEX_SANDBOX_NETWORK_DISABLED", environment)
        for key, value in qualify_candidate.QUALIFICATION_ENVIRONMENT.items():
            self.assertEqual(environment[key], value)

    def test_environment_isolates_home_without_losing_rust_toolchains(self) -> None:
        isolated_home = Path("/candidate/.qualification/home")
        environment = qualify_candidate._qualification_environment(
            {
                "HOME": "/developer",
                "CARGO_HOME": "/cache/cargo",
                "UNRELATED": "preserved",
            },
            isolated_home=isolated_home,
        )

        self.assertEqual(environment["HOME"], str(isolated_home))
        self.assertEqual(environment["CARGO_HOME"], "/cache/cargo")
        self.assertEqual(environment["RUSTUP_HOME"], "/developer/.rustup")
        self.assertEqual(environment["UNRELATED"], "preserved")

    def test_package_tests_record_reproducible_environment(self) -> None:
        self.assertEqual(
            qualify_candidate.EVIDENCE_DIR,
            "docs/releases/v0.0.7/codex-upstream-sync/evidence/"
            "rust-v0.151.0/attempt-1-isolated-qualification",
        )
        self.assertEqual(
            qualify_candidate.ISOLATED_HOME_TEST_IDS,
            {"05-app-server-tests"},
        )

    def test_commands_use_supported_isolated_entrypoints(self) -> None:
        commands = dict(qualify_candidate.COMMANDS)

        self.assertEqual(
            commands["02-cli-check"],
            ("cargo", "check", "-p", "codex-cli", "--bin", "codex", "--offline"),
        )
        self.assertEqual(
            commands["03-code-mode-host-build"],
            (
                "cargo",
                "build",
                "--offline",
                "-p",
                "codex-code-mode-host",
                "--bin",
                "codex-code-mode-host",
            ),
        )
        for command_id in ("04-core-tests", "05-app-server-tests", "06-tui-tests"):
            self.assertIn("--no-fail-fast", commands[command_id])
        self.assertEqual(
            qualify_candidate.PREPARATION_COMMAND,
            ("cargo", "fetch"),
        )
        self.assertEqual(
            qualify_candidate.PACKAGE_TEST_IDS,
            {"04-core-tests", "05-app-server-tests", "06-tui-tests"},
        )
        self.assertEqual(
            qualify_candidate.TEST_SUPPORT_COMMAND,
            (
                "cargo",
                "build",
                "--offline",
                "-p",
                "codex-cli",
                "--bin",
                "codex",
                "-p",
                "codex-rmcp-client",
                "--bin",
                "test_stdio_server",
            ),
        )

    def test_output_normalization_removes_paths_and_trailing_whitespace(self) -> None:
        normalized = qualify_candidate._normalize_output(
            "/candidate/file  \n/runtime/file\n/repo/file\t\n/home/file\n",
            Path("/repo"),
            Path("/candidate"),
            Path("/runtime"),
        )

        self.assertEqual(
            normalized,
            "<candidate>/file\n<runtime>/file\n<repo>/file\n/home/file",
        )


if __name__ == "__main__":
    unittest.main()
