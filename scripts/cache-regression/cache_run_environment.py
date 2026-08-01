#!/usr/bin/env python3
"""Credential discovery and run-directory resolution for cache smoke runs."""

import os
from pathlib import Path


def ensure_deepseek_api_key(repo: Path) -> str:
    if os.environ.get("DEEPSEEK_API_KEY", "").strip():
        return "process_environment"
    env_path = repo / ".env.local"
    if not env_path.is_file():
        raise RuntimeError("DEEPSEEK_API_KEY is missing and .env.local does not exist")
    for raw_line in env_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line.removeprefix("export ").lstrip()
        key, separator, value = line.partition("=")
        if separator and key.strip() == "DEEPSEEK_API_KEY":
            value = value.strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
                value = value[1:-1]
            if not value:
                break
            os.environ["DEEPSEEK_API_KEY"] = value
            return ".env.local"
    raise RuntimeError("DEEPSEEK_API_KEY is missing from .env.local")


def find_run_dir_by_id(run_root: Path, run_id: str) -> Path:
    candidates = [path for path in run_root.glob(f"*/{run_id}") if path.is_dir()]
    if len(candidates) != 1:
        raise RuntimeError(
            f"benchmark run id {run_id} resolved to {len(candidates)} directories"
        )
    return candidates[0]
