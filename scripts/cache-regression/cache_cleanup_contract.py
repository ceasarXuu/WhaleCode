#!/usr/bin/env python3
"""Pure cleanup-proof contract shared by execution and evidence consumers."""

from __future__ import annotations

from typing import Any


CLEANUP_SUCCESS_STATUSES = frozenset({"verified_absent", "removed_verified"})
CLEANUP_STABLE_EMPTY_POLLS = 3


def cleanup_verified(result: dict[str, Any]) -> bool:
    dimensions = (
        ("status", "container_ids"),
        ("network_cleanup_status", "network_ids"),
        ("secret_cleanup_status", "secret_paths"),
    )
    if (
        not isinstance(result, dict)
        or type(result.get("stable_empty_polls")) is not int
        or result["stable_empty_polls"] < CLEANUP_STABLE_EMPTY_POLLS
        or result.get("error") != ""
    ):
        return False
    for status_key, residue_key in dimensions:
        status = result.get(status_key)
        residue = result.get(residue_key)
        if status not in CLEANUP_SUCCESS_STATUSES or not isinstance(residue, list):
            return False
        if (status == "verified_absent" and residue) or (
            status == "removed_verified" and not residue
        ):
            return False
    return True
