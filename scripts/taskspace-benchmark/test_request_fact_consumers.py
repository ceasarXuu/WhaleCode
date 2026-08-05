#!/usr/bin/env python3
"""Contract tests for the request fact consumer inventory gate."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-request-fact-consumers.py")
SPEC = importlib.util.spec_from_file_location("request_fact_consumer_gate", SCRIPT)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class ConsumerInventoryTests(unittest.TestCase):
    def test_repository_inventory_is_complete(self) -> None:
        repo = Path(__file__).resolve().parents[2]
        inventory = GATE.load_inventory(
            Path(__file__).with_name("request-fact-consumers.json")
        )
        self.assertEqual(GATE.compare(GATE.discover(repo), inventory), [])

    def test_unknown_reader_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            target = repo / "scripts/taskspace-benchmark/lib/new-reader.py"
            target.parent.mkdir(parents=True)
            target.write_text('STATUS = "payload_captured"\n', encoding="utf-8")
            errors = GATE.compare(GATE.discover(repo), {})
        self.assertEqual(
            errors,
            [
                "unclassified request fact reader: "
                "scripts/taskspace-benchmark/lib/new-reader.py ['local_attempt']"
            ],
        )

    def test_test_support_does_not_trigger_gate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            target = repo / "scripts/cache-regression/test_example.py"
            target.parent.mkdir(parents=True)
            target.write_text('STATUS = "payload_captured"\n', encoding="utf-8")
            self.assertEqual(GATE.discover(repo), {})


if __name__ == "__main__":
    unittest.main()
