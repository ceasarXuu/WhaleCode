#!/usr/bin/env python3
"""Pure monetary-cost settlement shared by runner and promotion validation."""

from __future__ import annotations

from typing import Any


COST_FORMULA = (
    "cached_input/1e6*cached_rate + uncached_input/1e6*miss_rate "
    "+ output/1e6*output_rate"
)


def settled_monetary_cost(
    tokens: dict[str, int],
    pricing: dict[str, Any],
    *,
    evidence_status: str,
) -> dict[str, Any]:
    has_usage = evidence_status in {"complete", "partial"}
    components = {
        "cached_input": tokens["cached_input"]
        / 1_000_000
        * pricing["cached_input_per_million"],
        "uncached_input": tokens["uncached_input"]
        / 1_000_000
        * pricing["uncached_input_per_million"],
        "output": tokens["output"] / 1_000_000 * pricing["output_per_million"],
    }
    return {
        "status": (
            "estimated"
            if evidence_status == "complete"
            else "estimated_partial"
            if evidence_status == "partial"
            else "unavailable"
        ),
        "currency": pricing["currency"],
        "amount": round(sum(components.values()), 10) if has_usage else None,
        "actual_billed_amount": None,
        "components": components if has_usage else None,
        "pricing_snapshot": pricing,
        "formula": COST_FORMULA if has_usage else None,
        "note": (
            "按完整 provider token 遥测和冻结价格估算。"
            if evidence_status == "complete"
            else "按已取得的部分 provider token 遥测估算；金额是已知最低值。"
            if evidence_status == "partial"
            else "无完整 token 证据。"
        ),
    }


def complete_cost_from_counts(
    input_tokens: int,
    cached_input_tokens: int,
    output_tokens: int,
    pricing: dict[str, Any],
) -> dict[str, Any]:
    return settled_monetary_cost(
        {
            "input": input_tokens,
            "cached_input": cached_input_tokens,
            "uncached_input": input_tokens - cached_input_tokens,
            "output": output_tokens,
        },
        pricing,
        evidence_status="complete",
    )
