#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_binary_health import run_whale_binary_health_preflight
from cache_run_execution_test_support import CacheRunExecutionFixture
from run_cache_hit_regression import main


class CacheBinaryHealthTest(unittest.TestCase):
    def test_persists_shared_health_result(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root)
            output = repo / "evidence/health.json"
            health = {
                "schema_version": 1,
                "status": "pass",
                "run_validity": "valid",
                "whale_binary_sha256": "a" * 64,
                "findings": [],
            }

            def run(_command, _repo, _timeout):
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(json.dumps(health), encoding="utf-8")
                return type("Completed", (), {"returncode": 0})()

            with patch("cache_binary_health.run_captured_command", side_effect=run):
                result = run_whale_binary_health_preflight(repo, repo / "whale", output)

            self.assertEqual(result["status"], "passed")
            self.assertEqual(result["whale_binary_sha256"], "a" * 64)

    def test_rejects_attestation_failure_with_stable_code(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root)
            output = repo / "evidence/health.json"
            health = {
                "schema_version": 1,
                "status": "fail",
                "run_validity": "invalid_harness",
                "whale_binary_sha256": "b" * 64,
                "findings": [
                    {
                        "severity": "fail",
                        "stable_code": "whale_binary_attestation_invalid",
                    }
                ],
            }

            def run(_command, _repo, _timeout):
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(json.dumps(health), encoding="utf-8")
                return type("Completed", (), {"returncode": 3})()

            with patch("cache_binary_health.run_captured_command", side_effect=run):
                with self.assertRaisesRegex(
                    ValueError, "whale_binary_attestation_invalid"
                ):
                    run_whale_binary_health_preflight(repo, repo / "whale", output)


class CacheBinaryHealthRunnerTest(CacheRunExecutionFixture):
    def test_blocks_before_provider_route_and_authorization_claim(self) -> None:
        self.binary_health_mock.side_effect = ValueError(
            "Whale binary preflight failed before provider route: "
            "whale_binary_attestation_invalid"
        )
        argv = [
            "run_cache_hit_regression.py",
            "--repo-root",
            str(self.repo),
            "--proposal",
            str(self.proposal_path),
            "--authorization",
            str(self.authorization_path),
        ]

        with (
            patch("sys.argv", argv),
            patch(
                "run_cache_hit_regression.load_authorized_proposal",
                return_value=(
                    self.proposal,
                    self.authorization,
                    self.proposal_path,
                    self.authorization_path,
                ),
            ),
            self.assertRaisesRegex(SystemExit, "whale_binary_attestation_invalid"),
        ):
            main()

        self.route_preflight_mock.assert_not_called()
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"], [])


if __name__ == "__main__":
    unittest.main()
