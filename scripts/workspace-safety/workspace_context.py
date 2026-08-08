#!/usr/bin/env python3
"""Resolve, plan, and later manage an isolated WhaleCode workspace."""

from __future__ import annotations

import hashlib
import argparse
import json
import os
import re
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Any

MARKER_SCHEMA_VERSION = 1
PLAN_SCHEMA_VERSION = 1

UNBOOTSTRAPPED = "Unbootstrapped"
READY = "Ready"
STALE = "Stale"
CONFLICT = "Conflict"
DOCTOR_FAILED = "DoctorFailed"


class ContextError(RuntimeError):
    """Raised when a workspace cannot be resolved without unsafe assumptions."""


def canonical_path(path: Path | str) -> str:
    """Return the canonical absolute representation used by identity contracts."""

    return str(Path(path).expanduser().resolve(strict=True))


def sanitize_display_name(name: str) -> str:
    """Return a readable ASCII slug without using it as the unique identity."""

    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-._").lower()
    return slug or "workspace"


def derive_workspace_id(canonical_root: Path | str) -> str:
    """Derive a readable ID whose suffix prevents same-basename collisions."""

    root = canonical_path(canonical_root)
    display_name = sanitize_display_name(Path(root).name)
    digest = hashlib.sha256(os.fsencode(root)).hexdigest()[:10]
    return f"{display_name}-{digest}"


def _result(code: str, reason_code: str) -> dict[str, str]:
    return {"code": code, "reason_code": reason_code}


def evaluate_state(
    marker: Mapping[str, Any] | None,
    current: Mapping[str, Any],
) -> dict[str, str]:
    """Evaluate the five-state contract without reading or writing the filesystem."""

    if marker is None:
        return _result(UNBOOTSTRAPPED, "marker_missing")
    required_marker = {
        "schema_version",
        "workspace_id",
        "canonical_root",
        "git_common_dir",
        "branch",
        "resources",
        "last_doctor",
    }
    if not required_marker.issubset(marker):
        return _result(CONFLICT, "marker_invalid")
    if marker.get("schema_version") != MARKER_SCHEMA_VERSION:
        return _result(CONFLICT, "marker_schema_unsupported")
    if marker.get("workspace_id") != current.get("workspace_id"):
        return _result(CONFLICT, "workspace_id_collision")
    if marker.get("canonical_root") != current.get("canonical_root"):
        return _result(CONFLICT, "workspace_root_collision")
    if current.get("detached_head"):
        return _result(STALE, "detached_head")
    if marker.get("git_common_dir") != current.get("git_common_dir"):
        return _result(STALE, "git_common_dir_changed")
    if marker.get("branch") != current.get("branch"):
        return _result(STALE, "branch_changed")
    current_resources = current.get("resources")
    if current_resources is not None and marker.get("resources") != current_resources:
        return _result(STALE, "resource_paths_changed")
    doctor = marker.get("last_doctor")
    if not isinstance(doctor, Mapping) or doctor.get("status") != "passed":
        return _result(DOCTOR_FAILED, "last_doctor_not_passed")
    return _result(READY, "workspace_ready")


def _git(repo: Path, *args: str, allow_failure: bool = False) -> str | None:
    environment = os.environ.copy()
    environment["GIT_TERMINAL_PROMPT"] = "0"
    environment["GIT_OPTIONAL_LOCKS"] = "0"
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=environment,
    )
    if completed.returncode != 0:
        if allow_failure:
            return None
        detail = completed.stderr.strip() or f"exit {completed.returncode}"
        raise ContextError(f"git {' '.join(args)} failed: {detail}")
    return completed.stdout.strip()


def _resolve_git_path(repo: Path, raw: str) -> str:
    path = Path(raw)
    resolved = path.resolve() if path.is_absolute() else (repo / path).resolve()
    return str(resolved)


def _environment_root(
    environment: Mapping[str, str], key: str, fallback: Path
) -> Path:
    raw = environment.get(key)
    path = Path(raw).expanduser() if raw else fallback
    if not path.is_absolute():
        raise ContextError(f"{key} must be an absolute path when set")
    return path.resolve(strict=False)


def resolve_context(
    start: Path | str,
    environment: Mapping[str, str] | None = None,
) -> dict[str, Any]:
    """Resolve Git and XDG facts without creating any filesystem state."""

    supplied_environment = environment if environment is not None else os.environ
    start_path = Path(start).expanduser().resolve(strict=True)
    if start_path.is_file():
        start_path = start_path.parent
    root_raw = _git(start_path, "rev-parse", "--show-toplevel")
    git_dir_raw = _git(start_path, "rev-parse", "--git-dir")
    common_dir_raw = _git(start_path, "rev-parse", "--git-common-dir")
    if not root_raw or not git_dir_raw or not common_dir_raw:
        raise ContextError("Git did not return complete workspace metadata")
    root = Path(root_raw).resolve(strict=True)
    branch = _git(root, "symbolic-ref", "--quiet", "--short", "HEAD", allow_failure=True)
    head = _git(root, "rev-parse", "--verify", "HEAD", allow_failure=True)
    dirty = bool(_git(root, "status", "--porcelain", "--untracked-files=normal"))

    home = _environment_root(supplied_environment, "HOME", Path.home())
    xdg_state = _environment_root(
        supplied_environment, "XDG_STATE_HOME", home / ".local/state"
    )
    xdg_data = _environment_root(
        supplied_environment, "XDG_DATA_HOME", home / ".local/share"
    )
    workspace_id = derive_workspace_id(root)
    state_root = xdg_state / "whalecode/workspaces" / workspace_id
    data_root = xdg_data / "whalecode/workspaces" / workspace_id
    resources = {
        "state_root": str(state_root),
        "runtime_home": str(state_root / "home"),
        "data_root": str(data_root),
        "binary_dir": str(data_root / "bin"),
    }
    return {
        "workspace_id": workspace_id,
        "display_name": root.name,
        "canonical_root": str(root),
        "git_dir": _resolve_git_path(root, git_dir_raw),
        "git_common_dir": _resolve_git_path(root, common_dir_raw),
        "branch": branch,
        "detached_head": branch is None,
        "head": head,
        "working_tree_dirty": dirty,
        "resources": resources,
        "marker_path": str(state_root / "workspace-identity.json"),
        "legacy_home": str(home / ".whale"),
    }


def _load_marker(path: Path) -> tuple[Mapping[str, Any] | None, dict[str, Any]]:
    if not path.exists():
        return None, {"present": False, "sha256": None, "parse_status": "missing"}
    try:
        content = path.read_bytes()
    except OSError as error:
        raise ContextError(f"cannot read workspace marker: {type(error).__name__}") from error
    digest = hashlib.sha256(content).hexdigest()
    try:
        value = json.loads(content)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return {}, {"present": True, "sha256": digest, "parse_status": "invalid_json"}
    if not isinstance(value, dict):
        return {}, {"present": True, "sha256": digest, "parse_status": "invalid_type"}
    return value, {"present": True, "sha256": digest, "parse_status": "parsed"}


def _fingerprint(value: Mapping[str, Any]) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def build_plan(
    start: Path | str,
    environment: Mapping[str, str] | None = None,
) -> dict[str, Any]:
    """Build a deterministic bootstrap plan without writing files or logs."""

    context = resolve_context(start, environment)
    marker, marker_summary = _load_marker(Path(context["marker_path"]))
    state_input = {
        key: context[key]
        for key in (
            "workspace_id",
            "canonical_root",
            "git_common_dir",
            "branch",
            "detached_head",
            "resources",
        )
    }
    state = evaluate_state(marker, state_input)
    blocking_reason_codes = []
    if context["detached_head"]:
        blocking_reason_codes.append("detached_head")
    if state["code"] == CONFLICT:
        blocking_reason_codes.append(state["reason_code"])
    warnings = []
    if context["working_tree_dirty"]:
        warnings.append("working_tree_dirty")
    if Path(context["legacy_home"]).exists():
        warnings.append("legacy_home_present_no_action")

    binding = {
        "schema_version": PLAN_SCHEMA_VERSION,
        "canonical_root": context["canonical_root"],
        "workspace_id": context["workspace_id"],
        "git_common_dir": context["git_common_dir"],
        "branch": context["branch"],
        "resources": context["resources"],
        "marker_path": context["marker_path"],
        "existing_marker_sha256": marker_summary["sha256"],
    }
    directory_actions = [
        {
            "action": "ensure_directory",
            "target": path,
            "disposition": "reuse" if Path(path).is_dir() else "create",
        }
        for path in context["resources"].values()
    ]
    marker_disposition = "create" if not marker_summary["present"] else "update"
    if state["code"] == READY:
        marker_disposition = "reuse"
    return {
        "schema_version": PLAN_SCHEMA_VERSION,
        "source": "workspace-bootstrap-plan-read-only",
        "context": context,
        "existing_marker": marker_summary,
        "state": state,
        "can_apply": not blocking_reason_codes,
        "blocking_reason_codes": blocking_reason_codes,
        "warnings": warnings,
        "actions": [
            *directory_actions,
            {
                "action": "write_marker",
                "target": context["marker_path"],
                "disposition": marker_disposition,
            },
            {"action": "run_doctor", "target": context["workspace_id"]},
        ],
        "untouched": [context["legacy_home"], context["canonical_root"]],
        "fingerprint_basis": binding,
        "fingerprint": _fingerprint(binding),
    }


def render_json(document: Mapping[str, Any]) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def render_human(plan: Mapping[str, Any]) -> str:
    context = plan["context"]
    lines = [
        "Workspace bootstrap plan",
        f"State: {plan['state']['code']} ({plan['state']['reason_code']})",
        f"Workspace: {context['display_name']} ({context['workspace_id']})",
        f"Root: {context['canonical_root']}",
        f"Branch: {context['branch'] or '<detached>'}",
        f"Runtime home: {context['resources']['runtime_home']}",
        f"Binary dir: {context['resources']['binary_dir']}",
        "Actions:",
    ]
    lines.extend(
        f"- {item['action']} {item['target']} ({item.get('disposition', 'planned')})"
        for item in plan["actions"]
    )
    if plan["warnings"]:
        lines.append("Warnings: " + ", ".join(plan["warnings"]))
    if plan["blocking_reason_codes"]:
        lines.append("Blocked: " + ", ".join(plan["blocking_reason_codes"]))
    lines.append(f"Fingerprint: {plan['fingerprint']}")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Manage WhaleCode workspace isolation.")
    commands = parser.add_subparsers(dest="command", required=True)
    bootstrap = commands.add_parser("bootstrap")
    bootstrap_commands = bootstrap.add_subparsers(dest="bootstrap_command", required=True)
    plan_parser = bootstrap_commands.add_parser("plan")
    plan_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    plan_parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        plan = build_plan(args.repo_root)
    except (ContextError, OSError, ValueError) as error:
        print(f"workspace context failed: {error}", file=sys.stderr)
        return 2
    sys.stdout.write(render_json(plan) if args.json else render_human(plan))
    return 0 if plan["can_apply"] else 3


if __name__ == "__main__":
    sys.exit(main())
