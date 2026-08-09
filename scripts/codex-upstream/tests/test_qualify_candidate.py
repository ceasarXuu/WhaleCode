#!/usr/bin/env python3

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import qualify_candidate  # noqa: E402


class QualifyCandidateTests(unittest.TestCase):
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
            commands["04-code-mode-host-build"],
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
        for command_id in ("03-core-tests", "05-app-server-tests", "06-tui-tests"):
            self.assertIn("--no-fail-fast", commands[command_id])

    def test_output_normalization_removes_paths_and_trailing_whitespace(self) -> None:
        normalized = qualify_candidate._normalize_output(
            "/candidate/file  \n/repo/file\t\n/home/file\n",
            Path("/repo"),
            Path("/candidate"),
        )

        self.assertEqual(normalized, "<candidate>/file\n<repo>/file\n/home/file")


if __name__ == "__main__":
    unittest.main()
