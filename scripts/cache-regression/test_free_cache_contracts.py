#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from free_cache_contracts import run_free_validation, validate_free_validation


def command_config(argv: list[str], *, timeout: int = 10) -> dict:
    return {
        "semantic_baseline_globs": ["snapshots/*.snap"],
        "commands": [
            {
                "id": "fixture",
                "cwd": ".",
                "argv": argv,
                "timeout_seconds": timeout,
            }
        ],
    }


class FreeCacheContractsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_success_records_structured_evidence(self) -> None:
        result = run_free_validation(
            self.repo,
            command_config(["python3", "-c", "print('wire stable')"]),
        )

        self.assertTrue(result["passed"])
        self.assertEqual(result["commands"][0]["status"], "pass")
        self.assertEqual(result["commands"][0]["output_tail"], ["wire stable"])

    def test_failure_stops_remaining_commands(self) -> None:
        config = command_config(["python3", "-c", "raise SystemExit(9)"])
        config["commands"].append(
            {
                "id": "must_not_run",
                "cwd": ".",
                "argv": ["python3", "-c", "print('unexpected')"],
                "timeout_seconds": 10,
            }
        )

        result = run_free_validation(self.repo, config)

        self.assertFalse(result["passed"])
        self.assertEqual(len(result["commands"]), 1)
        self.assertEqual(result["commands"][0]["exit_code"], 9)

    def test_timeout_is_a_failure(self) -> None:
        result = run_free_validation(
            self.repo,
            command_config(
                ["python3", "-c", "import time; time.sleep(2)"], timeout=1
            ),
        )

        self.assertFalse(result["passed"])
        self.assertTrue(result["commands"][0]["timed_out"])

    def test_missing_executable_is_a_structured_failure(self) -> None:
        result = run_free_validation(
            self.repo, command_config(["definitely-not-a-real-executable"])
        )

        self.assertFalse(result["passed"])
        self.assertIsNone(result["commands"][0]["exit_code"])
        self.assertIn("failed to start command", result["commands"][0]["output_tail"][0])

    def test_absolute_cwd_is_rejected(self) -> None:
        config = command_config(["python3", "-c", "pass"])
        config["commands"][0]["cwd"] = "/tmp"

        with self.assertRaisesRegex(ValueError, "invalid cwd"):
            validate_free_validation(config)

    def test_duplicate_command_id_is_rejected(self) -> None:
        config = command_config(["python3", "-c", "pass"])
        config["commands"].append(dict(config["commands"][0]))

        with self.assertRaisesRegex(ValueError, "duplicate"):
            validate_free_validation(config)


if __name__ == "__main__":
    unittest.main()
