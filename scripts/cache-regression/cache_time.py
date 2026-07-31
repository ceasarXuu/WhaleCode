#!/usr/bin/env python3
"""Shared timestamp validation for cache authorization evidence."""

from __future__ import annotations

from datetime import datetime, timedelta, timezone


MAX_CLOCK_SKEW = timedelta(minutes=5)


def now_iso() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")


def parse_timestamp(value: object, label: str) -> datetime:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} is missing")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise ValueError(f"{label} is not ISO-8601") from error
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise ValueError(f"{label} must include a timezone")
    return parsed


def require_not_future(value: datetime, label: str) -> None:
    if value > datetime.now(timezone.utc).astimezone() + MAX_CLOCK_SKEW:
        raise ValueError(f"{label} is in the future")


def require_ordered(
    earlier: datetime, later: datetime, earlier_label: str, later_label: str
) -> None:
    if later < earlier:
        raise ValueError(f"{later_label} precedes {earlier_label}")
