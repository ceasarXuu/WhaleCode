#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_run_ledger import claim_entry, store_entry


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


if __name__ == "__main__":
    unittest.main()
