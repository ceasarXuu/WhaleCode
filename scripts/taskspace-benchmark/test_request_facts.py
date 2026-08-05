#!/usr/bin/env python3
"""Contract tests for canonical request fact normalization."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
from request_facts import build_request_facts  # noqa: E402


class RequestFactsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixtures = ROOT / "fixtures/i07"

    def test_rollout_snapshots_do_not_duplicate_usage(self) -> None:
        facts = build_request_facts(
            rollout_path=self.fixtures / "usage-double-count-rollout.jsonl"
        )
        self.assertEqual(facts["summary"]["completed_response_count"], 8)
        self.assertEqual(facts["summary"]["usage_record_count"], 8)
        self.assertEqual(facts["summary"]["state_snapshot_count"], 7)
        self.assertEqual(facts["summary"]["duplicate_event_count"], 0)
        self.assertEqual(
            facts["summary"]["usage"]["distribution"]["first_input_tokens"], 13147
        )
        self.assertEqual(facts["availability"]["usage"], "measured")

    def test_local_failed_attempt_is_not_a_boundary_mismatch(self) -> None:
        facts = build_request_facts(
            wire_path=self.fixtures / "attempt-boundary-wire.jsonl",
            boundary_path=self.fixtures / "attempt-boundary-events.jsonl",
            expected_model="deepseek-v4-flash",
        )
        self.assertEqual(facts["summary"]["local_attempt_count"], 11)
        self.assertEqual(facts["summary"]["boundary_request_count"], 10)
        self.assertEqual(facts["summary"]["completed_response_count"], 10)
        self.assertEqual(facts["summary"]["local_only_attempt_count"], 1)
        self.assertEqual(facts["findings"], [])

    def test_partial_identity_and_conflicting_duplicate_fail_closed(self) -> None:
        partial = self._write(
            "rollout.jsonl",
            [{"type": "event_msg", "payload": {"type": "token_count", "provider_request_id": "r1", "info": {"last_token_usage": self._usage(1)}}}],
        )
        facts = build_request_facts(rollout_path=partial)
        self.assertIn("identity_incomplete", self._codes(facts))
        self.assertEqual(facts["findings"][0]["line_number"], 1)
        duplicate = self._write(
            "duplicate.jsonl",
            [self._rollout("r1", "l1", 1, 1), self._rollout("r1", "l1", 1, 2)],
        )
        facts = build_request_facts(rollout_path=duplicate)
        self.assertIn("identity_conflict", self._codes(facts))
        self.assertEqual(facts["availability"]["usage"], "incomparable")

    def test_equal_duplicate_is_idempotent(self) -> None:
        event = self._rollout("r1", "l1", 1, 1)
        path = self._write("equal.jsonl", [event, event])
        facts = build_request_facts(rollout_path=path)
        self.assertEqual(facts["summary"]["usage_record_count"], 1)
        self.assertEqual(facts["summary"]["duplicate_event_count"], 1)

    def test_completed_without_boundary_and_unknown_boundary_fail_closed(self) -> None:
        wire = self._write(
            "wire.jsonl",
            [self._shape("r1", "l1", 1, 1, "a" * 64), self._terminal("r1", "l1", 1, "response_completed")],
        )
        boundary = self._write(
            "boundary.jsonl",
            [self._boundary_started(), self._claim(1, "b" * 64), self._boundary_stopped(1)],
        )
        facts = build_request_facts(wire_path=wire, boundary_path=boundary)
        self.assertIn("completed_without_boundary", self._codes(facts))
        self.assertIn("boundary_unattributed", self._codes(facts))
        self.assertEqual(facts["availability"]["boundary"], "measured")
        self.assertEqual(facts["availability"]["boundary_correlation"], "incomparable")

    def test_retry_preserves_logical_and_attempt_counts(self) -> None:
        wire = self._write(
            "retry.jsonl",
            [
                self._shape("r1", "logical", 1, 1, "a" * 64),
                self._terminal("r1", "logical", 1, "response_failed"),
                self._shape("r2", "logical", 2, 2, "b" * 64),
                self._terminal("r2", "logical", 2, "response_completed"),
            ],
        )
        facts = build_request_facts(wire_path=wire)
        self.assertEqual(facts["summary"]["logical_request_count"], 1)
        self.assertEqual(facts["summary"]["local_attempt_count"], 2)
        self.assertEqual(facts["summary"]["completed_response_count"], 1)

    def test_missing_terminal_is_partial_not_failed(self) -> None:
        wire = self._write(
            "missing-terminal.jsonl",
            [self._shape("r1", "l1", 1, 1, "a" * 64)],
        )
        facts = build_request_facts(wire_path=wire)
        self.assertIn("terminal_missing", self._codes(facts))
        self.assertEqual(
            next(item for item in facts["findings"] if item["code"] == "terminal_missing")["wire_line_number"],
            1,
        )
        self.assertEqual(facts["availability"]["completion"], "partial")
        self.assertEqual(facts["summary"]["failed_or_cancelled_attempt_count"], 0)

    def test_failed_terminal_rejects_rollout_usage(self) -> None:
        rollout = self._write("failed-rollout.jsonl", [self._rollout("r1", "l1", 1, 10)])
        wire = self._write(
            "failed-wire.jsonl",
            [
                self._shape("r1", "l1", 1, 1, "a" * 64),
                self._terminal("r1", "l1", 1, "response_failed"),
            ],
        )
        facts = build_request_facts(rollout_path=rollout, wire_path=wire)
        self.assertIn("usage_terminal_conflict", self._codes(facts))
        self.assertEqual(facts["summary"]["usage_record_count"], 0)
        self.assertEqual(facts["rows"][0]["usage"], None)
        self.assertEqual(facts["availability"]["usage"], "incomparable")

    def test_boundary_lifecycle_distinguishes_missing_from_healthy_zero(self) -> None:
        missing = self._write("missing-boundary.jsonl", [])
        facts = build_request_facts(boundary_path=missing)
        self.assertIn("boundary_lifecycle_missing", self._codes(facts))
        self.assertEqual(facts["availability"]["boundary"], "incomparable")
        self.assertEqual(facts["summary"]["boundary_request_count"], None)
        healthy = self._write(
            "healthy-zero-boundary.jsonl", [self._boundary_started(), self._boundary_stopped(0)]
        )
        facts = build_request_facts(boundary_path=healthy)
        self.assertEqual(facts["availability"]["boundary"], "measured")
        self.assertEqual(facts["summary"]["boundary_request_count"], 0)

    def test_identical_retry_payload_only_blocks_boundary_correlation(self) -> None:
        digest = "a" * 64
        wire = self._write(
            "same-retry-wire.jsonl",
            [
                self._shape("r1", "logical", 1, 1, digest),
                self._terminal("r1", "logical", 1, "response_failed"),
                self._shape("r2", "logical", 2, 2, digest),
                self._terminal("r2", "logical", 2, "response_completed"),
            ],
        )
        boundary = self._write(
            "same-retry-boundary.jsonl",
            [self._boundary_started(), self._claim(1, digest), self._claim(2, digest), self._boundary_stopped(2)],
        )
        facts = build_request_facts(wire_path=wire, boundary_path=boundary)
        self.assertEqual(facts["availability"]["attempt"], "measured")
        self.assertEqual(facts["availability"]["boundary"], "measured")
        self.assertEqual(facts["availability"]["boundary_correlation"], "incomparable")
        self.assertEqual(facts["availability"]["completion"], "measured")
        self.assertEqual(facts["availability"]["usage"], "measured")
        self.assertEqual(facts["summary"]["boundary_request_count"], 2)

    def test_diagnostics_are_recomputable_and_payload_free(self) -> None:
        facts = build_request_facts(
            rollout_path=self.fixtures / "usage-double-count-rollout.jsonl"
        )
        diagnostics = facts["diagnostics"]
        rows = facts["rows"]
        self.assertEqual(diagnostics["normalized_counts"]["row_count"], len(rows))
        self.assertEqual(
            diagnostics["normalized_counts"]["completed_count"],
            sum(row["terminal_status"] == "response_completed" for row in rows),
        )
        self.assertEqual(diagnostics["exclusions"]["state_snapshot_count"], 7)
        self.assertEqual(diagnostics["findings"]["total_count"], len(facts["findings"]))
        serialized = json.dumps(diagnostics, sort_keys=True).lower()
        for forbidden in ("prompt", "command", "arguments", "tool_output", "content"):
            self.assertNotIn(forbidden, serialized)

    def _write(self, name: str, events: list[dict]) -> Path:
        if not hasattr(self, "temporary"):
            self.temporary = tempfile.TemporaryDirectory()
            self.addCleanup(self.temporary.cleanup)
        path = Path(self.temporary.name) / name
        path.write_text("".join(json.dumps(event) + "\n" for event in events), encoding="utf-8")
        return path

    @staticmethod
    def _usage(seed: int) -> dict[str, int]:
        return {"input_tokens": seed, "cached_input_tokens": 0, "output_tokens": 1, "reasoning_output_tokens": 0, "total_tokens": seed + 1}

    def _rollout(self, request: str, logical: str, attempt: int, seed: int) -> dict:
        return {"type": "event_msg", "payload": {"type": "token_count", "provider_request_id": request, "provider_logical_request_id": logical, "provider_attempt_seq": attempt, "info": {"last_token_usage": self._usage(seed)}}}

    @staticmethod
    def _shape(request: str, logical: str, attempt: int, index: int, digest: str) -> dict:
        return {"schema_version": "provider-chat-wire-trace-v10", "request_id": request, "logical_request_id": logical, "attempt_seq": attempt, "request_index": index, "status": "payload_captured", "provider_payload_sha256": digest}

    def _terminal(self, request: str, logical: str, attempt: int, status: str) -> dict:
        event = {"schema_version": "provider-chat-wire-trace-v10", "request_id": request, "logical_request_id": logical, "attempt_seq": attempt, "status": status}
        if status == "response_completed":
            event.update(self._usage(10))
        return event

    @staticmethod
    def _claim(index: int, digest: str) -> dict:
        return {"event": "provider_request_claimed", "count": index, "method": "POST", "path": "/responses", "model": "deepseek-v4-flash", "body_sha256": digest}

    @staticmethod
    def _boundary_started() -> dict:
        return {"event": "provider_boundary_started", "limit": 10, "allowed_method": "POST", "allowed_path": "/responses", "allowed_model": "deepseek-v4-flash"}

    @staticmethod
    def _boundary_stopped(count: int) -> dict:
        return {"event": "provider_boundary_stopped", "request_count": count}

    @staticmethod
    def _codes(facts: dict) -> set[str]:
        return {finding["code"] for finding in facts["findings"]}


if __name__ == "__main__":
    unittest.main()
