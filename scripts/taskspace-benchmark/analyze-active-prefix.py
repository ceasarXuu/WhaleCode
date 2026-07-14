#!/usr/bin/env python3
"""Extract comparable metrics from one active-prefix app-server artifact set."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import Counter
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifacts", default="")
    parser.add_argument("--records", default="")
    parser.add_argument("--arm", choices=("STD", "P1", "C1"))
    parser.add_argument("--output", default="")
    args = parser.parse_args()
    if bool(args.artifacts) == bool(args.records):
        parser.error("provide exactly one of --artifacts or --records")
    if args.artifacts and not args.arm:
        parser.error("--arm is required with --artifacts")
    if args.records and args.arm:
        parser.error("--arm is not valid with --records")
    return args


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
    budget_events: dict[str, tuple[dict[str, Any], str | None]] = {}
    for event in rollout:
        payload = event.get("payload") or {}
        if (
            event.get("type") == "event_msg"
            and payload.get("type") == "map_runtime"
            and payload.get("map_event_type") == "taskspace_trace_event_recorded"
            and payload.get("kind") == "projection_budget"
        ):
            event_id = str(payload.get("traceEventId", len(budget_events)))
            budget_events.setdefault(event_id, (payload, None))
        if payload.get("map_event_type") != "snapshot_delta":
            continue
        for operation in payload.get("patch") or []:
            value = operation.get("value") or {}
            if isinstance(value, dict) and value.get("kind") == "projection_budget":
                budget_events[str(value.get("id", len(budget_events)))] = (
                    value,
                    payload.get("previousSnapshotSha256") or payload.get("baseSnapshotSha256"),
                )
    budgets = [
        (parse_tags(event.get("tags") or []), snapshot_hash)
        for event, snapshot_hash in budget_events.values()
    ]
    latest, input_snapshot_hash = budgets[-1] if budgets else ({}, None)
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
        "inputSnapshotSha256": input_snapshot_hash,
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


def numeric_stats(values: list[int | float]) -> dict[str, int | float] | None:
    if not values:
        return None
    total = sum(values)
    return {
        "total": total,
        "mean": round(total / len(values), 2),
        "median": round(statistics.median(values), 2),
    }


def aggregate_metrics(records: list[dict[str, Any]]) -> dict[str, Any]:
    metric_paths = {
        "requests": ("provider", "continuation", "requestCount"),
        "wallMs": ("time", "continuation_ms"),
        "inputTokens": ("tokens", "continuation", "inputTokens"),
        "cachedInputTokens": ("tokens", "continuation", "cachedInputTokens"),
        "uncachedInputTokens": ("tokens", "continuation", "uncachedInputTokens"),
        "outputTokens": ("tokens", "continuation", "outputTokens"),
        "commands": ("actions", "commandCount"),
        "failedCommands": ("actions", "failedCommandCount"),
        "projectionBytes": ("projection", "projectionBytes"),
    }

    def value_at(record: dict[str, Any], path: tuple[str, ...]) -> Any:
        value: Any = record
        for key in path:
            value = value[key]
        return value

    arms: dict[str, Any] = {}
    for arm in ("STD", "P1", "C1"):
        arm_records = [record for record in records if record.get("arm") == arm]
        if not arm_records:
            continue
        metrics: dict[str, Any] = {}
        for name, path in metric_paths.items():
            values = [value_at(record, path) for record in arm_records]
            metrics[name] = numeric_stats([value for value in values if value is not None])
        cache_rates = [
            float(record["tokens"]["continuation"]["cacheHitPercent"])
            for record in arm_records
        ]
        cached_total = metrics["cachedInputTokens"]["total"]
        input_total = metrics["inputTokens"]["total"]
        arms[arm] = {
            "runCount": len(arm_records),
            "successCount": sum(
                record.get("validation", {}).get("finalExitCode") == 0
                for record in arm_records
            ),
            "metrics": metrics,
            "cacheHitPercent": {
                "weighted": round(cached_total * 100 / input_total, 2)
                if input_total
                else None,
                "mean": round(sum(cache_rates) / len(cache_rates), 2),
                "median": round(statistics.median(cache_rates), 2),
            },
        }

    ratios: dict[str, Any] = {}
    if "P1" in arms and "C1" in arms:
        for name in metric_paths:
            previous = arms["P1"]["metrics"][name]
            candidate = arms["C1"]["metrics"][name]
            if previous is None or candidate is None:
                continue
            previous_runs = arms["P1"]["runCount"]
            candidate_runs = arms["C1"]["runCount"]
            ratios[name] = {
                "total": round(candidate["total"] / previous["total"], 3)
                if previous["total"]
                else None,
                "mean": round(
                    (candidate["total"] / candidate_runs)
                    / (previous["total"] / previous_runs),
                    3,
                )
                if previous["total"]
                else None,
                "median": round(candidate["median"] / previous["median"], 3)
                if previous["median"]
                else None,
            }
        ratios["cacheHitDeltaPercentagePoints"] = {
            key: round(
                arms["C1"]["cacheHitPercent"][key]
                - arms["P1"]["cacheHitPercent"][key],
                2,
            )
            for key in ("weighted", "mean", "median")
        }
    return {
        "schemaVersion": "taskspace-active-prefix-summary-v2",
        "arms": arms,
        "candidatePreviousRatios": ratios,
    }


def main() -> int:
    args = parse_args()
    if args.records:
        records_path = Path(args.records)
        metrics = aggregate_metrics(json.loads(records_path.read_text(encoding="utf-8")))
        output = Path(args.output) if args.output else records_path.with_name("summary.json")
        output.write_text(json.dumps(metrics, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(output)
        return 0

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
