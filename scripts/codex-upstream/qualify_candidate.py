#!/usr/bin/env python3
"""Qualify an immutable upstream Codex candidate outside the production vendor."""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import os
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

from generate_overlay_inventory import VENDOR_PATH
from git_snapshot import git, index_subtree, resolve_commit, resolve_tree
from metadata_contract import validate_candidate

RELEASE_TAG = "rust-v0.147.0"
RELEASE_DATE = "2026-08-07"
CANDIDATE_TARGET = "be6e8eac029b183056b7e4402879f15d2c85f61b"
OUTPUT_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-candidate.json"
EVIDENCE_DIR = "docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.147.0"
QUALIFICATION_ENVIRONMENT = {
    "INSTA_UPDATE": "no",
    "NEXTEST_PROFILE": "local",
    "RUST_MIN_STACK": "8388608",
}
PROXY_ENVIRONMENT_KEYS = frozenset(
    {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}
)
COMMANDS = (
    ("01-fmt", ("cargo", "fmt", "--all", "--", "--check")),
    (
        "02-cli-check",
        ("cargo", "check", "-p", "codex-cli", "--bin", "codex", "--offline"),
    ),
    (
        "03-code-mode-host-build",
        (
            "cargo",
            "build",
            "--offline",
            "-p",
            "codex-code-mode-host",
            "--bin",
            "codex-code-mode-host",
        ),
    ),
    (
        "04-core-tests",
        ("cargo", "nextest", "run", "--no-fail-fast", "-p", "codex-core"),
    ),
    (
        "05-app-server-tests",
        (
            "cargo",
            "nextest",
            "run",
            "--no-fail-fast",
            "-p",
            "codex-app-server",
        ),
    ),
    (
        "06-tui-tests",
        ("cargo", "nextest", "run", "--no-fail-fast", "-p", "codex-tui"),
    ),
)
PACKAGE_TEST_IDS = frozenset({"04-core-tests", "05-app-server-tests", "06-tui-tests"})
PREPARATION_COMMAND = ("cargo", "fetch")


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _qualification_environment(source: dict[str, str] | None = None) -> dict[str, str]:
    environment = dict(os.environ if source is None else source)
    for key in tuple(environment):
        if key.lower() in PROXY_ENVIRONMENT_KEYS:
            environment.pop(key)
    environment.update(QUALIFICATION_ENVIRONMENT)
    return environment


def _normalize_output(output: str, repo: Path, candidate_root: Path) -> str:
    normalized = output.replace(str(candidate_root), "<candidate>")
    normalized = normalized.replace(str(repo), "<repo>")
    normalized = normalized.replace(str(Path.home()), "<home>")
    return "\n".join(line.rstrip() for line in normalized.splitlines())


def _run_text(command: tuple[str, ...], cwd: Path) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        text=True,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"{' '.join(command)} failed:\n{completed.stdout}")
    return completed.stdout.strip()


def _toolchain(candidate_root: Path) -> dict[str, str]:
    codex_rs = candidate_root / "codex-rs"
    return {
        "cargo": _run_text(("cargo", "--version"), codex_rs),
        "nextest": _run_text(("cargo", "nextest", "--version"), codex_rs),
        "rustc": _run_text(("rustc", "--version"), codex_rs),
    }


def _export_candidate(repo: Path, destination: Path) -> None:
    archive = destination / "candidate.tar"
    with archive.open("wb") as output:
        completed = subprocess.run(
            ["git", "-C", str(repo), "archive", "--format=tar", CANDIDATE_TARGET],
            stdout=output,
            stderr=subprocess.PIPE,
            check=False,
        )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.decode("utf-8", "replace"))
    completed = subprocess.run(
        ["tar", "-xf", str(archive), "-C", str(destination)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.decode("utf-8", "replace"))
    archive.unlink()


def _run_qualification(
    repo: Path, candidate_root: Path, evidence_dir: Path
) -> list[dict]:
    results: list[dict] = []
    codex_rs = candidate_root / "codex-rs"
    for command_id, command in COMMANDS:
        logging.info("running candidate qualification %s", command_id)
        proxy_environment = (
            "scrubbed" if command_id in PACKAGE_TEST_IDS else "inherited"
        )
        environment = (
            _qualification_environment()
            if command_id in PACKAGE_TEST_IDS
            else {**os.environ, **QUALIFICATION_ENVIRONMENT}
        )
        completed = subprocess.run(
            command,
            cwd=codex_rs,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            text=True,
        )
        normalized = _normalize_output(completed.stdout, repo, candidate_root)
        evidence_path = evidence_dir / f"{command_id}.log"
        evidence_path.parent.mkdir(parents=True, exist_ok=True)
        evidence_path.write_text(
            f"command: {' '.join(command)}\n"
            f"cwd: codex-rs\n"
            f"environment: {json.dumps(QUALIFICATION_ENVIRONMENT, sort_keys=True)}\n"
            f"proxy_environment: {proxy_environment}\n"
            f"exit_code: {completed.returncode}\n\n{normalized}",
            encoding="utf-8",
        )
        results.append(
            {
                "command": list(command),
                "cwd": "codex-rs",
                "environment": QUALIFICATION_ENVIRONMENT,
                "proxy_environment": proxy_environment,
                "evidence": evidence_path.relative_to(repo).as_posix(),
                "exit_code": completed.returncode,
                "id": command_id,
                "result": "passed" if completed.returncode == 0 else "failed",
            }
        )
    return results


def _prepare_dependencies(repo: Path, candidate_root: Path, evidence_dir: Path) -> None:
    logging.info("preparing candidate dependencies")
    completed = subprocess.run(
        PREPARATION_COMMAND,
        cwd=candidate_root / "codex-rs",
        env={**os.environ, **QUALIFICATION_ENVIRONMENT},
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        text=True,
    )
    evidence_path = evidence_dir / "00-dependency-fetch.log"
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    normalized = _normalize_output(completed.stdout, repo, candidate_root)
    evidence_path.write_text(
        f"command: {' '.join(PREPARATION_COMMAND)}\n"
        "cwd: codex-rs\n"
        "proxy_environment: inherited\n"
        f"exit_code: {completed.returncode}\n\n{normalized}",
        encoding="utf-8",
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"candidate dependency preparation failed; see {evidence_path.relative_to(repo)}"
        )


def _manifest(repo: Path, candidate_root: Path, commands: list[dict]) -> dict:
    license_content = (candidate_root / "LICENSE").read_bytes()
    counts = Counter(entry["result"] for entry in commands)
    return {
        "schema_version": 1,
        "release_tag": RELEASE_TAG,
        "commit_sha": resolve_commit(repo, CANDIDATE_TARGET),
        "tree_sha": resolve_tree(repo, CANDIDATE_TARGET),
        "release_date": RELEASE_DATE,
        "license_path": "LICENSE",
        "license_sha256": hashlib.sha256(license_content).hexdigest(),
        "source_method": "git-archive",
        "source_object_verified": True,
        "toolchain": _toolchain(candidate_root),
        "qualification_commands": commands,
        "production_vendor_unchanged": True,
        "model_request_count": 0,
        "summary": {
            "command_count": len(commands),
            "by_result": dict(sorted(counts.items())),
        },
    }


def run(repo: Path) -> int:
    before = index_subtree(repo, VENDOR_PATH)
    with tempfile.TemporaryDirectory(prefix="whale-codex-0.147-") as temp_dir:
        candidate_root = Path(temp_dir)
        _export_candidate(repo, candidate_root)
        _prepare_dependencies(repo, candidate_root, repo / EVIDENCE_DIR)
        commands = _run_qualification(repo, candidate_root, repo / EVIDENCE_DIR)
        manifest = _manifest(repo, candidate_root, commands)
    after = index_subtree(repo, VENDOR_PATH)
    if before != after:
        raise RuntimeError("production vendor index tree changed during qualification")
    errors = validate_candidate(manifest)
    if errors:
        raise RuntimeError("; ".join(errors))
    output = repo / OUTPUT_PATH
    output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    logging.info("wrote %s with vendor tree unchanged at %s", OUTPUT_PATH, after)
    return 0 if not manifest["summary"]["by_result"].get("failed") else 1


def check(repo: Path) -> int:
    document = json.loads((repo / OUTPUT_PATH).read_text(encoding="utf-8"))
    errors = validate_candidate(document)
    if document.get("commit_sha") != resolve_commit(repo, CANDIDATE_TARGET):
        errors.append("candidate commit does not match configured target")
    if document.get("tree_sha") != resolve_tree(repo, CANDIDATE_TARGET):
        errors.append("candidate tree does not match configured target")
    license_content = git(repo, "show", f"{CANDIDATE_TARGET}:LICENSE")
    if document.get("license_sha256") != hashlib.sha256(license_content).hexdigest():
        errors.append("candidate license digest is stale")
    expected_ids = [command_id for command_id, _ in COMMANDS]
    actual_ids = [entry.get("id") for entry in document.get("qualification_commands", [])]
    if actual_ids != expected_ids:
        errors.append("candidate qualification command set is stale")
    for entry in document.get("qualification_commands", []):
        if not (repo / entry.get("evidence", "")).is_file():
            errors.append(f"missing candidate evidence for {entry.get('id')}")
    if errors:
        for error in errors:
            logging.error("%s", error)
        return 1
    logging.info("candidate manifest and evidence are current")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--run", action="store_true")
    action.add_argument("--check", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    try:
        return run(_repo_root()) if args.run else check(_repo_root())
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("candidate qualification failed: %s", error)
        return 2


if __name__ == "__main__":
    sys.exit(main())
