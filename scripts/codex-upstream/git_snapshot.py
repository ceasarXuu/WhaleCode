#!/usr/bin/env python3
"""Git tree primitives for deterministic Codex vendor inventories."""

from __future__ import annotations

import hashlib
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


class GitError(RuntimeError):
    """Raised when a required Git object or command is unavailable."""


@dataclass(frozen=True)
class TreeEntry:
    mode: str
    object_type: str
    oid: str


@dataclass(frozen=True)
class DiffStat:
    status: str
    additions: int | None
    deletions: int | None
    binary: bool


def git(repo: Path, *args: str, input_bytes: bytes | None = None) -> bytes:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", "replace").strip()
        raise GitError(f"git {' '.join(args)} failed: {stderr}")
    return completed.stdout


def resolve_commit(repo: Path, revision: str) -> str:
    return git(repo, "rev-parse", f"{revision}^{{commit}}").decode().strip()


def resolve_tree(repo: Path, revision: str) -> str:
    return git(repo, "rev-parse", f"{revision}^{{tree}}").decode().strip()


def index_subtree(repo: Path, vendor_path: str) -> str:
    root_tree = git(repo, "write-tree").decode().strip()
    return git(repo, "rev-parse", f"{root_tree}:{vendor_path}").decode().strip()


def list_tree(repo: Path, treeish: str) -> dict[str, TreeEntry]:
    raw = git(repo, "ls-tree", "-rz", "--full-tree", "-r", treeish)
    entries: dict[str, TreeEntry] = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        metadata, path_bytes = record.split(b"\t", 1)
        mode, object_type, oid = metadata.decode("ascii").split(" ")
        path = path_bytes.decode("utf-8", "surrogateescape")
        entries[path] = TreeEntry(mode=mode, object_type=object_type, oid=oid)
    return entries


def read_blobs(repo: Path, oids: set[str]) -> dict[str, bytes]:
    if not oids:
        return {}
    ordered = sorted(oids)
    request = b"".join(f"{oid}\n".encode("ascii") for oid in ordered)
    raw = git(repo, "cat-file", "--batch", input_bytes=request)
    cursor = 0
    blobs: dict[str, bytes] = {}
    for requested_oid in ordered:
        header_end = raw.index(b"\n", cursor)
        header = raw[cursor:header_end].decode("ascii")
        cursor = header_end + 1
        oid, object_type, size_text = header.split(" ")
        if object_type != "blob":
            raise GitError(f"expected blob for {requested_oid}, found {object_type}")
        size = int(size_text)
        blobs[requested_oid] = raw[cursor : cursor + size]
        cursor += size + 1
        if oid != requested_oid:
            raise GitError(f"cat-file returned {oid} for requested {requested_oid}")
    return blobs


def diff_stats(
    repo: Path, baseline_tree: str, current_tree: str
) -> dict[str, DiffStat]:
    status_raw = git(
        repo,
        "diff",
        "--name-status",
        "-z",
        "--no-renames",
        baseline_tree,
        current_tree,
    )
    status_parts = status_raw.split(b"\0")
    statuses: dict[str, str] = {}
    for index in range(0, len(status_parts) - 1, 2):
        status = status_parts[index].decode("ascii")
        path = status_parts[index + 1].decode("utf-8", "surrogateescape")
        statuses[path] = {"A": "added", "M": "modified", "D": "deleted"}[status]

    numstat_raw = git(
        repo,
        "diff",
        "--numstat",
        "-z",
        "--no-renames",
        baseline_tree,
        current_tree,
    )
    counts: dict[str, tuple[int | None, int | None, bool]] = {}
    for record in numstat_raw.split(b"\0"):
        if not record:
            continue
        additions_text, deletions_text, path_bytes = record.split(b"\t", 2)
        path = path_bytes.decode("utf-8", "surrogateescape")
        binary = additions_text == b"-" or deletions_text == b"-"
        counts[path] = (
            None if binary else int(additions_text),
            None if binary else int(deletions_text),
            binary,
        )

    if statuses.keys() != counts.keys():
        raise GitError("name-status and numstat produced different path sets")
    return {path: DiffStat(statuses[path], *counts[path]) for path in sorted(statuses)}


def sha256(content: bytes | None) -> str | None:
    return None if content is None else hashlib.sha256(content).hexdigest()


def evidence_commits(
    repo: Path,
    import_commit: str,
    head_commit: str,
    vendor_path: str,
) -> dict[str, list[str]]:
    raw = git(
        repo,
        "log",
        "--format=%x1e%H%x00",
        "--name-only",
        "-z",
        "--no-renames",
        f"{import_commit}..{head_commit}",
        "--",
        vendor_path,
    )
    result: dict[str, list[str]] = defaultdict(list)
    prefix = f"{vendor_path}/"
    for segment in raw.split(b"\x1e"):
        if not segment:
            continue
        fields = segment.split(b"\0")
        commit = fields[0].decode("ascii")
        for raw_path in fields[2:]:
            path = raw_path.decode("utf-8", "surrogateescape").strip()
            if not path.startswith(prefix):
                continue
            relative = path[len(prefix) :]
            if commit not in result[relative]:
                result[relative].append(commit)
    return dict(result)
