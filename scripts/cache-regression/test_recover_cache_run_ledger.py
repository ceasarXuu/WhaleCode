#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_evidence import RESULT_SCHEMA_VERSION
from cache_surface import write_json
from recover_cache_run_ledger import mark_unsettled, recover


class RecoverCacheRunLedgerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        self.ledger = self.repo / "benchmarks/whale-agent-run-ledger.json"
        self.result = self.repo / "benchmarks/cache-regression/results/WAR-1.json"
        entry = {
            "record_id": "WAR-1",
            "status": "running",
            "started_at": "2026-08-01T00:00:00+00:00",
            "ended_at": None,
            "elapsed_calendar_seconds": None,
            "authorization": {
                "id": "CBA-FIXTURE-001",
                "reference": "approved fixture",
                "budget_summary": {
                    "provider_requests": 2,
                    "input_tokens": 100,
                    "output_tokens": 10,
                    "elapsed_seconds": 10,
                },
            },
            "execution": {
                "model": "deepseek-v4-flash",
                "sample_ids": ["simple"],
                "arm_ids": ["standard"],
                "repeats_per_arm_per_sample": 1,
                "planned_sample_runs": 1,
                "actual_sample_runs": 1,
                "api_requests": 0,
            },
            "tokens": {},
            "monetary_cost": {
                "pricing_snapshot": {
                    "currency": "USD",
                    "cached_input_per_million": 0.0028,
                    "uncached_input_per_million": 0.14,
                    "output_per_million": 0.28,
                }
            },
            "evidence": {
                "planned_run_root": "target/run",
                "subject_commit": "a" * 40,
                "surface_sha256": "b" * 64,
                "proposal_id": "CBP-FIXTURE",
                "proposal_contract_sha256": "c" * 64,
                "authorization_sha256": "d" * 64,
                "approved_selection": {
                    "model": "deepseek-v4-flash",
                    "samples": ["simple"],
                    "arms": ["standard"],
                    "repeat": 1,
                    "planned_sample_runs": 1,
                    "retry_sample_run_limit": 0,
                    "maximum_sample_runs": 1,
                    "stop_conditions": ["after_any_run_failure"],
                    "selection_reason": "fixture",
                },
                "evidence_boundary": "fixture only",
            },
        }
        write_json(self.ledger, {"updated_at": None, "entries": [entry]})
        result = {
            "schema_version": RESULT_SCHEMA_VERSION,
            "record_id": "WAR-1",
            "status": "completed",
            "started_at": "2026-08-01T00:00:00+00:00",
            "ended_at": "2026-08-01T00:00:02+00:00",
            "elapsed_seconds": 2.0,
            "result_path": self.result.relative_to(self.repo).as_posix(),
            "runner_exit_code": 0,
            "run_root": "target/run",
            "subject_commit": "a" * 40,
            "surface_sha256": "b" * 64,
            "proposal_id": "CBP-FIXTURE",
            "proposal_sha256": "c" * 64,
            "authorization_reference": "approved fixture",
            "authorization_sha256": "d" * 64,
            "observed_scope": entry["evidence"]["approved_selection"],
            "evidence_boundary": "fixture only",
            "actual_sample_runs": 1,
            "credential_source": "fixture",
            "observations": [
                {
                    "provider_requests": 2,
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "uncached_input_tokens": 20,
                    "output_tokens": 10,
                    "sample": "simple",
                    "arm": "standard",
                    "repeat": 1,
                    "run_id": "CACHE-001",
                }
            ],
            "attempts": [
                {
                    "sample": "simple",
                    "arm": "standard",
                    "repeat": 1,
                    "run_id": "CACHE-001",
                    "provider_boundary_request_count": 2,
                }
            ],
        }
        write_json(self.result, result)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_recovers_and_is_idempotent(self) -> None:
        self.assertEqual(recover(self.repo, self.ledger, self.result), "settled")
        self.assertEqual(
            recover(self.repo, self.ledger, self.result), "already_settled"
        )
        entry = json.loads(self.ledger.read_text(encoding="utf-8"))["entries"][0]
        self.assertEqual(entry["status"], "settled")
        self.assertEqual(entry["execution"]["api_requests"], 2)
        self.assertEqual(entry["evidence"]["usage_evidence_status"], "complete")

    def test_incomplete_run_can_be_explicitly_marked_unsettled(self) -> None:
        self.result.unlink()
        self.assertEqual(
            mark_unsettled(self.ledger, "WAR-1", "runner crashed before result"),
            "unsettled",
        )
        self.assertEqual(
            mark_unsettled(self.ledger, "WAR-1", "same audit"),
            "already_unsettled",
        )
        entry = json.loads(self.ledger.read_text(encoding="utf-8"))["entries"][0]
        self.assertEqual(entry["status"], "unsettled")
        self.assertEqual(entry["monetary_cost"]["status"], "unavailable")
        self.assertEqual(entry["evidence"]["outcome"], "unsettled")
        self.assertEqual(
            entry["evidence"]["recovery_reason"], "runner crashed before result"
        )

    def test_recovery_does_not_overwrite_concurrent_final_settlement(self) -> None:
        from cache_run_ledger import mutate_entry as locked_mutate

        def settle_before_recovery(path, record_id, update):
            ledger = json.loads(path.read_text(encoding="utf-8"))
            entry = ledger["entries"][0]
            entry["status"] = "settled"
            entry["evidence"]["result_path"] = self.result.relative_to(
                self.repo
            ).as_posix()
            write_json(path, ledger)
            return locked_mutate(path, record_id, update)

        with patch(
            "recover_cache_run_ledger.mutate_entry",
            side_effect=settle_before_recovery,
        ):
            self.assertEqual(
                recover(self.repo, self.ledger, self.result), "already_settled"
            )

    def test_recovery_rejects_nonstandard_json_numbers(self) -> None:
        content = self.result.read_text(encoding="utf-8").replace(
            '"elapsed_seconds": 2.0', '"elapsed_seconds": NaN'
        )
        self.result.write_text(content, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "JSON constant"):
            recover(self.repo, self.ledger, self.result)

    def test_recovery_rejects_status_exit_code_mismatch(self) -> None:
        value = json.loads(self.result.read_text(encoding="utf-8"))
        value["runner_exit_code"] = 3
        write_json(self.result, value)
        with self.assertRaisesRegex(ValueError, "envelope"):
            recover(self.repo, self.ledger, self.result)

    def test_recovery_rejects_result_outside_durable_claim(self) -> None:
        value = json.loads(self.result.read_text(encoding="utf-8"))
        value["proposal_id"] = "CBP-OTHER"
        write_json(self.result, value)
        with self.assertRaisesRegex(ValueError, "durable claim"):
            recover(self.repo, self.ledger, self.result)

        value["proposal_id"] = "CBP-FIXTURE"
        value["attempts"] *= 2
        value["actual_sample_runs"] = 2
        write_json(self.result, value)
        with self.assertRaisesRegex(ValueError, "approved matrix"):
            recover(self.repo, self.ledger, self.result)

    def test_recovery_rejects_boolean_as_integer_scope_evidence(self) -> None:
        value = json.loads(self.result.read_text(encoding="utf-8"))
        value["observed_scope"]["repeat"] = True
        value["attempts"][0]["repeat"] = True
        value["observations"][0]["repeat"] = True
        write_json(self.result, value)

        with self.assertRaisesRegex(ValueError, "durable claim"):
            recover(self.repo, self.ledger, self.result)

    def test_partial_result_recovers_with_truthful_request_minimum(self) -> None:
        value = json.loads(self.result.read_text(encoding="utf-8"))
        value["status"] = "partial"
        value["runner_exit_code"] = 3
        value["observations"] = []
        value["attempts"][0]["provider_boundary_request_count"] = None
        write_json(self.result, value)

        self.assertEqual(recover(self.repo, self.ledger, self.result), "settled")
        entry = json.loads(self.ledger.read_text(encoding="utf-8"))["entries"][0]
        self.assertEqual(entry["status"], "failed")
        self.assertIsNone(entry["execution"]["api_requests"])
        self.assertEqual(entry["execution"]["api_requests_minimum"], 0)
        self.assertEqual(
            entry["execution"]["api_requests_evidence_status"], "unavailable"
        )


if __name__ == "__main__":
    unittest.main()
