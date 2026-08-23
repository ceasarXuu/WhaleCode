#!/usr/bin/env python3
"""Run tests against the current Codex vendor with host state isolated."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

from qualify_candidate import _qualification_environment
from qualify_candidate import _set_reproducible_child_umask

CODEX_ROOT = "third_party/codex-cli/codex-rs"
RUNTIME_ROOT_ENV = "WHALE_CODEX_TEST_TMPDIR"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _isolated_environment(runtime_root: Path) -> dict[str, str]:
    temporary_root = runtime_root / "tmp"
    temporary_root.mkdir(mode=0o700)
    environment = _qualification_environment()
    environment["TMPDIR"] = str(temporary_root)
    environment["GIT_CEILING_DIRECTORIES"] = str(temporary_root)
    return environment


def _has_workspace_markers(path: Path) -> bool:
    resolved = path.resolve()
    return any(
        (directory / marker).exists()
        for directory in (resolved, *resolved.parents)
        for marker in (".git", ".codex")
    )


def _runtime_base(source: dict[str, str] | None = None) -> Path:
    environment = os.environ if source is None else source
    candidates: list[Path] = []
    if configured := environment.get(RUNTIME_ROOT_ENV):
        candidates.append(Path(configured).expanduser())
    if os.name == "posix":
        candidates.append(Path("/dev/shm"))
    candidates.append(Path(tempfile.gettempdir()))

    for candidate in candidates:
        if (
            candidate.is_dir()
            and os.access(candidate, os.W_OK | os.X_OK)
            and not _has_workspace_markers(candidate)
        ):
            return candidate.resolve()
    raise RuntimeError(
        "no writable temporary root without ancestor .git/.codex markers; "
        f"set {RUNTIME_ROOT_ENV} to an isolated existing directory"
    )


def _command(arguments: list[str]) -> list[str]:
    return ["cargo", "nextest", "run", "--no-fail-fast", *arguments]


def _selected_packages(arguments: list[str]) -> set[str]:
    packages: set[str] = set()
    for index, argument in enumerate(arguments):
        if argument in {"-p", "--package"} and index + 1 < len(arguments):
            packages.add(arguments[index + 1])
        elif argument.startswith("--package="):
            packages.add(argument.partition("=")[2])
    return packages


def _runtime_helper_commands(arguments: list[str]) -> list[list[str]]:
    packages = _selected_packages(arguments)
    if packages and "codex-core" not in packages:
        return []
    return [["cargo", "build", "-p", "codex-rmcp-client", "--bin", "test_stdio_server"]]


def main(arguments: list[str] | None = None) -> int:
    test_arguments = sys.argv[1:] if arguments is None else arguments
    if not test_arguments:
        print(
            "usage: run_isolated_tests.py <cargo-nextest run arguments>",
            file=sys.stderr,
        )
        return 2

    codex_root = _repo_root() / CODEX_ROOT
    try:
        runtime_base = _runtime_base()
        with tempfile.TemporaryDirectory(
            prefix="whale-codex-tests-", dir=runtime_base
        ) as runtime:
            runtime_root = Path(runtime)
            environment = _isolated_environment(runtime_root)
            for helper_command in _runtime_helper_commands(test_arguments):
                helper = subprocess.run(
                    helper_command,
                    cwd=codex_root,
                    env=environment,
                    check=False,
                    preexec_fn=(
                        _set_reproducible_child_umask if os.name == "posix" else None
                    ),
                )
                if helper.returncode != 0:
                    return helper.returncode
            completed = subprocess.run(
                _command(test_arguments),
                cwd=codex_root,
                env=environment,
                check=False,
                preexec_fn=(
                    _set_reproducible_child_umask if os.name == "posix" else None
                ),
            )
    except (OSError, RuntimeError) as error:
        print(f"isolated test setup failed: {error}", file=sys.stderr)
        return 2
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())
