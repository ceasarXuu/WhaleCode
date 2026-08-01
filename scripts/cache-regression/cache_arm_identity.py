#!/usr/bin/env python3
"""Validate the benchmark launch identity behind a cache arm."""

from __future__ import annotations

from typing import Any


TASKSPACE_ARMS = ("map-always", "map-append", "map-request")


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
    common = ["exec", "--json", "-m", "deepseek-v4-flash", "-"]
    if arm == "standard":
        argv = list(common)
        delta = None
    else:
        expected_policy = policy_argument(arm)
        argv = [
            "exec",
            "--json",
            "--taskspace",
            "-c",
            expected_policy,
            "-m",
            "deepseek-v4-flash",
            "-",
        ]
        delta = ["--taskspace", f"-c {expected_policy}"]
    return (
        {
            "logical_mode": "standard" if arm == "standard" else "taskspace",
            "argv": argv,
            "common_argv_without_treatment": common,
            "treatment_delta": delta,
            "execution_substrate": "docker",
        },
        {"repeat": 1, "left": "standard", "right": "taskspace"},
    )
