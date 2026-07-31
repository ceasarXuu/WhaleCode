#!/usr/bin/env python3
"""Compare captured final-wire evidence without rewriting provider semantics."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "whalecode-final-wire-comparison-v2"
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
        raise ValueError("final-wire required body fields changed without a schema version")
    if (
        tuple(policy.get("protected_provider_identity_fields", ()))
        != PROTECTED_PROVIDER_IDENTITY_FIELDS
    ):
        raise ValueError("provider identity fields changed without a schema version")
    if policy.get("ignored_body_pointers"):
        raise ValueError("ignored final-wire fields require a new reviewed policy version")
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
            difference = _first_difference(
                before_item, after_item, f"{path}/{index}"
            )
            if difference is not None:
                return difference
        return None
    return None if before == after else (path or "/")


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
