#!/usr/bin/env python3
"""Reject active-code references to retired TaskSpace protocols."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path


ACTIVE_ROOTS = (
    "third_party/codex-cli/codex-rs/core/src",
    "third_party/codex-cli/codex-rs/core/tests",
    "third_party/codex-cli/codex-rs/protocol/src",
    "third_party/codex-cli/codex-rs/tools/src",
    "third_party/codex-cli/codex-rs/state/src",
    "third_party/codex-cli/codex-rs/state/migrations",
    "third_party/codex-cli/codex-rs/cli/src",
    "third_party/codex-cli/codex-rs/cli/tests",
    "third_party/codex-cli/codex-rs/tui/src",
    "third_party/codex-cli/codex-rs/app-server-protocol/src",
    "third_party/codex-cli/codex-rs/app-server-protocol/schema",
    "apps",
)

ACTIVE_FILES = (
    "scripts/action-map-store-export-lib.ps1",
    "scripts/action-map-observability-report-lib.ps1",
)

SCANNED_SUFFIXES = {
    ".json",
    ".md",
    ".ps1",
    ".rs",
    ".toml",
    ".ts",
    ".tsx",
    ".yaml",
    ".yml",
}

RETIRED_SYMBOLS = (
    "taskspace_control",
    "TaskSpaceControlResult",
    "TaskSpaceResponse",
    "ToolSequencePreflight",
    "ProviderToolResponsePreflight",
    "TaskSpaceGateResult",
    "TaskSpaceSequence",
    "sequence_manifest",
    "prepared_sibling",
    "taskspace_core_protocol",
    "taskspace-advanced",
    "TaskSpaceSkillSnapshot",
    "ActionReservation",
    "action_reservations",
    "response_call_index",
    "reservation_id",
    "TaskSpaceEventStore",
    "TaskContextEventRecorded",
    "TaskContextOwnershipChanged",
    "TaskSpaceCompactionCheckpoint",
    "TaskSpaceMapEdge",
    "TaskSpaceCompletionRecord",
    "TaskSpaceBlockRecord",
    "TaskSpaceResultRef",
    "TaskSpaceEvidenceRef",
    "TaskSpaceTerminalRecord",
    "ActionMapSnapshotEdge",
    "ActionMapSnapshotResult",
    "ActionMapSnapshotEvidenceRef",
    "ActionMapSnapshotMaintenanceBarrier",
    "ActionMapSnapshotNodeEvent",
    "ActionMapSnapshotSentinel",
    "ActionMapSnapshotTrace",
    "taskspace-canonical-map-v2",
    "taskspace-canonical-map-v3",
    "TaskSpaceMapProjectionR7V1",
    "TaskSpaceMapHandleR7V1",
    "TaskSpaceMapExportR7V2",
    "source_refs",
    "completion_records",
    "block_records",
    "action_records",
    "result_refs",
    "terminal_record",
    "terminal_history",
    "graph_events",
    "node_events",
    "detail_fold",
)


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    symbol: str


def scan_zero_base(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    paths: set[Path] = set()
    for relative_root in ACTIVE_ROOTS:
        active_root = root / relative_root
        if not active_root.exists():
            continue
        for path in sorted(active_root.rglob("*")):
            if not path.is_file() or path.suffix not in SCANNED_SUFFIXES:
                continue
            paths.add(path)
    paths.update(root / relative_path for relative_path in ACTIVE_FILES)

    for path in sorted(paths):
        if not path.is_file() or path.suffix not in SCANNED_SUFFIXES:
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        if path.suffix == ".rs":
            lines = lines[: next((index for index, line in enumerate(lines) if line.strip() == "#[cfg(test)]"), len(lines))]
        for line_number, line in enumerate(lines, start=1):
            for symbol in RETIRED_SYMBOLS:
                if symbol in line:
                    findings.append(
                        Finding(
                            path=path.relative_to(root),
                            line=line_number,
                            symbol=symbol,
                        )
                    )
    return findings


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check that retired TaskSpace protocols are absent from active surfaces."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root",
    )
    return parser.parse_args()


def main() -> int:
    root = parse_args().root.resolve()
    findings = scan_zero_base(root)
    if findings:
        print("TaskSpace zero-base gate: FAIL")
        for finding in findings:
            print(f"{finding.path}:{finding.line}: retired symbol {finding.symbol}")
        return 1
    print("TaskSpace zero-base gate: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
