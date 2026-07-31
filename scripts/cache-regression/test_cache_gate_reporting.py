#!/usr/bin/env python3

from __future__ import annotations

import unittest
from contextlib import redirect_stdout
from io import StringIO

from check_cache_regression_gate import (
    classify_free_validation,
    is_promotion_transition,
    print_free_validation_failure,
)


class CacheGateReportingTest(unittest.TestCase):
    def test_discovery_state_does_not_infer_product_acceptance(self) -> None:
        validation = {
            "passed": False,
            "commands": [{"change_report": {"status": "changed"}}],
        }
        self.assertEqual(classify_free_validation(validation, True), "changed")
        validation["commands"][0]["change_report"]["status"] = "uncomparable"
        self.assertEqual(classify_free_validation(validation, True), "uncomparable")

    def test_revalidation_promotion_requires_exact_empty_scenario_set(self) -> None:
        accepted = {"valid": True, "accepted_scenario_paths": []}
        self.assertTrue(
            is_promotion_transition(
                baseline_changed=True,
                semantic_baselines=[],
                accepted_validation=accepted,
                policy_changes=[],
                sensitive_changes=[],
            )
        )
        self.assertFalse(
            is_promotion_transition(
                baseline_changed=True,
                semantic_baselines=["snapshots/unapproved.snap"],
                accepted_validation=accepted,
                policy_changes=[],
                sensitive_changes=[],
            )
        )

    def test_structured_change_output_replaces_command_log_noise(self) -> None:
        validation = {
            "commands": [
                {
                    "id": "wire",
                    "status": "fail",
                    "exit_code": 1,
                    "timed_out": False,
                    "output_tail": ["noisy command log"],
                    "change_report": {
                        "status": "changed",
                        "scenario_count": 1,
                        "changed_scenario_count": 1,
                        "uncomparable_scenario_count": 0,
                        "scenarios": [
                            {
                                "scenario_id": "standard",
                                "status": "changed",
                                "first_difference": "/request_2/input/3",
                                "before_payload_sha256": "a" * 64,
                                "after_payload_sha256": "b" * 64,
                            }
                        ],
                    },
                }
            ]
        }
        stream = StringIO()
        with redirect_stdout(stream):
            print_free_validation_failure(validation)
        rendered = stream.getvalue()
        self.assertIn("standard: changed", rendered)
        self.assertIn("/request_2/input/3", rendered)
        self.assertNotIn("noisy command log", rendered)


if __name__ == "__main__":
    unittest.main()
