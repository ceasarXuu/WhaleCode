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
RUSTY_V8_VERSION = "150.4.0"
RUSTY_V8_TARGET = "x86_64-unknown-linux-gnu"
RUSTY_V8_PROFILE = "ptrcomp_sandbox_release"
OUTPUT_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-candidate.json"
EVIDENCE_DIR = (
    "docs/v0.0.5/codex-upstream-sync/evidence/"
    "rust-v0.147.0/attempt-9-runtime-ceiling"
)
QUALIFICATION_ENVIRONMENT = {
    "INSTA_UPDATE": "no",
    "NEXTEST_PROFILE": "local",
    "RUST_MIN_STACK": "8388608",
}
PROXY_ENVIRONMENT_KEYS = frozenset(
    {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}
)
AMBIENT_CODEX_ENVIRONMENT_KEYS = frozenset({"CODEX_SANDBOX_NETWORK_DISABLED"})
COMMANDS = (
    ("01-fmt", ("cargo", "fmt", "--all", "--", "--check")),
    (
        "02-cli-check",
        ("cargo", "check", "-p", "codex-cli", "--bin", "codex", "--offline"),
    ),
    (
        "03-code-mode-host-build",
        ("cargo", "build", "--offline", "-p", "codex-code-mode-host", "--bin", "codex-code-mode-host"),
    ),
    (
        "04-core-tests",
        ("cargo", "nextest", "run", "--no-fail-fast", "-p", "codex-core"),
    ),
    (
        "05-app-server-tests",
        ("cargo", "nextest", "run", "--no-fail-fast", "-p", "codex-app-server"),
    ),
    (
        "06-tui-tests",
        ("cargo", "nextest", "run", "--no-fail-fast", "-p", "codex-tui"),
    ),
)
PACKAGE_TEST_IDS = frozenset({"04-core-tests", "05-app-server-tests", "06-tui-tests"})
ISOLATED_HOME_TEST_IDS = frozenset({"05-app-server-tests"})
PREPARATION_COMMAND = ("cargo", "fetch")
TEST_SUPPORT_COMMAND = (
    "cargo", "build", "--offline", "-p", "codex-cli", "--bin", "codex",
    "-p", "codex-rmcp-client", "--bin", "test_stdio_server",
)


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _qualification_environment(
    source: dict[str, str] | None = None,
    isolated_home: Path | None = None,
) -> dict[str, str]:
    environment = dict(os.environ if source is None else source)
    for key in tuple(environment):
        if (
            key.lower() in PROXY_ENVIRONMENT_KEYS
            or key in AMBIENT_CODEX_ENVIRONMENT_KEYS
        ):
            environment.pop(key)
    environment.update(QUALIFICATION_ENVIRONMENT)
    if isolated_home is not None:
        original_home = Path(environment.get("HOME", str(Path.home())))
        environment["HOME"] = str(isolated_home)
        environment.setdefault("CARGO_HOME", str(original_home / ".cargo"))
        environment.setdefault("RUSTUP_HOME", str(original_home / ".rustup"))
    return environment


def _set_reproducible_child_umask() -> None:
    os.umask(0o022)


def _normalize_output(
    output: str,
    repo: Path,
    candidate_root: Path,
    runtime_root: Path | None = None,
) -> str:
    normalized = output.replace(str(candidate_root), "<candidate>")
    if runtime_root is not None:
        normalized = normalized.replace(str(runtime_root), "<runtime>")
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
    repo: Path,
    candidate_root: Path,
    runtime_root: Path,
    evidence_dir: Path,
    rusty_v8_environment: dict[str, str],
) -> list[dict]:
    results: list[dict] = []
    codex_rs = candidate_root / "codex-rs"
    external_tmp = runtime_root / "tmp"
    private_home = runtime_root / "home"
    external_tmp.mkdir(parents=True, exist_ok=True, mode=0o700)
    private_home.mkdir(parents=True, exist_ok=True, mode=0o700)
    external_tmp.chmod(0o700)
    private_home.chmod(0o700)
    for command_id, command in COMMANDS:
        logging.info("running candidate qualification %s", command_id)
        proxy_environment = (
            "scrubbed" if command_id in PACKAGE_TEST_IDS else "inherited"
        )
        environment = (
            _qualification_environment(
                isolated_home=(
                    private_home if command_id in ISOLATED_HOME_TEST_IDS else None
                )
            )
            if command_id in PACKAGE_TEST_IDS
            else {**os.environ, **QUALIFICATION_ENVIRONMENT}
        )
        environment.update(rusty_v8_environment)
        environment["TMPDIR"] = str(external_tmp)
        if command_id in PACKAGE_TEST_IDS:
            environment["GIT_CEILING_DIRECTORIES"] = str(external_tmp)
        completed = subprocess.run(
            command,
            cwd=codex_rs,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            text=True,
            preexec_fn=(
                _set_reproducible_child_umask
                if command_id in PACKAGE_TEST_IDS
                else None
            ),
        )
        normalized = _normalize_output(
            completed.stdout,
            repo,
            candidate_root,
            runtime_root,
        )
        evidence_path = evidence_dir / f"{command_id}.log"
        evidence_path.parent.mkdir(parents=True, exist_ok=True)
        evidence_path.write_text(
            f"command: {' '.join(command)}\n"
            f"cwd: codex-rs\n"
            f"environment: {json.dumps(QUALIFICATION_ENVIRONMENT, sort_keys=True)}\n"
            f"proxy_environment: {proxy_environment}\n"
            f"home_environment: {'isolated' if command_id in ISOLATED_HOME_TEST_IDS else 'inherited'}\n"
            "tmp_environment: external\n"
            f"git_ceiling_directories: {'runtime-temp' if command_id in PACKAGE_TEST_IDS else 'inherited'}\n"
            f"child_umask: {'0022' if command_id in PACKAGE_TEST_IDS else 'inherited'}\n"
            f"exit_code: {completed.returncode}\n\n{normalized}",
            encoding="utf-8",
        )
        results.append(
            {
                "command": list(command),
                "cwd": "codex-rs",
                "environment": QUALIFICATION_ENVIRONMENT,
                "proxy_environment": proxy_environment,
                "home_environment": (
                    "isolated" if command_id in ISOLATED_HOME_TEST_IDS else "inherited"
                ),
                "tmp_environment": "external",
                "git_ceiling_directories": (
                    "runtime-temp" if command_id in PACKAGE_TEST_IDS else "inherited"
                ),
                "child_umask": (
                    "0022" if command_id in PACKAGE_TEST_IDS else "inherited"
                ),
                "rusty_v8_artifacts": "codex-release-verified",
                "evidence": evidence_path.relative_to(repo).as_posix(),
                "exit_code": completed.returncode,
                "id": command_id,
                "result": "passed" if completed.returncode == 0 else "failed",
            }
        )
    return results


def _prepare_rusty_v8(repo: Path, evidence_dir: Path) -> dict[str, str]:
    release_tag = f"rusty-v8-v{RUSTY_V8_VERSION}"
    base_url = f"https://github.com/openai/codex/releases/download/{release_tag}"
    archive_name = (
        f"librusty_v8_{RUSTY_V8_PROFILE}_{RUSTY_V8_TARGET}.a.gz"
    )
    binding_name = f"src_binding_{RUSTY_V8_PROFILE}_{RUSTY_V8_TARGET}.rs"
    checksums_name = f"rusty_v8_{RUSTY_V8_PROFILE}_{RUSTY_V8_TARGET}.sha256"
    cache_dir = (
        Path(tempfile.gettempdir())
        / "whale-codex-candidate-cache"
        / release_tag
        / RUSTY_V8_TARGET
    )
    cache_dir.mkdir(parents=True, exist_ok=True)
    checksums_path = cache_dir / checksums_name
    _download(f"{base_url}/{checksums_name}", checksums_path)
    expected: dict[str, str] = {}
    for line in checksums_path.read_text(encoding="utf-8").splitlines():
        digest, name = line.split(maxsplit=1)
        expected[name.strip()] = digest
    artifact_names = {archive_name, binding_name}
    if expected.keys() != artifact_names:
        raise RuntimeError("rusty_v8 checksum manifest has unexpected contents")
    artifacts = {name: cache_dir / name for name in artifact_names}
    for name, path in artifacts.items():
        if not path.is_file() or _sha256_file(path) != expected[name]:
            _download(f"{base_url}/{name}", path)
        if _sha256_file(path) != expected[name]:
            raise RuntimeError(f"rusty_v8 checksum mismatch for {name}")
    evidence_path = evidence_dir / "00-rusty-v8-artifacts.log"
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    evidence_path.write_text(
        f"release: {release_tag}\n"
        f"base_url: {base_url}\n"
        f"target: {RUSTY_V8_TARGET}\n"
        f"archive: {archive_name}\n"
        f"archive_sha256: {expected[archive_name]}\n"
        f"binding: {binding_name}\n"
        f"binding_sha256: {expected[binding_name]}\n"
        "verification: passed\n",
        encoding="utf-8",
    )
    return {
        "RUSTY_V8_ARCHIVE": str(artifacts[archive_name]),
        "RUSTY_V8_SRC_BINDING_PATH": str(artifacts[binding_name]),
    }


def _download(url: str, destination: Path) -> None:
    completed = subprocess.run(
        ("curl", "-fsSL", url, "-o", str(destination)),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        text=True,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"download failed for {url}: {completed.stdout}")


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


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


def _prepare_test_support(
    repo: Path,
    candidate_root: Path,
    evidence_dir: Path,
    rusty_v8_environment: dict[str, str],
) -> None:
    logging.info("building candidate test support binaries")
    environment = {**os.environ, **QUALIFICATION_ENVIRONMENT, **rusty_v8_environment}
    for key in AMBIENT_CODEX_ENVIRONMENT_KEYS:
        environment.pop(key, None)
    completed = subprocess.run(
        TEST_SUPPORT_COMMAND,
        cwd=candidate_root / "codex-rs",
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        text=True,
    )
    evidence_path = evidence_dir / "00-test-support-build.log"
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    normalized = _normalize_output(completed.stdout, repo, candidate_root)
    evidence_path.write_text(
        f"command: {' '.join(TEST_SUPPORT_COMMAND)}\n"
        "cwd: codex-rs\n"
        "proxy_environment: inherited\n"
        "rusty_v8_artifacts: codex-release-verified\n"
        f"exit_code: {completed.returncode}\n\n{normalized}",
        encoding="utf-8",
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"candidate test support build failed; see {evidence_path.relative_to(repo)}"
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
    with (
        tempfile.TemporaryDirectory(prefix="whale-codex-0.147-") as temp_dir,
        tempfile.TemporaryDirectory(prefix="whale-codex-0.147-runtime-") as runtime_dir,
    ):
        candidate_root = Path(temp_dir)
        runtime_root = Path(runtime_dir)
        _export_candidate(repo, candidate_root)
        _prepare_dependencies(repo, candidate_root, repo / EVIDENCE_DIR)
        rusty_v8_environment = _prepare_rusty_v8(repo, repo / EVIDENCE_DIR)
        _prepare_test_support(
            repo,
            candidate_root,
            repo / EVIDENCE_DIR,
            rusty_v8_environment,
        )
        commands = _run_qualification(
            repo,
            candidate_root,
            runtime_root,
            repo / EVIDENCE_DIR,
            rusty_v8_environment,
        )
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
    if not (repo / EVIDENCE_DIR / "00-rusty-v8-artifacts.log").is_file():
        errors.append("missing verified Codex rusty_v8 artifact evidence")
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
