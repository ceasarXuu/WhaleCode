#!/usr/bin/env python3
"""Validate the benchmark launch identity behind a cache arm."""

from __future__ import annotations

from typing import Any

from cache_provider_route import EXPECTED_ROUTE, validate_route_identity


TASKSPACE_ARMS = ("map-always", "map-append", "map-request")


def provider_route_overrides() -> list[str]:
    provider_id = EXPECTED_ROUTE["transport_provider_id"]
    return [
        f'model_provider="{provider_id}"',
        f'model_providers.{provider_id}.name="{EXPECTED_ROUTE["provider_name"]}"',
        f'model_providers.{provider_id}.base_url="{EXPECTED_ROUTE["base_url"]}"',
        f'model_providers.{provider_id}.env_key="{EXPECTED_ROUTE["env_key"]}"',
        f'model_providers.{provider_id}.env_key_instructions="Set DEEPSEEK_API_KEY to a DeepSeek API key before starting Whale."',
        f'model_providers.{provider_id}.wire_api="{EXPECTED_ROUTE["wire_api"]}"',
    ]


def validate_route_argv(argv: list[str]) -> None:
    for override in provider_route_overrides():
        matches = [
            index
            for index, value in enumerate(argv)
            if value == override and index > 0 and argv[index - 1] == "-c"
        ]
        if len(matches) != 1:
            raise ValueError("cache execution provider route arguments are incomplete")
    if any(value.startswith("model_providers.deepseek.") for value in argv):
        raise ValueError("cache execution overrides the reserved DeepSeek provider")


def policy_argument(arm: str) -> str:
    if arm not in TASKSPACE_ARMS:
        raise ValueError(f"unsupported taskspace cache arm: {arm}")
    return f'taskspace_projection_policy="{arm}"'


def validate_arm_identity(
    argv_evidence: dict[str, Any], mode_map: dict[str, Any], arm: str
) -> None:
    if arm != "standard" and arm not in TASKSPACE_ARMS:
        raise ValueError(f"unsupported cache arm: {arm}")
    expected_mode = "standard" if arm == "standard" else "taskspace"
    expected_side = "left" if arm == "standard" else "right"
    argv = argv_evidence.get("argv")
    common = argv_evidence.get("common_argv_without_treatment")
    if (
        argv_evidence.get("logical_mode") != expected_mode
        or argv_evidence.get("execution_substrate") != "docker"
        or not isinstance(argv, list)
        or not all(isinstance(item, str) for item in argv)
        or not isinstance(common, list)
        or not all(isinstance(item, str) for item in common)
        or mode_map.get(expected_side) != expected_mode
        or type(mode_map.get("repeat")) is not int
        or mode_map["repeat"] < 1
    ):
        raise ValueError("cache execution identity does not match its arm")
    model_indexes = [index for index, value in enumerate(common) if value == "-m"]
    if (
        len(model_indexes) != 1
        or model_indexes[0] + 1 >= len(common)
        or not common[model_indexes[0] + 1].startswith("deepseek-")
    ):
        raise ValueError("cache execution identity does not use the approved DeepSeek model")
    validate_route_identity(argv_evidence.get("provider_routing"))
    validate_route_argv(common)
    if arm == "standard":
        if (
            argv != common
            or argv_evidence.get("treatment_delta") is not None
            or "--taskspace" in argv
            or any(policy_argument(item) in argv for item in TASKSPACE_ARMS)
        ):
            raise ValueError("standard cache execution identity is invalid")
        return
    expected_policy = policy_argument(arm)
    expected_delta = ["--taskspace", f"-c {expected_policy}"]
    if argv_evidence.get("treatment_delta") != expected_delta:
        raise ValueError("taskspace cache treatment delta is invalid")
    normalized = list(argv)
    try:
        normalized.remove("--taskspace")
        policy_index = normalized.index(expected_policy)
    except ValueError as error:
        raise ValueError("taskspace cache launch arguments are incomplete") from error
    if policy_index == 0 or normalized[policy_index - 1] != "-c":
        raise ValueError("taskspace projection policy is not a config argument")
    del normalized[policy_index - 1 : policy_index + 1]
    if normalized != common:
        raise ValueError("taskspace cache launch differs beyond its treatment")


def fixture_arm_identity(arm: str) -> tuple[dict[str, Any], dict[str, Any]]:
    common = ["exec", "--json"]
    for override in provider_route_overrides():
        common.extend(["-c", override])
    common.extend(["-m", "deepseek-v4-flash", "-"])
    if arm == "standard":
        argv = list(common)
        delta = None
    else:
        expected_policy = policy_argument(arm)
        argv = list(common)
        argv.insert(2, "--taskspace")
        argv[3:3] = ["-c", expected_policy]
        delta = ["--taskspace", f"-c {expected_policy}"]
    return (
        {
            "logical_mode": "standard" if arm == "standard" else "taskspace",
            "argv": argv,
            "common_argv_without_treatment": common,
            "treatment_delta": delta,
            "execution_substrate": "docker",
            "provider_routing": EXPECTED_ROUTE,
        },
        {"repeat": 1, "left": "standard", "right": "taskspace"},
    )
