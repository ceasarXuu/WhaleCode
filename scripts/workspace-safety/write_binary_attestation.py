#!/usr/bin/env python3
"""Write the Linux Whale binary attestation consumed by workspace doctor."""

from __future__ import annotations

import argparse
import hashlib
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from workspace_persistence import atomic_write_json


def _git(repo: Path, *args: str) -> str:
    environment = {**os.environ, "GIT_OPTIONAL_LOCKS": "0", "GIT_TERMINAL_PROMPT": "0"}
    result = subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True,
        env=environment,
    )
    return result.stdout.strip()


def build_attestation(binary: Path, repo: Path, build_command: str) -> dict[str, object]:
    binary = binary.resolve(strict=True)
    repo = repo.resolve(strict=True)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("binary must be an executable file")
    if _git(repo, "status", "--porcelain"):
        raise ValueError("cannot attest a binary from a dirty worktree")
    probe = subprocess.run(
        [str(binary), "--version"], check=False, capture_output=True, text=True
    )
    probe_output = (probe.stdout or probe.stderr).strip()
    if probe.returncode != 0 or not probe_output:
        raise ValueError("binary version probe failed")
    source_record = _git(
        repo, "log", "-1", "--format=%H%x00%cI", "--", "third_party/codex-cli"
    ).split("\0", 1)
    if len(source_record) != 2:
        raise ValueError("cannot resolve Codex source provenance")
    stat = binary.stat()
    return {
        "schema_version": 2,
        "status": "pass",
        "producer": "write_binary_attestation.py",
        "repo_root": str(repo),
        "current_git_head": _git(repo, "rev-parse", "HEAD"),
        "head_tree_id": _git(repo, "rev-parse", "HEAD^{tree}"),
        "codex_tree_id": _git(repo, "rev-parse", "HEAD:third_party/codex-cli"),
        "worktree_clean": True,
        "codex_source_latest_commit": source_record[0],
        "codex_source_latest_commit_time_utc": source_record[1],
        "whale_bin": str(binary),
        "whale_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "whale_binary_last_write_utc": datetime.fromtimestamp(
            stat.st_mtime, timezone.utc
        ).isoformat(),
        "build_command": build_command,
        "executable_probe": {
            "exit_code": probe.returncode,
            "output": probe_output,
            "output_sha256": hashlib.sha256(probe_output.encode()).hexdigest(),
        },
        "generated_at": datetime.now(timezone.utc).isoformat(),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Attest a local Whale binary.")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--build-command", required=True)
    args = parser.parse_args()
    try:
        document = build_attestation(args.binary, args.repo_root, args.build_command)
        path = Path(f"{args.binary.resolve(strict=True)}.build-attestation.json")
        atomic_write_json(path, document)
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"binary attestation failed: {error}", file=os.sys.stderr)
        return 1
    print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
