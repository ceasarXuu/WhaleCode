"""Pure summaries derived from normalized request fact rows."""

from __future__ import annotations

from typing import Any


def _percentile(values: list[int], percentile: int) -> int | None:
    if not values:
        return None
    ordered = sorted(values)
    rank = (percentile * len(ordered) + 99) // 100
    return ordered[max(0, min(len(ordered) - 1, rank - 1))]


def usage_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    usage = [row["usage"] for row in rows if row["usage"] is not None]
    inputs = [value["input_tokens"] for value in usage]
    cached = [value["cached_input_tokens"] for value in usage]
    outputs = [value["output_tokens"] for value in usage]
    return {
        "input_tokens": sum(inputs),
        "cached_input_tokens": sum(cached),
        "uncached_input_tokens": sum(inputs) - sum(cached),
        "output_tokens": sum(outputs),
        "reasoning_output_tokens": sum(value["reasoning_output_tokens"] for value in usage),
        "total_tokens": sum(value["total_tokens"] for value in usage),
        "distribution": {
            "first_input_tokens": inputs[0] if inputs else None,
            "last_input_tokens": inputs[-1] if inputs else None,
            "max_input_tokens": max(inputs) if inputs else None,
            "p95_input_tokens": _percentile(inputs, 95),
            "first_output_tokens": outputs[0] if outputs else None,
            "last_output_tokens": outputs[-1] if outputs else None,
            "max_output_tokens": max(outputs) if outputs else None,
            "p95_output_tokens": _percentile(outputs, 95),
            "max_cached_input_tokens": max(cached) if cached else None,
            "p95_cached_input_tokens": _percentile(cached, 95),
        },
    }
