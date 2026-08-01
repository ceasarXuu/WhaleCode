#!/usr/bin/env python3
"""Semantic change evidence shared by cache promotion and its tests."""

from __future__ import annotations

from typing import Any


def changed_scenarios(report: dict[str, Any]) -> list[dict[str, Any]]:
    scenarios = [
        item
        for command in report["free_validation"]["commands"]
        if command.get("change_report")
        for item in command["change_report"]["scenarios"]
        if item["status"] == "changed"
    ]
    scenarios.sort(key=lambda item: item["scenario_id"])
    ids = [item["scenario_id"] for item in scenarios]
    if len(ids) != len(set(ids)):
        raise ValueError("changed scenarios are invalid")
    if report.get("discovery_state") != "revalidation_requested" and not scenarios:
        raise ValueError("changed scenarios are invalid")
    return scenarios
