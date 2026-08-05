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

    def test_canonical_consumers_are_discovered(self) -> None:
        samples = {
            "artifact.py": 'PATH = "request-facts.json"\n',
            "import.py": "from request_facts import build_request_facts\n",
            "wrapper.ps1": "Invoke-TaskspaceRequestFactsGenerator -WireTracePath $Path\n",
            "summary.py": 'count = summary["boundary_request_count"]\n',
        }
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / "scripts/taskspace-benchmark/lib"
            root.mkdir(parents=True)
            for name, content in samples.items():
                (root / name).write_text(content, encoding="utf-8")
            discovered = GATE.discover(repo)
        self.assertEqual(len(discovered), len(samples))
        self.assertTrue(all(sources == {"canonical"} for sources in discovered.values()))

    def test_test_support_does_not_trigger_gate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            target = repo / "scripts/cache-regression/test_example.py"
            target.parent.mkdir(parents=True)
            target.write_text('STATUS = "payload_captured"\n', encoding="utf-8")
            self.assertEqual(GATE.discover(repo), {})


if __name__ == "__main__":
    unittest.main()
