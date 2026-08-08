#!/usr/bin/env python3
"""Emit a read-only inventory of workspace isolation-sensitive surfaces."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit

SCHEMA_VERSION = 1
SCAN_ROOTS = ("scripts", ".vscode", "README.md", "docs/runbooks")
SKIPPED_DIRS = {".git", "__pycache__", "node_modules", "target"}
TEXT_SUFFIXES = {".md", ".json", ".py", ".ps1", ".sh", ".toml", ".yaml", ".yml"}

REFERENCE_RULES = {
    "legacy-whale-binary": re.compile(
        r"(?:~|\$HOME|\$\{HOME\}|%USERPROFILE%|Path\.home\(\))"
        r"[^\n]{0,80}\.whale[/\\]bin[/\\]whale(?:\.exe)?",
        re.IGNORECASE,
    ),
    "runtime-home": re.compile(r"\b(?:WHALE_HOME|CODEX_SQLITE_HOME)\b"),
    "cargo-target-override": re.compile(r"\bCARGO_TARGET_DIR\b"),
    "bazel-output-override": re.compile(r"--output_(?:base|user_root)\b"),
    "model-credential": re.compile(
        r"\b(?:DEEPSEEK|OPENAI|ANTHROPIC)_[A-Z0-9_]*API_KEY\b"
    ),
    "model-run-marker": re.compile(
        r"\b(?:ensure_deepseek_api_key|provider_boundary_proxy)\b|"
        r"api\.deepseek\.com",
        re.IGNORECASE,
    ),
}

SIDE_EFFECT_RULES = {
    "container": re.compile(r"\b(?:docker|podman)\b", re.IGNORECASE),
    "filesystem-write": re.compile(
        r"\b(?:Add-Content|Set-Content|Out-File|Copy-Item|Move-Item|"
        r"write_text|write_bytes|mkdir|makedirs)\b"
    ),
    "process-execution": re.compile(
        r"\b(?:subprocess\.(?:run|Popen)|Start-Process|cargo\s+(?:run|test|build)|"
        r"docker\s+run)\b",
        re.IGNORECASE,
    ),
}


class InventoryError(RuntimeError):
    """Raised when required repository metadata cannot be inspected safely."""


def _git(repo: Path, *args: str, allow_failure: bool = False) -> str | None:
    environment = os.environ.copy()
    environment["GIT_TERMINAL_PROMPT"] = "0"
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
        raise InventoryError(f"git {' '.join(args)} failed: {detail}")
    return completed.stdout.strip()


def _resolve_repo_root(start: Path) -> Path:
    root = _git(start.resolve(), "rev-parse", "--show-toplevel")
    if not root:
        raise InventoryError("git did not return a repository root")
    return Path(root).resolve()


def _resolve_git_path(repo: Path, value: str) -> Path:
    path = Path(value)
    return path.resolve() if path.is_absolute() else (repo / path).resolve()


def _sanitize_remote(raw: str) -> dict[str, str]:
    if "://" in raw:
        parsed = urlsplit(raw)
        hostname = parsed.hostname or "unknown-host"
        if parsed.port:
            hostname = f"{hostname}:{parsed.port}"
        endpoint = urlunsplit((parsed.scheme, hostname, parsed.path, "", ""))
        return {"kind": "url", "endpoint": endpoint}
    scp_like = re.fullmatch(r"(?:[^@/:]+@)?([^:/]+):(.+)", raw)
    if scp_like:
        return {
            "kind": "ssh",
            "endpoint": f"{scp_like.group(1)}/{scp_like.group(2)}",
        }
    return {"kind": "local", "endpoint": "local-path-redacted"}


def _git_inventory(repo: Path) -> dict:
    git_dir_raw = _git(repo, "rev-parse", "--git-dir")
    common_dir_raw = _git(repo, "rev-parse", "--git-common-dir")
    if not git_dir_raw or not common_dir_raw:
        raise InventoryError("git directory metadata is incomplete")
    git_dir = _resolve_git_path(repo, git_dir_raw)
    common_dir = _resolve_git_path(repo, common_dir_raw)
    branch = _git(repo, "symbolic-ref", "--quiet", "--short", "HEAD", allow_failure=True)
    head = _git(repo, "rev-parse", "--verify", "HEAD", allow_failure=True)
    remotes = []
    remote_names = (_git(repo, "remote", allow_failure=True) or "").splitlines()
    for name in sorted(filter(None, remote_names)):
        urls = (_git(repo, "remote", "get-url", "--all", name, allow_failure=True) or "").splitlines()
        remotes.append(
            {"name": name, "endpoints": [_sanitize_remote(url) for url in urls]}
        )
    return {
        "root": str(repo),
        "git_dir": str(git_dir),
        "git_common_dir": str(common_dir),
        "linked_worktree": git_dir != common_dir,
        "branch": branch,
        "detached_head": branch is None,
        "head": head,
        "remotes": remotes,
    }


def _walk_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for current, directories, names in os.walk(root):
        directories[:] = sorted(
            name for name in directories if name not in SKIPPED_DIRS
        )
        for name in sorted(names):
            files.append(Path(current) / name)
    return files


def _build_roots(repo: Path) -> list[dict[str, str]]:
    cargo_candidates: list[tuple[Path, dict]] = []
    bazel_candidates: list[Path] = []
    for path in _walk_files(repo):
        if path.name == "Cargo.toml":
            try:
                parsed = tomllib.loads(path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
                raise InventoryError(f"cannot parse {path.relative_to(repo)}: {error}") from error
            cargo_candidates.append((path, parsed))
        elif path.name in {"MODULE.bazel", "WORKSPACE", "WORKSPACE.bazel"}:
            bazel_candidates.append(path)

    cargo_workspaces = [item for item in cargo_candidates if "workspace" in item[1]]
    if not cargo_workspaces:
        cargo_workspaces = [item for item in cargo_candidates if "package" in item[1]]
    roots = [
        {
            "kind": "cargo",
            "root": str(path.parent.relative_to(repo)) or ".",
            "manifest": str(path.relative_to(repo)),
            "scope": _path_scope(path.relative_to(repo)),
        }
        for path, _ in cargo_workspaces
    ]
    roots.extend(
        {
            "kind": "bazel",
            "root": str(path.parent.relative_to(repo)) or ".",
            "manifest": str(path.relative_to(repo)),
            "scope": _path_scope(path.relative_to(repo)),
        }
        for path in bazel_candidates
    )
    return sorted(roots, key=lambda item: item["manifest"])


def _scan_paths(repo: Path) -> list[Path]:
    paths: list[Path] = []
    for relative in SCAN_ROOTS:
        candidate = repo / relative
        if candidate.is_file():
            paths.append(candidate)
        elif candidate.is_dir():
            paths.extend(
                path for path in _walk_files(candidate) if path.suffix.lower() in TEXT_SUFFIXES
            )
    return sorted(set(paths))


def _platform(path: Path) -> str:
    return {
        ".ps1": "powershell",
        ".sh": "linux-posix",
        ".py": "cross-platform-python",
        ".json": "editor-config",
    }.get(path.suffix.lower(), "documentation")


def _path_scope(relative: Path) -> str:
    parts = relative.parts
    if len(parts) >= 2 and parts[:2] == ("archive", "deprecated"):
        return "archived"
    if parts and parts[0] == "third_party":
        return "vendored"
    return "repository"


def _is_entrypoint(relative: Path, text: str) -> bool:
    parts = set(relative.parts)
    name = relative.name.lower()
    if "tests" in parts or "test" in parts or name.startswith(("test_", "test-")):
        return False
    if "lib" in parts or name.endswith(("-lib.ps1", "_lib.py")):
        return False
    if relative.parts and relative.parts[0] == ".vscode":
        return True
    if relative.suffix.lower() == ".py":
        return 'if __name__ == "__main__"' in text
    return relative.suffix.lower() in {".ps1", ".sh"}


def _binary_resolution(text: str) -> list[str]:
    modes = []
    if REFERENCE_RULES["legacy-whale-binary"].search(text):
        modes.append("legacy-user-slot")
    if "--whale-bin" in text.lower() or re.search(r"\bWhaleBin\b", text):
        modes.append("explicit-argument")
    if re.search(r"target[/\\](?:debug|release|dist)[/\\]whale(?:\.exe)?", text):
        modes.append("cargo-target")
    if re.search(r"(?:^|[\s;&])whale(?:\.exe)?(?:\s|$)", text, re.MULTILINE):
        modes.append("path-search")
    return modes or ["none-detected"]


def _home_resolution(text: str) -> list[str]:
    modes = []
    if re.search(r"\b(?:WHALE_HOME|CODEX_SQLITE_HOME)\b", text):
        modes.append("process-environment")
    if re.search(r"(?:~|\$HOME|%USERPROFILE%)[^\n]{0,40}\.whale", text, re.IGNORECASE):
        modes.append("legacy-user-home")
    return modes or ["none-detected"]


def _scan_surfaces(repo: Path) -> tuple[list[dict], list[dict]]:
    entrypoints: list[dict] = []
    references: list[dict] = []
    for path in _scan_paths(repo):
        if path.resolve() == Path(__file__).resolve():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            raise InventoryError(f"cannot read {path.relative_to(repo)}: {error}") from error
        relative = str(path.relative_to(repo))
        matched_rule_ids: set[str] = set()
        for rule_id, pattern in REFERENCE_RULES.items():
            for match in pattern.finditer(text):
                matched_rule_ids.add(rule_id)
                references.append(
                    {
                        "path": relative,
                        "line": text.count("\n", 0, match.start()) + 1,
                        "rule_id": rule_id,
                    }
                )
        relative_path = path.relative_to(repo)
        if (
            not relative.startswith(("scripts/", ".vscode/"))
            or not matched_rule_ids
            or not _is_entrypoint(relative_path, text)
        ):
            continue
        side_effects = [
            name for name, pattern in SIDE_EFFECT_RULES.items() if pattern.search(text)
        ]
        if path.name.startswith("install-"):
            side_effects.append("installation")
        entrypoints.append(
            {
                "path": relative,
                "platform": _platform(path),
                "matched_rule_ids": sorted(matched_rule_ids),
                "side_effects": sorted(set(side_effects)) or ["none-detected"],
                "binary_resolution": _binary_resolution(text),
                "runtime_home_resolution": _home_resolution(text),
                "model_request_risk": (
                    "possible"
                    if matched_rule_ids & {"model-credential", "model-run-marker"}
                    else "none-detected"
                ),
            }
        )
    return entrypoints, references


def collect_inventory(start: Path) -> dict:
    repo = _resolve_repo_root(start)
    entrypoints, references = _scan_surfaces(repo)
    reference_counts = Counter(item["rule_id"] for item in references)
    platform_counts = Counter(item["platform"] for item in entrypoints)
    build_roots = _build_roots(repo)
    return {
        "schema_version": SCHEMA_VERSION,
        "source": "working-tree-read-only",
        "repository": _git_inventory(repo),
        "build_roots": build_roots,
        "entrypoints": entrypoints,
        "shared_resource_references": references,
        "summary": {
            "build_root_count": len(build_roots),
            "entrypoint_count": len(entrypoints),
            "entrypoints_by_platform": dict(sorted(platform_counts.items())),
            "references_by_rule": dict(sorted(reference_counts.items())),
        },
    }


def render(document: dict) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Inspect workspace isolation-sensitive repository surfaces without writing state."
    )
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    try:
        sys.stdout.write(render(collect_inventory(args.repo_root)))
    except (InventoryError, OSError, ValueError) as error:
        print(f"workspace inventory failed: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
