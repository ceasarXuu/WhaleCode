from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

from generate_replay_ledger import cutover_batch, decide, lineage  # noqa: E402


class ReplayDecisionTests(unittest.TestCase):
    def test_taskspace_takes_priority_over_brand(self) -> None:
        self.assertEqual(
            "batch-4-taskspace-multi-agent",
            cutover_batch({"brand_home", "taskspace_domain"}, None),
        )

    def test_generated_artifact_is_final_batch(self) -> None:
        self.assertEqual(
            "batch-5-generated-release",
            cutover_batch({"provider_model"}, "config-schema"),
        )

    def test_identical_target_is_adopted(self) -> None:
        overlay = {
            "baseline_sha256": "a",
            "current_sha256": "b",
            "categories": ["runtime_utilities"],
        }
        delta = {"target_sha256": "b"}
        self.assertEqual("adopt-upstream", decide(overlay, delta, None)[0])

    def test_protected_domain_is_semantically_adapted(self) -> None:
        overlay = {
            "baseline_sha256": "a",
            "current_sha256": "b",
            "categories": ["provider_transport"],
        }
        self.assertEqual("adapt-semantically", decide(overlay, None, None)[0])

    def test_known_schema_has_lineage(self) -> None:
        value = lineage("app-server-json-schema")
        self.assertIsNotNone(value)
        self.assertEqual("just write-app-server-schema", value["command"])


if __name__ == "__main__":
    unittest.main()
