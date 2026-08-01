from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RESULT = ROOT / "benchmarks/taskspace/r7/a2-b0-provider-wire-result.json"


def load_result() -> dict:
    return json.loads(RESULT.read_text(encoding="utf-8"))


def test_b0_is_completed_without_production_activation() -> None:
    result = load_result()
    assert result["phase"] == "R7.1-A2-B0"
    assert result["status"] == "completed"
    assert result["decision"] == "proceed_to_a2_b1"
    assert result["production_behavior_changed"] is False


def test_all_positive_and_negative_scenarios_passed() -> None:
    result = load_result()
    assert result["execution"]["repeat_per_scenario"] == 3
    assert len(result["scenario_results"]) == 6
    assert result["aggregate"]["all_scenarios_passed"] == 18
    assert result["aggregate"]["all_scenarios_requests"] == 18
    assert result["aggregate"]["positive_passed"] == 12
    assert result["aggregate"]["positive_requests"] == 12
    assert all(row["passed"] == row["requests"] for row in result["scenario_results"])


def test_b0_capability_boundary_is_explicit() -> None:
    capability = load_result()["capabilities"]
    assert capability["ordered_multi_function_calls"] is True
    assert capability["agent_manifest_sibling_pairing"] is True
    assert capability["agent_declared_node_id_per_action"] is True
    assert capability["apply_patch_text_exact"] is True
    assert capability["provider_native_non_function_tool"] == (
        "unsupported_by_deepseek_chat"
    )
    assert capability["positive_thinking_tool_choice"] == "auto"
    assert capability["required_with_thinking"] == "unsupported_by_deepseek_chat"


def test_b0_evidence_contains_no_secret_or_raw_payload() -> None:
    result = load_result()
    assert result["privacy"]["api_key_recorded"] is False
    assert result["privacy"]["authorization_header_recorded"] is False
    assert result["privacy"]["tracked_artifact_contains_raw_provider_payload"] is False
