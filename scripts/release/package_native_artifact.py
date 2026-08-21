#!/usr/bin/env python3
"""Create a deterministic Whale npm-native payload and its evidence contract."""

import argparse
import gzip
import hashlib
import json
import os
import subprocess
import tarfile
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    expected = f"whale {args.version}"
    actual = subprocess.check_output([args.binary, "--version"], text=True).strip()
    if actual != expected:
        raise SystemExit(f"binary identity mismatch: {actual!r} != {expected!r}")

    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"whale-package-{args.target}.tar.gz"
    member_name = "bin/whale.exe" if "windows" in args.target else "bin/whale"
    info = tarfile.TarInfo(member_name)
    info.size = args.binary.stat().st_size
    info.mode = 0o755
    info.mtime = 0
    with archive.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode="w") as tar, args.binary.open("rb") as binary:
            tar.addfile(info, binary)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (args.output / f"{archive.name}.sha256").write_text(f"{digest}  {archive.name}\n")
    contract = {
        "schema_version": 1,
        "product": "WhaleCode",
        "version": args.version,
        "target": args.target,
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "unsigned": True,
        "archive": archive.name,
        "sha256": digest,
    }
    (args.output / "artifact-contract.json").write_text(
        json.dumps(contract, indent=2, sort_keys=True) + "\n"
    )


if __name__ == "__main__":
    main()
