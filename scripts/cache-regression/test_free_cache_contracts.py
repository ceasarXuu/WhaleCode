#!/usr/bin/env python3

from __future__ import annotations

import json
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

    def test_commands_do_not_scan_the_host_home(self) -> None:
        script = (
            "import os, pathlib; "
            "home=pathlib.Path(os.environ['HOME']); "
            "assert home.name.startswith('whale-cache-home-'); "
            "assert os.environ['USERPROFILE'] == str(home); "
            "assert pathlib.Path(os.environ['CARGO_HOME']).name == '.cargo'"
        )

        result = run_free_validation(
            self.repo,
            command_config(["python3", "-c", script]),
        )

        self.assertTrue(result["passed"])

    def test_commands_receive_the_repository_rust_stack_floor(self) -> None:
        script = "import os; assert os.environ['RUST_MIN_STACK'] == '8388608'"

        result = run_free_validation(
            self.repo,
            command_config(["python3", "-c", script]),
        )

        self.assertTrue(result["passed"])

    def test_failure_does_not_hide_later_independent_results(self) -> None:
        config = command_config(["python3", "-c", "raise SystemExit(9)"])
        config["commands"].append(
            {
                "id": "later",
                "cwd": ".",
                "argv": ["python3", "-c", "print('later evidence')"],
                "timeout_seconds": 10,
            }
        )

        result = run_free_validation(self.repo, config)

        self.assertFalse(result["passed"])
        self.assertEqual(
            [item["id"] for item in result["commands"]], ["fixture", "later"]
        )
        self.assertEqual(result["commands"][0]["exit_code"], 9)
        self.assertEqual(result["commands"][1]["status"], "pass")

    def test_timeout_is_a_failure(self) -> None:
        result = run_free_validation(
            self.repo,
            command_config(["python3", "-c", "import time; time.sleep(2)"], timeout=1),
        )

        self.assertFalse(result["passed"])
        self.assertTrue(result["commands"][0]["timed_out"])

    def test_missing_executable_is_a_structured_failure(self) -> None:
        result = run_free_validation(
            self.repo, command_config(["definitely-not-a-real-executable"])
        )

        self.assertFalse(result["passed"])
        self.assertIsNone(result["commands"][0]["exit_code"])
        self.assertIn(
            "failed to start command", result["commands"][0]["output_tail"][0]
        )

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

    def test_boolean_timeout_is_rejected(self) -> None:
        config = command_config(["python3", "-c", "pass"])
        config["commands"][0]["timeout_seconds"] = True

        with self.assertRaisesRegex(ValueError, "invalid timeout"):
            validate_free_validation(config)

    def write_snapshot(self, value: dict) -> None:
        snapshots = self.repo / "snapshots"
        snapshots.mkdir(exist_ok=True)
        (snapshots / "all__suite__fixture__stable.snap").write_text(
            "---\nsource: fixture.rs\n---\n" + json.dumps(value, indent=2) + "\n",
            encoding="utf-8",
        )

    def report_config(self, candidate: dict | None) -> dict:
        script = (
            "import json, os, pathlib; "
            "root=pathlib.Path(os.environ['WHALE_CACHE_CHANGE_REPORT_DIR']); "
            "root.mkdir(parents=True, exist_ok=True); "
        )
        if candidate is not None:
            script += (
                "(root/'stable.json').write_text("
                + repr(json.dumps(candidate))
                + ", encoding='utf-8')"
            )
        config = command_config(["python3", "-c", script])
        config["commands"][0]["change_report"] = {
            "type": "final_wire_snapshot_set",
            "baseline_globs": ["snapshots/*.snap"],
        }
        return config

    def test_snapshot_report_passes_only_for_unchanged_semantics(self) -> None:
        value = {"request_1": {"input": ["stable"]}}
        self.write_snapshot(value)

        result = run_free_validation(self.repo, self.report_config(value))

        self.assertTrue(result["passed"])
        report = result["commands"][0]["change_report"]
        self.assertEqual(report["status"], "unchanged")

    def test_snapshot_report_blocks_semantic_change_even_when_command_exits_zero(
        self,
    ) -> None:
        self.write_snapshot({"request_1": {"input": ["stable"]}})

        result = run_free_validation(
            self.repo,
            self.report_config({"request_1": {"input": ["changed"]}}),
        )

        self.assertFalse(result["passed"])
        report = result["commands"][0]["change_report"]
        self.assertEqual(report["status"], "changed")
        self.assertEqual(
            report["scenarios"][0]["first_difference"], "/request_1/input/0"
        )

    def test_snapshot_report_blocks_missing_candidate_as_uncomparable(self) -> None:
        self.write_snapshot({"request_1": {"input": ["stable"]}})

        result = run_free_validation(self.repo, self.report_config(None))

        self.assertFalse(result["passed"])
        self.assertEqual(
            result["commands"][0]["change_report"]["status"], "uncomparable"
        )

    def test_invalid_snapshot_report_config_is_rejected(self) -> None:
        config = command_config(["python3", "-c", "pass"])
        config["commands"][0]["change_report"] = {
            "type": "guess_from_logs",
            "baseline_globs": ["snapshots/*.snap"],
        }

        with self.assertRaisesRegex(ValueError, "invalid change report"):
            validate_free_validation(config)


if __name__ == "__main__":
    unittest.main()
