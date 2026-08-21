#!/usr/bin/env python3
"""Fetch checksum-verified upstream V8 build inputs for a Whale target."""

import argparse
import hashlib
import os
import re
import urllib.request
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--github-env", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    lock = (root / "third_party/codex-cli/codex-rs/Cargo.lock").read_text()
    match = re.search(r'\[\[package\]\]\nname = "v8"\nversion = "([^"]+)"', lock)
    if not match:
        raise SystemExit("v8 crate version is missing from Cargo.lock")

    profile = "ptrcomp_sandbox_release"
    target = args.target
    archive = (
        f"rusty_v8_{profile}_{target}.lib.gz"
        if target.endswith("pc-windows-msvc")
        else f"librusty_v8_{profile}_{target}.a.gz"
    )
    binding = f"src_binding_{profile}_{target}.rs"
    checksum = f"rusty_v8_{profile}_{target}.sha256"
    base = f"https://github.com/openai/codex/releases/download/rusty-v8-v{match.group(1)}"
    destination = Path(os.environ["RUNNER_TEMP"]) / "whale-rusty-v8"
    destination.mkdir(parents=True, exist_ok=True)
    for name in (archive, binding, checksum):
        urllib.request.urlretrieve(f"{base}/{name}", destination / name)

    expected = {}
    for line in (destination / checksum).read_text().replace("\r", "").splitlines():
        digest, name = line.split(maxsplit=1)
        expected[name.lstrip("*")] = digest
    if set(expected) != {archive, binding}:
        raise SystemExit(f"unexpected V8 checksum inventory: {sorted(expected)}")
    for name, digest in expected.items():
        actual = hashlib.sha256((destination / name).read_bytes()).hexdigest()
        if actual != digest:
            raise SystemExit(f"V8 checksum mismatch: {name}")

    with args.github_env.open("a", encoding="utf-8") as env:
        env.write(f"RUSTY_V8_ARCHIVE={destination / archive}\n")
        env.write(f"RUSTY_V8_SRC_BINDING_PATH={destination / binding}\n")


if __name__ == "__main__":
    main()
