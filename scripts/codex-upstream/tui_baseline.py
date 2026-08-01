#!/usr/bin/env python3
"""Normalize cargo-nextest JUnit into a deterministic TUI baseline."""

from __future__ import annotations

from collections import Counter
from html.parser import HTMLParser

CLASSIFICATIONS = (
    "environment",
    "flaky_candidate",
    "functional_assertion",
    "snapshot_review",
    "unknown",
)


def classify_failure(name: str, output: str) -> str:
    lowered = f"{name}\n{output}".lower()
    if any(
        marker in lowered
        for marker in ("snapshot assertion", ".snap.new", "insta", "snapshot mismatch")
    ):
        return "snapshot_review"
    if any(
        marker in lowered
        for marker in (
            "stack overflow",
            "overflowed its stack",
            "could not spawn",
            "no space left on device",
            "out of memory",
        )
    ):
        return "environment"
    if any(marker in lowered for marker in ("assertion failed", "panicked at")):
        return "functional_assertion"
    return "unknown"


def _case_name(case: dict) -> str:
    classname = case.get("classname", "").strip()
    name = case.get("name", "").strip()
    if not classname:
        return name
    if name.startswith(f"{classname}::"):
        return name
    return f"{classname}::{name}"


class _JUnitParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.cases: list[dict] = []
        self._active: dict | None = None
        self._output_tag: str | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == "testcase":
            if self._active is not None:
                raise ValueError("JUnit contains nested testcase elements")
            self._active = {
                **{key: value or "" for key, value in attrs},
                "result": "passed",
                "output": [],
            }
        elif self._active is not None and tag in {"failure", "error"}:
            self._active["result"] = "failed"
            self._output_tag = tag
        elif self._active is not None and tag == "skipped":
            self._active["result"] = "ignored"

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)
        self.handle_endtag(tag)

    def handle_endtag(self, tag: str) -> None:
        if tag == "testcase":
            if self._active is None:
                raise ValueError("JUnit closes a testcase that was not opened")
            self.cases.append(self._active)
            self._active = None
            self._output_tag = None
        elif tag == self._output_tag:
            self._output_tag = None

    def handle_data(self, data: str) -> None:
        if self._active is not None and self._output_tag is not None:
            self._active["output"].append(data)

    def finish(self) -> list[dict]:
        if self._active is not None:
            raise ValueError("JUnit ended with an open testcase")
        if not self.cases:
            raise ValueError("JUnit contains no testcase elements")
        return self.cases


def parse_junit(xml_bytes: bytes) -> dict:
    parser = _JUnitParser()
    parser.feed(xml_bytes.decode("utf-8"))
    cases = parser.finish()
    entries: list[dict] = []
    for case in cases:
        result = case["result"]
        output = "".join(case["output"])
        classification = (
            classify_failure(_case_name(case), output) if result == "failed" else None
        )
        entries.append(
            {
                "name": _case_name(case),
                "result": result,
                "classification": classification,
            }
        )
    entries.sort(key=lambda entry: entry["name"])
    if len(entries) != len({entry["name"] for entry in entries}):
        raise ValueError("JUnit contains duplicate normalized test names")
    counts = Counter(entry["result"] for entry in entries)
    classifications = Counter(
        entry["classification"]
        for entry in entries
        if entry["classification"] is not None
    )
    return {
        "schema_version": 1,
        "runner": "cargo-nextest",
        "package": "codex-tui",
        "profile": "whale-baseline",
        "environment": {
            "INSTA_UPDATE": "no",
            "RUST_MIN_STACK": "8388608",
        },
        "entries": entries,
        "summary": {
            "test_count": len(entries),
            "by_result": dict(sorted(counts.items())),
            "by_classification": dict(sorted(classifications.items())),
        },
    }
