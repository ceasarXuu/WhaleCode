#!/usr/bin/env python3
"""Extract comparable metrics from one active-prefix app-server artifact set."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifacts", required=True)
    parser.add_argument("--arm", choices=("STD", "P1", "C1"), required=True)
    parser.add_argument("--output", default="")
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            records.append(json.loads(line))
    return records


def read_exit_code(path: Path) -> int | None:
    if not path.exists():
        return None
    return int(path.read_text(encoding="utf-8").strip())


def token_metrics(events: list[dict[str, Any]], turn_id: str) -> dict[str, Any]:
    usages = [
        event["params"]["tokenUsage"]["last"]
        for event in events
        if event.get("method") == "thread/tokenUsage/updated"
        and (event.get("params") or {}).get("turnId") == turn_id
    ]
    fields = (
        "inputTokens",
        "cachedInputTokens",
        "outputTokens",
        "reasoningOutputTokens",
    )
    totals = {field: sum(int(usage.get(field, 0)) for usage in usages) for field in fields}
    totals["uncachedInputTokens"] = totals["inputTokens"] - totals["cachedInputTokens"]
    totals["cacheHitPercent"] = (
        round(totals["cachedInputTokens"] * 100 / totals["inputTokens"], 2)
        if totals["inputTokens"]
        else None
    )
    totals["usageEventCount"] = len(usages)
    return totals


def provider_metrics(wire: list[dict[str, Any]]) -> dict[str, Any]:
    requests = [
        event
        for event in wire
        if event.get("event_name") != "provider.chat_wire_request_terminal"
    ]
    epochs: list[str] = []
    for event in requests:
        epoch_id = str(event.get("epoch_id", ""))
        if epoch_id not in epochs:
            epochs.append(epoch_id)

    def summarize(epoch_id: str) -> dict[str, Any]:
        records = [event for event in requests if str(event.get("epoch_id", "")) == epoch_id]
        return {
            "epochId": epoch_id,
            "requestCount": len(records),
            "payloadBytes": sum(int(event.get("provider_payload_bytes", 0)) for event in records),
            "strictPrefixPreservedCount": sum(event.get("prefix_preserved") is True for event in records),
            "sameEpochWarmRequestCount": max(0, len(records) - 1),
        }

    compact = summarize(epochs[0]) if epochs else summarize("")
    continuation = summarize(epochs[1]) if len(epochs) > 1 else summarize("")
    return {
        "requestCount": len(requests),
        "payloadBytes": sum(int(event.get("provider_payload_bytes", 0)) for event in requests),
        "compact": compact,
        "continuation": continuation,
    }


def parse_tags(tags: list[str]) -> dict[str, str]:
    parsed: dict[str, str] = {}
    for tag in tags:
        key, separator, value = tag.partition(":")
        if separator:
            parsed[key] = value
    return parsed


def projection_metrics(rollout: list[dict[str, Any]]) -> dict[str, Any]:
    budget_events: dict[str, dict[str, Any]] = {}
    for event in rollout:
        payload = event.get("payload") or {}
        if (
            event.get("type") == "event_msg"
            and payload.get("type") == "map_runtime"
            and payload.get("map_event_type") == "taskspace_trace_event_recorded"
            and payload.get("kind") == "projection_budget"
        ):
            budget_events[str(payload.get("traceEventId", len(budget_events)))] = payload
        if payload.get("map_event_type") != "snapshot_delta":
            continue
        for operation in payload.get("patch") or []:
            value = operation.get("value") or {}
            if isinstance(value, dict) and value.get("kind") == "projection_budget":
                budget_events[str(value.get("id", len(budget_events)))] = value
    budgets = [parse_tags(event.get("tags") or []) for event in budget_events.values()]
    latest = budgets[-1] if budgets else {}
    activation = int(latest.get("strategy_activation_count", "0"))
    before = int(latest["projection_bytes_before_strategy"]) if "projection_bytes_before_strategy" in latest else None
    after = int(latest["projection_bytes_after_strategy"]) if "projection_bytes_after_strategy" in latest else None
    return {
        "available": bool(budgets),
        "eventCount": len(budgets),
        "activationCount": activation,
        "projectionBytes": int(latest["projection_bytes"]) if "projection_bytes" in latest else None,
        "bytesBeforeStrategy": before,
        "bytesAfterStrategy": after,
        "afterBeforeRatio": round(after / before, 4) if before and after is not None else None,
        "coveredNodeCount": int(latest["covered_node_count"]) if "covered_node_count" in latest else 0,
        "archiveRef": latest.get("archive_ref"),
    }


def map_metrics(rollout: list[dict[str, Any]]) -> dict[str, Any]:
    statuses: dict[str, str] = {}
    edges: set[tuple[str, str]] = set()
    runtime_mode = "standard"
    ownership_active = False
    for event in rollout:
        payload = event.get("payload") or {}
        if event.get("type") != "event_msg" or payload.get("type") != "map_runtime":
            continue
        kind = payload.get("map_event_type")
        if kind == "node_status_changed":
            statuses[str(payload["nodeId"])] = str(payload["currentStatus"])
        elif kind == "edge_recorded":
            edges.add((str(payload.get("fromNodeId")), str(payload.get("toNodeId"))))
        elif kind == "mode_changed":
            runtime_mode = str(payload.get("currentMode", runtime_mode))
        elif kind == "task_context_ownership_changed":
            ownership_active = bool(payload.get("active", ownership_active))
    open_statuses = {"pending", "ready", "running", "blocked"}
    recorded_open_nodes = sorted(
        node for node, status in statuses.items() if status in open_statuses
    )
    return {
        "runtimeMode": runtime_mode,
        "taskContextOwnershipActive": ownership_active,
        "nodeCount": len(statuses),
        "edgeCount": len(edges),
        "nodeStatuses": dict(sorted(statuses.items())),
        "recordedOpenNodeIds": recorded_open_nodes,
        "activeOpenNodeIds": recorded_open_nodes if ownership_active else [],
    }


def action_metrics(events: list[dict[str, Any]]) -> dict[str, Any]:
    completed_items = [
        (event.get("params") or {}).get("item") or {}
        for event in events
        if event.get("method") == "item/completed"
    ]
    counts = Counter(str(item.get("type", "unknown")) for item in completed_items)
    commands = [item for item in completed_items if item.get("type") == "commandExecution"]
    return {
        "completedItemCounts": dict(sorted(counts.items())),
        "commandCount": len(commands),
        "failedCommandCount": sum(item.get("status") == "failed" for item in commands),
        "fileChangeCount": counts.get("fileChange", 0),
        "rpcErrorResponseCount": sum(event.get("error") is not None for event in events),
        "terminalProviderErrorCount": sum(
            event.get("method") == "error"
            and not ((event.get("params") or {}).get("willRetry", False))
            for event in events
        ),
    }


def main() -> int:
    args = parse_args()
    artifacts = Path(args.artifacts)
    summary = read_json(artifacts / "client-summary.json")
    events = read_jsonl(artifacts / "app-server-events.jsonl")
    wire = read_jsonl(artifacts / "provider-wire-trace.jsonl")
    rollout = read_jsonl(artifacts / "final-rollout.jsonl")
    compact_turn_id = str((summary.get("compact_turn") or {}).get("id", ""))
    continuation_turn_id = str((summary.get("continuation_turn") or {}).get("id", ""))
    metrics = {
        "schemaVersion": "taskspace-active-prefix-metrics-v1",
        "arm": args.arm,
        "mode": summary.get("mode"),
        "status": summary.get("status"),
        "validation": {
            "initialExitCode": read_exit_code(artifacts / "initial-validation.exit-code.txt"),
            "finalExitCode": read_exit_code(artifacts / "final-validation.exit-code.txt"),
        },
        "time": summary.get("phase_times"),
        "provider": provider_metrics(wire),
        "tokens": {
            "compact": token_metrics(events, compact_turn_id),
            "continuation": token_metrics(events, continuation_turn_id),
        },
        "actions": action_metrics(events),
        "projection": projection_metrics(rollout),
        "map": map_metrics(rollout),
    }
    output = Path(args.output) if args.output else artifacts / "active-prefix-metrics.json"
    output.write_text(json.dumps(metrics, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
