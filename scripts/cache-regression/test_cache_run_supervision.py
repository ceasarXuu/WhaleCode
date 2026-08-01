#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from cache_run_supervision import persist_final_settlement


class CacheRunSupervisionTest(unittest.TestCase):
    def setUp(self) -> None:
        self.entry = {"record_id": "WAR-FIXTURE"}
        self.result = {"status": "completed", "runner_exit_code": 0}
        self.result_path = Path("result.json")
        self.ledger_path = Path("ledger.json")

    def test_result_write_interrupt_retries_as_failed_settlement(self) -> None:
        with (
            patch(
                "cache_run_supervision.atomic_write_json",
                side_effect=[KeyboardInterrupt(), None],
            ) as write_result,
            patch("cache_run_supervision.settle_entry") as settle,
            patch("cache_run_supervision.store_entry") as store,
        ):
            persist_final_settlement(
                self.entry, self.result, self.result_path, self.ledger_path
            )
        self.assertEqual(write_result.call_count, 2)
        settle.assert_called_once_with(self.entry, self.result)
        store.assert_called_once_with(self.ledger_path, self.entry)
        self.assertEqual(self.result["status"], "failed")
        self.assertEqual(self.result["runner_exit_code"], 3)
        self.assertIn("KeyboardInterrupt", self.result["finalization_error"])

    def test_ledger_write_error_retries_entire_failed_settlement(self) -> None:
        store = Mock(side_effect=[OSError("ledger write failed"), None])
        with (
            patch("cache_run_supervision.atomic_write_json") as write_result,
            patch("cache_run_supervision.settle_entry") as settle,
            patch("cache_run_supervision.store_entry", store),
        ):
            persist_final_settlement(
                self.entry, self.result, self.result_path, self.ledger_path
            )
        self.assertEqual(write_result.call_count, 2)
        self.assertEqual(settle.call_count, 2)
        self.assertEqual(store.call_count, 2)
        self.assertEqual(self.result["status"], "failed")
        self.assertIn("ledger write failed", self.result["finalization_error"])


if __name__ == "__main__":
    unittest.main()
