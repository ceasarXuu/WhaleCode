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
            "be6e8eac029b183056b7e4402879f15d2c85f61b",
        )

    def test_environment_scrubs_proxy_variants(self) -> None:
        environment = qualify_candidate._qualification_environment(
            {
                "HTTP_PROXY": "upper",
                "https_proxy": "lower",
                "All_Proxy": "mixed",
                "NO_PROXY": "localhost",
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
        for key, value in qualify_candidate.QUALIFICATION_ENVIRONMENT.items():
            self.assertEqual(environment[key], value)

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

    def test_output_normalization_removes_paths_and_trailing_whitespace(self) -> None:
        normalized = qualify_candidate._normalize_output(
            "/candidate/file  \n/repo/file\t\n/home/file\n",
            Path("/repo"),
            Path("/candidate"),
        )

        self.assertEqual(normalized, "<candidate>/file\n<repo>/file\n/home/file")


if __name__ == "__main__":
    unittest.main()
