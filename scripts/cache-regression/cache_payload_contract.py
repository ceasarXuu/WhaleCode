#!/usr/bin/env python3
"""Compare captured final-wire evidence without rewriting provider semantics."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "whalecode-final-wire-comparison-v2"
CHANGE_REPORT_SCHEMA_VERSION = "whalecode-final-wire-change-report-v1"
REQUIRED_BODY_POINTERS = (
    "/instructions",
    "/input",
    "/tools",
    "/tool_choice",
    "/model",
)
PROTECTED_PROVIDER_IDENTITY_FIELDS = (
    "provider_id",
    "wire_api",
    "endpoint_path",
)


def load_policy(path: Path) -> dict[str, Any]:
    policy = json.loads(path.read_text(encoding="utf-8"))
    validate_policy(policy)
    return policy


def validate_policy(policy: dict[str, Any]) -> None:
    if policy.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("unsupported final-wire comparison policy")
    if policy.get("body_comparison") != "exact_json_value":
        raise ValueError("final-wire body comparison must remain exact")
    if policy.get("unknown_body_field_policy") != "protected":
        raise ValueError("unknown final-wire fields must remain protected")
    if policy.get("array_order_policy") != "protected":
        raise ValueError("final-wire array order must remain protected")
    if policy.get("string_content_policy") != "protected":
        raise ValueError("final-wire string content must remain protected")
    if tuple(policy.get("required_body_pointers", ())) != REQUIRED_BODY_POINTERS:
        raise ValueError(
            "final-wire required body fields changed without a schema version"
        )
    if (
        tuple(policy.get("protected_provider_identity_fields", ()))
        != PROTECTED_PROVIDER_IDENTITY_FIELDS
    ):
        raise ValueError("provider identity fields changed without a schema version")
    if policy.get("ignored_body_pointers"):
        raise ValueError(
            "ignored final-wire fields require a new reviewed policy version"
        )
    if policy.get("raw_body_sha_policy") != "integrity_evidence":
        raise ValueError("raw body SHA must remain integrity evidence")


def _json_pointer_part(value: str) -> str:
    return value.replace("~", "~0").replace("/", "~1")


def _first_difference(before: Any, after: Any, path: str = "") -> str | None:
    if type(before) is not type(after):
        return path or "/"
    if isinstance(before, dict):
        before_keys = set(before)
        after_keys = set(after)
        if before_keys != after_keys:
            changed_key = sorted(before_keys ^ after_keys)[0]
            return f"{path}/{_json_pointer_part(changed_key)}"
        for key in sorted(before_keys):
            difference = _first_difference(
                before[key], after[key], f"{path}/{_json_pointer_part(key)}"
            )
            if difference is not None:
                return difference
        return None
    if isinstance(before, list):
        if len(before) != len(after):
            return f"{path}/length"
        for index, (before_item, after_item) in enumerate(zip(before, after)):
            difference = _first_difference(before_item, after_item, f"{path}/{index}")
            if difference is not None:
                return difference
        return None
    return None if before == after else (path or "/")


def _canonical_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _load_insta_json_snapshot(path: Path) -> Any:
    content = path.read_text(encoding="utf-8")
    separator = "\n---\n"
    if not content.startswith("---\n") or separator not in content:
        raise ValueError("invalid insta snapshot envelope")
    return json.loads(content.split(separator, 1)[1])


def _load_candidate_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _scenario_id(path: Path) -> str:
    return path.name.removesuffix(".snap").rsplit("__", 1)[-1]


def compare_snapshot_set(
    repo: Path, baseline_globs: list[str], candidate_dir: Path
) -> dict[str, Any]:
    baseline_paths = sorted(
        {path for pattern in baseline_globs for path in repo.glob(pattern)}
    )
    candidate_paths = sorted(candidate_dir.glob("*.json"))
    baselines: dict[str, Path] = {}
    errors: list[str] = []
    for path in baseline_paths:
        scenario_id = _scenario_id(path)
        if scenario_id in baselines:
            errors.append(f"duplicate baseline scenario: {scenario_id}")
        baselines[scenario_id] = path
    candidates = {path.stem: path for path in candidate_paths}
    if len(candidates) != len(candidate_paths):
        errors.append("duplicate candidate scenario")

    scenarios = []
    for scenario_id in sorted(set(baselines).union(candidates)):
        baseline_path = baselines.get(scenario_id)
        candidate_path = candidates.get(scenario_id)
        scenario = {
            "scenario_id": scenario_id,
            "comparison_object": "normalized_final_wire_snapshot",
            "baseline_path": (
                baseline_path.relative_to(repo).as_posix() if baseline_path else None
            ),
            "status": "uncomparable",
            "first_difference": None,
            "before_payload_sha256": None,
            "after_payload_sha256": None,
            "candidate_payload": None,
        }
        if baseline_path is None:
            scenario["error"] = "candidate has no protected baseline"
        elif candidate_path is None:
            scenario["error"] = "protected baseline produced no candidate"
        else:
            try:
                before = _load_insta_json_snapshot(baseline_path)
                after = _load_candidate_json(candidate_path)
                if not isinstance(before, dict) or not isinstance(after, dict):
                    raise ValueError("final-wire snapshot root must be an object")
                difference = _first_difference(before, after)
                scenario.update(
                    {
                        "status": "changed" if difference is not None else "unchanged",
                        "first_difference": difference,
                        "before_payload_sha256": _canonical_sha256(before),
                        "after_payload_sha256": _canonical_sha256(after),
                        "candidate_payload": after if difference is not None else None,
                    }
                )
            except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
                scenario["error"] = f"{type(error).__name__}: {error}"
        scenarios.append(scenario)

    statuses = {scenario["status"] for scenario in scenarios}
    if errors or not scenarios or "uncomparable" in statuses:
        status = "uncomparable"
    elif "changed" in statuses:
        status = "changed"
    else:
        status = "unchanged"
    return {
        "schema_version": CHANGE_REPORT_SCHEMA_VERSION,
        "status": status,
        "comparison_policy": "exact_normalized_json_value",
        "scenario_count": len(scenarios),
        "changed_scenario_count": sum(
            scenario["status"] == "changed" for scenario in scenarios
        ),
        "uncomparable_scenario_count": sum(
            scenario["status"] == "uncomparable" for scenario in scenarios
        ),
        "errors": errors,
        "scenarios": scenarios,
    }


def _resolve_pointer(value: Any, pointer: str) -> Any:
    current = value
    for encoded_part in pointer.removeprefix("/").split("/"):
        part = encoded_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            raise ValueError(f"required final-wire field missing: {pointer}")
    return current


def compare_evidence(
    before: dict[str, Any], after: dict[str, Any], policy: dict[str, Any]
) -> dict[str, Any]:
    before_body = before["structured_body"]
    after_body = after["structured_body"]
    for pointer in policy["required_body_pointers"]:
        _resolve_pointer(before_body, pointer)
        _resolve_pointer(after_body, pointer)

    before_identity = before.get("provider_identity")
    after_identity = after.get("provider_identity")
    if not isinstance(before_identity, dict) or not isinstance(after_identity, dict):
        raise ValueError("final-wire provider identity must be present")

    identity_differences = []
    for field in policy["protected_provider_identity_fields"]:
        if field not in before_identity or field not in after_identity:
            raise ValueError(f"required provider identity field missing: {field}")
        before_value = before_identity[field]
        after_value = after_identity[field]
        if before_value != after_value:
            identity_differences.append(field)

    first_body_difference = _first_difference(before_body, after_body)
    raw_body_changed = before["raw_body_sha256"] != after["raw_body_sha256"]
    return {
        "cache_relevant_changed": bool(
            first_body_difference is not None or identity_differences
        ),
        "first_body_difference": first_body_difference,
        "provider_identity_differences": identity_differences,
        "raw_body_changed": raw_body_changed,
        "raw_only_change": bool(
            raw_body_changed
            and first_body_difference is None
            and not identity_differences
        ),
    }
