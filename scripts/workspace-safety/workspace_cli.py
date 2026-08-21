"""Command-line parser for the workspace safety API."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from types import ModuleType


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Manage WhaleCode workspace isolation.")
    commands = parser.add_subparsers(dest="command", required=True)
    bootstrap = commands.add_parser("bootstrap")
    bootstrap_commands = bootstrap.add_subparsers(dest="bootstrap_command", required=True)
    plan_parser = bootstrap_commands.add_parser("plan")
    plan_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    plan_parser.add_argument("--json", action="store_true")
    apply_parser = bootstrap_commands.add_parser("apply")
    apply_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    apply_parser.add_argument("--expect", required=True)
    apply_parser.add_argument("--json", action="store_true")
    doctor_parser = commands.add_parser("doctor")
    doctor_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    doctor_parser.add_argument("--require-binary", action="store_true")
    doctor_parser.add_argument("--json", action="store_true")
    exec_parser = commands.add_parser("exec")
    exec_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    exec_parser.add_argument("exec_command", nargs=argparse.REMAINDER)
    ready_parser = commands.add_parser("require-ready")
    ready_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    ready_parser.add_argument("--json", action="store_true")
    return parser


def main(api: ModuleType) -> int:
    args = _parser().parse_args()
    try:
        if args.command == "doctor":
            result = api.run_doctor(args.repo_root, require_binary=args.require_binary)
            sys.stdout.write(api.render_json(result) if args.json else api.render_doctor_human(result))
            return 0 if result["status"] == "passed" else 5
        if args.command == "exec":
            command = args.exec_command[1:] if args.exec_command[:1] == ["--"] else args.exec_command
            return api.exec_ready(args.repo_root, command)
        if args.command == "require-ready":
            result = api.require_ready(args.repo_root)
            if args.json:
                sys.stdout.write(api.render_json(result))
            else:
                print(f"Workspace gate: {result['state']} ({result['reason_code']})")
                if not result["ready"]:
                    print(f"Recovery: {result['recovery_command']}")
            return 0 if result["ready"] else 7
        if args.bootstrap_command == "plan":
            plan = api.build_plan(args.repo_root)
            sys.stdout.write(api.render_json(plan) if args.json else api.render_human(plan))
            return 0 if plan["can_apply"] else 3
        result = api.apply_plan(args.repo_root, args.expect)
    except api.ApplyError as error:
        print(f"workspace apply failed [{error.code}]: {error}", file=sys.stderr)
        return 4
    except api.ExecError as error:
        print(f"workspace exec failed [{error.code}]: {error}", file=sys.stderr)
        return 6
    except (api.ContextError, OSError, ValueError) as error:
        print(f"workspace context failed: {error}", file=sys.stderr)
        return 2
    if args.json:
        sys.stdout.write(api.render_json(result))
    else:
        print(f"Workspace bootstrap applied: {result['workspace_id']} ({result['state']['code']})")
    return 0
