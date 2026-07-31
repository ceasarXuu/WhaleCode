#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from cache_run_ledger import (
    _lock_file,
    _unlock_file,
    atomic_write_json,
    claim_entry,
    settle_entry,
    store_entry,
)


class CacheRunLedgerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.path = Path(self.temp.name) / "ledger.json"
        self.path.write_text(
            json.dumps({"updated_at": None, "entries": []}) + "\n",
            encoding="utf-8",
        )
        self.entry = {
            "record_id": "WAR-1",
            "status": "planned",
            "authorization": {"id": "CBA-FIXTURE-001"},
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_authorization_can_be_claimed_only_once(self) -> None:
        claim_entry(self.path, self.entry)
        duplicate = {
            **self.entry,
            "record_id": "WAR-2",
        }
        with self.assertRaisesRegex(ValueError, "already been claimed"):
            claim_entry(self.path, duplicate)
        ledger = json.loads(self.path.read_text(encoding="utf-8"))
        self.assertEqual([item["record_id"] for item in ledger["entries"]], ["WAR-1"])

    def test_store_updates_only_the_claimed_record(self) -> None:
        claim_entry(self.path, self.entry)
        updated = {**self.entry, "status": "running"}
        store_entry(self.path, updated)
        ledger = json.loads(self.path.read_text(encoding="utf-8"))
        self.assertEqual(ledger["entries"][0]["status"], "running")

    def test_interrupted_replace_preserves_valid_ledger(self) -> None:
        before = self.path.read_text(encoding="utf-8")
        with (
            patch("cache_run_ledger.os.replace", side_effect=OSError("interrupted")),
            self.assertRaisesRegex(OSError, "interrupted"),
        ):
            claim_entry(self.path, self.entry)
        self.assertEqual(self.path.read_text(encoding="utf-8"), before)
        self.assertEqual(list(self.path.parent.glob(".*.tmp")), [])

    def test_partial_usage_is_never_reported_as_complete_cost(self) -> None:
        entry = {
            "status": "running",
            "execution": {},
            "tokens": {},
            "monetary_cost": {
                "pricing_snapshot": {
                    "cached_input_per_million": 0.0028,
                    "uncached_input_per_million": 0.14,
                    "output_per_million": 0.28,
                }
            },
            "evidence": {},
        }
        result = {
            "status": "partial",
            "started_at": "start",
            "ended_at": "end",
            "elapsed_seconds": 1.0,
            "actual_sample_runs": 2,
            "run_root": "target/run",
            "result_path": "benchmarks/result.json",
            "runner_exit_code": 3,
            "observations": [
                {
                    "provider_requests": 2,
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "uncached_input_tokens": 20,
                    "output_tokens": 10,
                }
            ],
        }
        settle_entry(entry, result)
        self.assertEqual(entry["monetary_cost"]["status"], "estimated_partial")
        self.assertEqual(entry["evidence"]["usage_evidence_status"], "partial")
        self.assertEqual(entry["execution"]["api_requests_evidence_status"], "partial")

    def test_atomic_result_write_replaces_complete_json(self) -> None:
        result = self.path.parent / "result.json"
        result.write_text('{"old": true}\n', encoding="utf-8")
        atomic_write_json(result, {"status": "completed", "requests": 2})
        self.assertEqual(
            json.loads(result.read_text(encoding="utf-8")),
            {"status": "completed", "requests": 2},
        )
        self.assertEqual(list(result.parent.glob(f".{result.name}.*.tmp")), [])

    def test_windows_lock_backend_locks_the_same_byte(self) -> None:
        lock = Mock()
        lock.tell.return_value = 0
        lock.fileno.return_value = 42
        backend = Mock(LK_LOCK=1, LK_UNLCK=2)
        with (
            patch("cache_run_ledger.fcntl", None),
            patch("cache_run_ledger.msvcrt", backend),
        ):
            _lock_file(lock)
            _unlock_file(lock)
        lock.write.assert_called_once_with("\0")
        backend.locking.assert_any_call(42, 1, 1)
        backend.locking.assert_any_call(42, 2, 1)


if __name__ == "__main__":
    unittest.main()
