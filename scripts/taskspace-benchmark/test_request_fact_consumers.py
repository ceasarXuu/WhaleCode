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

    def test_r7_observer_does_not_rebuild_terminal_or_usage_facts(self) -> None:
        repo = Path(__file__).resolve().parents[2]
        source = (
            repo
            / "scripts/taskspace-benchmark/lib/r7-request-observability.ps1"
        ).read_text(encoding="utf-8")
        self.assertIn("Invoke-TaskspaceRequestFactsGenerator", source)
        self.assertNotIn("provider.chat_wire_request_terminal", source)
        self.assertNotIn("terminal_logical_request_id", source)
        self.assertNotIn("terminal_attempt_seq", source)

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
            "usage-helper.ps1": "$value = $Facts.summary.usage\n",
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

    def test_canonical_import_dependency_is_discovered(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / "scripts/taskspace-benchmark"
            root.mkdir(parents=True)
            (root / "request_facts.py").write_text(
                "from usage_helper import summarize\n", encoding="utf-8"
            )
            (root / "usage_helper.py").write_text(
                "def summarize(rows):\n    return len(rows)\n", encoding="utf-8"
            )
            discovered = GATE.discover(repo)
        self.assertEqual(discovered["scripts/taskspace-benchmark/usage_helper.py"], {"canonical"})

    def test_canonical_terminal_status_dependency_is_classified(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            target = repo / "scripts/cache-regression/cache_usage_contract.py"
            target.parent.mkdir(parents=True)
            target.write_text(
                "from request_facts import build_request_facts_from_events\n"
                'status = row["terminal_status"] == "response_completed"\n',
                encoding="utf-8",
            )
            discovered = GATE.discover(repo)
        self.assertEqual(
            discovered["scripts/cache-regression/cache_usage_contract.py"],
            {"canonical", "terminal"},
        )


if __name__ == "__main__":
    unittest.main()
