#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from cache_payload_contract import (
    compare_evidence,
    compare_snapshot_set,
    load_policy,
    validate_policy,
)


REPO = Path(__file__).resolve().parents[2]
POLICY_PATH = REPO / "benchmarks/cache-regression/final-wire-comparison-policy.json"


def evidence() -> dict:
    return {
        "provider_identity": {
            "provider_id": "deepseek",
            "wire_api": "responses",
            "endpoint_path": "/v1/responses",
        },
        "raw_body_sha256": "raw-a",
        "structured_body": {
            "model": "deepseek-v4-flash",
            "instructions": "base",
            "input": [
                {"type": "message", "role": "developer", "content": "policy"},
                {"type": "message", "role": "user", "content": "task"},
            ],
            "tools": [
                {"type": "function", "name": "exec", "parameters": {}},
                {"type": "function", "name": "read", "parameters": {}},
            ],
            "tool_choice": "auto",
        },
    }


class CachePayloadContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = load_policy(POLICY_PATH)
        self.before = evidence()

    def assert_cache_change(self, mutate) -> dict:
        after = copy.deepcopy(self.before)
        mutate(after)
        result = compare_evidence(self.before, after, self.policy)
        self.assertTrue(result["cache_relevant_changed"])
        return result

    def test_identical_evidence_is_unchanged(self) -> None:
        result = compare_evidence(self.before, copy.deepcopy(self.before), self.policy)
        self.assertFalse(result["cache_relevant_changed"])
        self.assertFalse(result["raw_body_changed"])

    def test_raw_only_change_is_diagnostic_not_semantic(self) -> None:
        after = copy.deepcopy(self.before)
        after["raw_body_sha256"] = "raw-b"
        result = compare_evidence(self.before, after, self.policy)
        self.assertFalse(result["cache_relevant_changed"])
        self.assertTrue(result["raw_only_change"])

    def test_input_role_order_and_content_are_protected(self) -> None:
        role = self.assert_cache_change(
            lambda value: value["structured_body"]["input"][0].update(role="user")
        )
        order = self.assert_cache_change(
            lambda value: value["structured_body"]["input"].reverse()
        )
        content = self.assert_cache_change(
            lambda value: value["structured_body"]["input"][1].update(content="changed")
        )
        self.assertEqual(role["first_body_difference"], "/input/0/role")
        self.assertEqual(order["first_body_difference"], "/input/0/content")
        self.assertEqual(content["first_body_difference"], "/input/1/content")

    def test_instructions_are_protected(self) -> None:
        result = self.assert_cache_change(
            lambda value: value["structured_body"].update(instructions="changed")
        )
        self.assertEqual(result["first_body_difference"], "/instructions")

    def test_tool_schema_and_order_are_protected(self) -> None:
        schema = self.assert_cache_change(
            lambda value: value["structured_body"]["tools"][0].update(
                parameters={"type": "object"}
            )
        )
        order = self.assert_cache_change(
            lambda value: value["structured_body"]["tools"].reverse()
        )
        self.assertIn("/tools/0/parameters", schema["first_body_difference"])
        self.assertEqual(order["first_body_difference"], "/tools/0/name")

    def test_tool_choice_model_and_unknown_fields_are_protected(self) -> None:
        self.assert_cache_change(
            lambda value: value["structured_body"].update(tool_choice="required")
        )
        self.assert_cache_change(
            lambda value: value["structured_body"].update(model="deepseek-v4-pro")
        )
        unknown = self.assert_cache_change(
            lambda value: value["structured_body"].update(new_provider_field=True)
        )
        self.assertEqual(unknown["first_body_difference"], "/new_provider_field")

    def test_provider_identity_is_protected(self) -> None:
        for field in self.policy["protected_provider_identity_fields"]:
            result = self.assert_cache_change(
                lambda value, field=field: value["provider_identity"].update(
                    {field: "changed"}
                )
            )
            self.assertEqual(result["provider_identity_differences"], [field])

    def test_missing_required_field_is_rejected(self) -> None:
        after = copy.deepcopy(self.before)
        del after["structured_body"]["input"]
        with self.assertRaisesRegex(ValueError, "required final-wire field missing"):
            compare_evidence(self.before, after, self.policy)

    def test_missing_provider_identity_is_rejected(self) -> None:
        after = copy.deepcopy(self.before)
        del after["provider_identity"]["wire_api"]
        with self.assertRaisesRegex(ValueError, "identity field missing"):
            compare_evidence(self.before, after, self.policy)

    def test_policy_cannot_silently_remove_required_fields(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["required_body_pointers"].remove("/tools")
        with self.assertRaisesRegex(ValueError, "changed without a schema version"):
            validate_policy(policy)

    def test_policy_cannot_silently_add_ignored_fields(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["ignored_body_pointers"] = ["/input/0/content"]
        with self.assertRaisesRegex(
            ValueError, "require a new reviewed policy version"
        ):
            validate_policy(policy)


class SnapshotChangeReportTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        self.snapshots = self.repo / "snapshots"
        self.candidates = self.repo / "candidates"
        self.snapshots.mkdir()
        self.candidates.mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_baseline(self, scenario_id: str, value: dict) -> None:
        (self.snapshots / f"all__suite__fixture__{scenario_id}.snap").write_text(
            "---\nsource: fixture.rs\n---\n"
            + json.dumps(value, ensure_ascii=False, indent=2)
            + "\n",
            encoding="utf-8",
        )

    def write_candidate(
        self, scenario_id: str, value: dict, *, indent: int = 2
    ) -> None:
        (self.candidates / f"{scenario_id}.json").write_text(
            json.dumps(value, ensure_ascii=False, indent=indent) + "\n",
            encoding="utf-8",
        )

    def report(self) -> dict:
        return compare_snapshot_set(
            self.repo,
            ["snapshots/all__suite__cache_*.snap", "snapshots/*.snap"],
            self.candidates,
        )

    def test_identical_semantics_ignore_json_formatting(self) -> None:
        value = {"request_1": {"input": ["stable"]}}
        self.write_baseline("stable", value)
        self.write_candidate("stable", value, indent=None)

        report = self.report()

        self.assertEqual(report["status"], "unchanged")
        self.assertEqual(report["scenario_count"], 1)
        scenario = report["scenarios"][0]
        self.assertEqual(scenario["status"], "unchanged")
        self.assertEqual(
            scenario["before_payload_sha256"], scenario["after_payload_sha256"]
        )

    def test_protected_value_change_reports_first_json_pointer(self) -> None:
        self.write_baseline("changed", {"request_1": {"tools": [{"name": "exec"}]}})
        self.write_candidate("changed", {"request_1": {"tools": [{"name": "read"}]}})

        report = self.report()

        self.assertEqual(report["status"], "changed")
        self.assertEqual(report["changed_scenario_count"], 1)
        self.assertEqual(
            report["scenarios"][0]["first_difference"],
            "/request_1/tools/0/name",
        )
        self.assertNotEqual(
            report["scenarios"][0]["before_payload_sha256"],
            report["scenarios"][0]["after_payload_sha256"],
        )
        self.assertEqual(
            report["scenarios"][0]["candidate_payload"],
            {"request_1": {"tools": [{"name": "read"}]}},
        )

    def test_unknown_field_is_a_semantic_change(self) -> None:
        self.write_baseline("unknown", {"request_1": {"model": "flash"}})
        self.write_candidate(
            "unknown", {"request_1": {"model": "flash", "new_field": True}}
        )

        report = self.report()

        self.assertEqual(report["status"], "changed")
        self.assertEqual(
            report["scenarios"][0]["first_difference"],
            "/request_1/new_field",
        )

    def test_missing_candidate_is_uncomparable(self) -> None:
        self.write_baseline("missing", {"request_1": {"input": []}})

        report = self.report()

        self.assertEqual(report["status"], "uncomparable")
        self.assertEqual(report["uncomparable_scenario_count"], 1)
        self.assertIn("produced no candidate", report["scenarios"][0]["error"])

    def test_unprotected_candidate_is_uncomparable(self) -> None:
        self.write_candidate("unexpected", {"request_1": {"input": []}})

        report = self.report()

        self.assertEqual(report["status"], "uncomparable")
        self.assertIn("no protected baseline", report["scenarios"][0]["error"])


if __name__ == "__main__":
    unittest.main()
