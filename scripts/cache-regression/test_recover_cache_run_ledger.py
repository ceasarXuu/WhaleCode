#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cache_evidence import RESULT_SCHEMA_VERSION
from cache_surface import write_json
from recover_cache_run_ledger import mark_unsettled, recover


class RecoverCacheRunLedgerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        self.ledger = self.repo / "benchmarks/whale-agent-run-ledger.json"
        self.result = self.repo / "benchmarks/cache-regression/results/WAR-1.json"
        entry = {
            "record_id": "WAR-1",
            "status": "running",
            "started_at": "2026-08-01T00:00:00+00:00",
            "ended_at": None,
            "elapsed_calendar_seconds": None,
            "authorization": {"id": "CBA-FIXTURE-001"},
            "execution": {"actual_sample_runs": 1, "api_requests": 0},
            "tokens": {},
            "monetary_cost": {
                "pricing_snapshot": {
                    "currency": "USD",
                    "cached_input_per_million": 0.0028,
                    "uncached_input_per_million": 0.14,
                    "output_per_million": 0.28,
                }
            },
            "evidence": {},
        }
        write_json(self.ledger, {"updated_at": None, "entries": [entry]})
        result = {
            "schema_version": RESULT_SCHEMA_VERSION,
            "record_id": "WAR-1",
            "status": "completed",
            "started_at": "2026-08-01T00:00:00+00:00",
            "ended_at": "2026-08-01T00:00:02+00:00",
            "elapsed_seconds": 2.0,
            "result_path": self.result.relative_to(self.repo).as_posix(),
            "runner_exit_code": 0,
            "run_root": "target/run",
            "actual_sample_runs": 1,
            "observations": [
                {
                    "provider_requests": 2,
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "uncached_input_tokens": 20,
                    "output_tokens": 10,
                }
            ],
            "attempts": [{"provider_boundary_request_count": 2}],
        }
        write_json(self.result, result)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_recovers_and_is_idempotent(self) -> None:
        self.assertEqual(recover(self.repo, self.ledger, self.result), "settled")
        self.assertEqual(
            recover(self.repo, self.ledger, self.result), "already_settled"
        )
        entry = json.loads(self.ledger.read_text(encoding="utf-8"))["entries"][0]
        self.assertEqual(entry["status"], "settled")
        self.assertEqual(entry["execution"]["api_requests"], 2)
        self.assertEqual(entry["evidence"]["usage_evidence_status"], "complete")

    def test_incomplete_run_can_be_explicitly_marked_unsettled(self) -> None:
        self.result.unlink()
        self.assertEqual(
            mark_unsettled(self.ledger, "WAR-1", "runner crashed before result"),
            "unsettled",
        )
        self.assertEqual(
            mark_unsettled(self.ledger, "WAR-1", "same audit"),
            "already_unsettled",
        )
        entry = json.loads(self.ledger.read_text(encoding="utf-8"))["entries"][0]
        self.assertEqual(entry["status"], "unsettled")
        self.assertEqual(entry["monetary_cost"]["status"], "unavailable")
        self.assertEqual(entry["evidence"]["outcome"], "unsettled")
        self.assertEqual(
            entry["evidence"]["recovery_reason"], "runner crashed before result"
        )


if __name__ == "__main__":
    unittest.main()
