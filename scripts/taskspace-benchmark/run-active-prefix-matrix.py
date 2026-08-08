#!/usr/bin/env python3
"""Run the natural active-prefix comparison in isolated Docker containers."""

from __future__ import annotations

import argparse
import concurrent.futures
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts/workspace-safety"))

from workspace_entrypoint import WorkspacePreflightError, require_ready


SAMPLE_DIR = (
    REPO_ROOT
    / "benchmarks/taskspace/map-compression/samples/subscription-billing-active-prefix"
)
THREAD_ID = "019f5c4c-4689-7d71-801d-9e888ddfff4b"
ROLLOUT_NAME = f"rollout-2026-07-13T16-25-30-{THREAD_ID}.jsonl"
IMAGE = "whalecode/taskspace-benchmark:r5"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repeats", type=int, default=1)
    parser.add_argument("--arms", nargs="+", choices=("STD", "P1", "C1"), default=["STD", "P1", "C1"])
    parser.add_argument("--run-root", default="")
    parser.add_argument(
        "--candidate-app-server",
        default="target/r5-map-compression/candidate-C1/bin/whale-app-server",
    )
    parser.add_argument(
        "--previous-app-server",
        default="target/r5-map-compression/matched-control-P1/bin/whale-app-server",
    )
    parser.add_argument("--env-file", default=".env.local")
    parser.add_argument("--max-parallel", type=int, default=3)
    parser.add_argument("--timeout-seconds", type=int, default=1000)
    parser.add_argument("--plan-only", action="store_true")
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run(
    command: list[str],
    *,
    cwd: Path = REPO_ROOT,
    env: dict[str, str] | None = None,
    stdout_path: Path | None = None,
    stderr_path: Path | None = None,
    timeout: int | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    stdout_file = stdout_path.open("w", encoding="utf-8") if stdout_path else subprocess.PIPE
    stderr_file = stderr_path.open("w", encoding="utf-8") if stderr_path else subprocess.PIPE
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            stdout=stdout_file,
            stderr=stderr_file,
            timeout=timeout,
            check=False,
        )
    finally:
        if stdout_path:
            stdout_file.close()
        if stderr_path:
            stderr_file.close()
    if check and result.returncode != 0:
        stdout = result.stdout if isinstance(result.stdout, str) else ""
        stderr = result.stderr if isinstance(result.stderr, str) else ""
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}\n{stdout}{stderr}")
    return result


def assert_sample_contract(contract: dict[str, Any]) -> None:
    prefix = SAMPLE_DIR / contract["prefix"]["path"]
    patch = SAMPLE_DIR / contract["workspace"]["patch"]
    prompt = SAMPLE_DIR / contract["continuation"]["prompt"]
    if sha256(prefix) != contract["prefix"]["compressed_sha256"]:
        raise RuntimeError("compressed rollout prefix hash mismatch")
    with gzip.open(prefix, "rb") as handle:
        decompressed_hash = hashlib.sha256(handle.read()).hexdigest()
    if decompressed_hash != contract["prefix"]["decompressed_sha256"]:
        raise RuntimeError("decompressed rollout prefix hash mismatch")
    if sha256(patch) != contract["workspace"]["patch_sha256"]:
        raise RuntimeError("workspace patch hash mismatch")
    if sha256(prompt) != contract["continuation"]["prompt_sha256"]:
        raise RuntimeError("continuation prompt hash mismatch")


def prepare_case(run_root: Path, arm: str, repeat: int, contract: dict[str, Any]) -> Path:
    case_dir = run_root / arm / f"repeat-{repeat:03d}"
    if case_dir.exists():
        raise RuntimeError(f"refusing to overwrite existing case: {case_dir}")
    repo = case_dir / "repo"
    artifacts = case_dir / "artifacts"
    sessions = artifacts / "home/.whale/sessions/2026/07/13"
    shutil.copytree(REPO_ROOT / contract["workspace"]["base_fixture"], repo)
    sessions.mkdir(parents=True)

    run(["git", "init", "-q", "-b", "main"], cwd=repo)
    run(["git", "config", "user.name", "TaskSpace Benchmark"], cwd=repo)
    run(["git", "config", "user.email", "taskspace-benchmark@example.local"], cwd=repo)
    run(["git", "add", "."], cwd=repo)
    git_env = os.environ.copy()
    git_env["GIT_AUTHOR_DATE"] = "2026-07-14T00:25:30+08:00"
    git_env["GIT_COMMITTER_DATE"] = "2026-07-14T00:25:30+08:00"
    run(["git", "commit", "-q", "-m", "baseline fixture"], cwd=repo, env=git_env)
    head = run(["git", "rev-parse", "HEAD"], cwd=repo).stdout.strip()
    if head != contract["workspace"]["base_commit"]:
        raise RuntimeError(f"fixture commit mismatch: {head}")
    run(["git", "apply", str((SAMPLE_DIR / contract["workspace"]["patch"]).resolve())], cwd=repo)

    prefix_path = SAMPLE_DIR / contract["prefix"]["path"]
    rollout_path = sessions / ROLLOUT_NAME
    with gzip.open(prefix_path, "rb") as source, rollout_path.open("wb") as target:
        shutil.copyfileobj(source, target)
    if sha256(rollout_path) != contract["prefix"]["decompressed_sha256"]:
        raise RuntimeError("materialized rollout hash mismatch")

    whale_home = artifacts / "home/.whale"
    (whale_home / "config.toml").write_text(
        '[projects."/workspace"]\ntrust_level = "trusted"\n', encoding="utf-8"
    )
    (whale_home / "installation_id").write_text(
        "0311a41f-55f4-4778-afc3-d1da29aa5002\n", encoding="utf-8"
    )
    shutil.copy2(SAMPLE_DIR / contract["continuation"]["prompt"], artifacts / "continuation-prompt.txt")
    return case_dir


def inspect_container(name: str, path: Path) -> None:
    run(["docker", "inspect", name], stdout_path=path)


def validator(case_dir: Path, stage: str) -> int:
    artifacts = case_dir / "artifacts"
    name = f"whale-r5-s1-{stage}-{uuid.uuid4().hex[:10]}"
    result = run(
        [
            "docker",
            "run",
            "--name",
            name,
            "--cpus",
            "2",
            "--memory",
            "2g",
            "--user",
            "1000:1000",
            "--workdir",
            "/workspace",
            "-v",
            f"{(case_dir / 'repo').resolve()}:/workspace:ro",
            IMAGE,
            "bash",
            "-lc",
            "python -m pytest tests -q",
        ],
        stdout_path=artifacts / f"{stage}-validation.stdout.log",
        stderr_path=artifacts / f"{stage}-validation.stderr.log",
        check=False,
        timeout=180,
    )
    inspect_container(name, artifacts / f"container-inspect-{stage}-validator.json")
    (artifacts / f"{stage}-validation.exit-code.txt").write_text(
        f"{result.returncode}\n", encoding="utf-8"
    )
    return result.returncode


def run_agent(case_dir: Path, arm: str, args: argparse.Namespace) -> int:
    artifacts = case_dir / "artifacts"
    binary = Path(args.previous_app_server if arm == "P1" else args.candidate_app_server)
    if not binary.is_absolute():
        binary = REPO_ROOT / binary
    mode = "standard" if arm == "STD" else "taskspace"
    name = f"whale-r5-s1-{arm.lower()}-{uuid.uuid4().hex[:10]}"
    command = [
        "docker",
        "run",
        "--name",
        name,
        "--cpus",
        "4",
        "--memory",
        "8g",
        "--memory-swap",
        "8g",
        "--pids-limit",
        "512",
        "--user",
        "1000:1000",
        "--workdir",
        "/workspace",
        "--log-driver",
        "local",
        "--log-opt",
        "max-size=10m",
        "--log-opt",
        "max-file=3",
        "-v",
        f"{(case_dir / 'repo').resolve()}:/workspace:rw",
        "-v",
        f"{artifacts.resolve()}:/artifacts:rw",
        "-v",
        f"{binary.resolve()}:/opt/whale/whale-app-server:ro",
        "-v",
        f"{(REPO_ROOT / 'scripts/taskspace-benchmark/app-server-active-prefix.py').resolve()}:/opt/benchmark/app-server-active-prefix.py:ro",
        "-v",
        f"{Path(args.env_file).resolve()}:/run/secrets/env.local:ro",
        "-e",
        "HOME=/artifacts/home",
        "-e",
        "WHALE_PROVIDER_WIRE_TRACE_PATH=/artifacts/provider-wire-trace.jsonl",
        "-e",
        "WHALE_TASKSPACE_PROFILE_NAME=taskspace-v005-deep",
        "-e",
        "WHALE_TASKSPACE_ROUTE_MODE=deep",
        IMAGE,
        "bash",
        "-lc",
        "set -euo pipefail; set -a; source /run/secrets/env.local; set +a; "
        ': "${DEEPSEEK_API_KEY:?missing DEEPSEEK_API_KEY}"; '
        "python /opt/benchmark/app-server-active-prefix.py "
        f"--binary /opt/whale/whale-app-server --thread-id {THREAD_ID} --mode {mode} "
        "--prompt /artifacts/continuation-prompt.txt --events /artifacts/app-server-events.jsonl "
        "--stderr /artifacts/app-server.stderr.log --summary /artifacts/client-summary.json "
        "--last-message /artifacts/last-message.md",
    ]
    try:
        result = run(
            command,
            stdout_path=artifacts / "container-agent.stdout.log",
            stderr_path=artifacts / "container-agent.stderr.log",
            check=False,
            timeout=args.timeout_seconds,
        )
    except subprocess.TimeoutExpired:
        run(["docker", "stop", "--time", "3", name], check=False)
        result = subprocess.CompletedProcess(command, 124)
    inspect_container(name, artifacts / "container-inspect-agent.json")
    (artifacts / "container-agent.exit-code.txt").write_text(
        f"{result.returncode}\n", encoding="utf-8"
    )
    return result.returncode


def execute_case(case_dir: Path, arm: str, args: argparse.Namespace) -> dict[str, Any]:
    if validator(case_dir, "initial") != 1:
        raise RuntimeError(f"{arm} initial validator did not expose the expected failure")
    if run_agent(case_dir, arm, args) != 0:
        raise RuntimeError(f"{arm} agent container failed")
    sessions = case_dir / "artifacts/home/.whale/sessions"
    rollout = max(sessions.rglob("rollout*.jsonl"), key=lambda path: path.stat().st_mtime_ns)
    shutil.copy2(rollout, case_dir / "artifacts/final-rollout.jsonl")
    if validator(case_dir, "final") != 0:
        raise RuntimeError(f"{arm} final validator failed")
    output = case_dir / "artifacts/active-prefix-metrics.json"
    run(
        [
            "python3",
            str(REPO_ROOT / "scripts/taskspace-benchmark/analyze-active-prefix.py"),
            "--artifacts",
            str(case_dir / "artifacts"),
            "--arm",
            arm,
            "--output",
            str(output),
        ]
    )
    return read_json(output)


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    args = parse_args()
    if args.repeats < 1 or args.max_parallel < 1:
        raise SystemExit("repeats and max-parallel must be positive")
    try:
        require_ready(REPO_ROOT)
    except WorkspacePreflightError as exc:
        raise SystemExit(str(exc)) from exc
    if not args.plan_only:
        raise SystemExit("run_ledger_authorization_unavailable")
    contract = read_json(SAMPLE_DIR / "sample.json")
    assert_sample_contract(contract)
    candidate = REPO_ROOT / args.candidate_app_server
    previous = REPO_ROOT / args.previous_app_server
    env_file = REPO_ROOT / args.env_file
    for required in (candidate, previous, env_file):
        if not required.exists():
            raise SystemExit(f"required input missing: {required}")
    timestamp = time.strftime("%Y%m%d-%H%M%S")
    run_root = Path(args.run_root) if args.run_root else REPO_ROOT / f"target/r5-map-compression/S1-natural-prefix-matrix-{timestamp}"
    run_root = run_root.resolve()
    run_root.mkdir(parents=True, exist_ok=False)
    plan = {
        "schemaVersion": "taskspace-active-prefix-run-plan-v1",
        "arms": args.arms,
        "repeats": args.repeats,
        "candidateAppServerSha256": sha256(candidate),
        "previousAppServerSha256": sha256(previous),
        "samplePrefixSha256": contract["prefix"]["decompressed_sha256"],
    }
    (run_root / "run-plan.json").write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    if args.plan_only:
        print(run_root)
        return 0

    results: list[dict[str, Any]] = []
    for repeat in range(1, args.repeats + 1):
        cases = {arm: prepare_case(run_root, arm, repeat, contract) for arm in args.arms}
        with concurrent.futures.ThreadPoolExecutor(max_workers=min(args.max_parallel, len(cases))) as pool:
            futures = {pool.submit(execute_case, case, arm, args): arm for arm, case in cases.items()}
            for future in concurrent.futures.as_completed(futures):
                results.append(future.result())
    (run_root / "matrix-results.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(run_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
