#!/usr/bin/env python3
"""Build and inspect the Whale npm meta-package without publishing it."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import check_release_identity


PACKAGE_NAME = "@ceasarxuu/whalecode"
PLATFORMS = (
    "linux-x64",
    "linux-arm64",
    "darwin-x64",
    "darwin-arm64",
    "win32-x64",
    "win32-arm64",
)
EXPECTED_FILES = {"README.md", "bin/whale.js", "package.json"}


class NpmCandidateError(ValueError):
    pass


def expected_optional_dependencies(version: str) -> dict[str, str]:
    return {
        f"whalecode-{platform}": f"npm:{PACKAGE_NAME}@{version}-{platform}"
        for platform in PLATFORMS
    }


def validate_staged_manifest(manifest: dict, version: str) -> None:
    expected = {
        "name": PACKAGE_NAME,
        "version": version,
        "bin": {"whale": "bin/whale.js"},
        "files": ["bin/whale.js"],
        "optionalDependencies": expected_optional_dependencies(version),
    }
    for field, value in expected.items():
        if manifest.get(field) != value:
            raise NpmCandidateError(
                f"staged npm manifest {field} must be {value!r}, got {manifest.get(field)!r}"
            )


def validate_pack_output(pack_output: object, version: str) -> dict:
    if not isinstance(pack_output, list) or len(pack_output) != 1:
        raise NpmCandidateError("npm pack must describe exactly one tarball")
    package = pack_output[0]
    if not isinstance(package, dict):
        raise NpmCandidateError("npm pack output must contain an object")
    if package.get("name") != PACKAGE_NAME or package.get("version") != version:
        raise NpmCandidateError("npm pack identity does not match the Whale release")
    files = package.get("files")
    if not isinstance(files, list):
        raise NpmCandidateError("npm pack output is missing its file inventory")
    paths = {entry.get("path") for entry in files if isinstance(entry, dict)}
    if paths != EXPECTED_FILES:
        raise NpmCandidateError(
            f"npm tarball files must be {sorted(EXPECTED_FILES)!r}, got {sorted(paths)!r}"
        )
    if not package.get("shasum") or not package.get("integrity"):
        raise NpmCandidateError("npm pack output is missing integrity metadata")
    return package


def validate(repo_root: Path, supplied_tag: str | None = None) -> dict:
    whale_tag, _ = check_release_identity.validate(repo_root, supplied_tag)
    version = whale_tag.removeprefix("v")
    builder = (
        repo_root
        / "third_party/codex-cli/codex-cli/scripts/build_npm_package.py"
    )

    with tempfile.TemporaryDirectory(prefix="whale-npm-candidate-") as tempdir:
        temp_root = Path(tempdir)
        stage = temp_root / "stage"
        subprocess.run(
            [
                sys.executable,
                str(builder),
                "--package",
                "whalecode",
                "--release-version",
                version,
                "--staging-dir",
                str(stage),
            ],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
        manifest = json.loads((stage / "package.json").read_text(encoding="utf-8"))
        validate_staged_manifest(manifest, version)

        env = os.environ.copy()
        env["NPM_CONFIG_CACHE"] = str(temp_root / "npm-cache")
        env["NPM_CONFIG_LOGS_DIR"] = str(temp_root / "npm-logs")
        result = subprocess.run(
            ["npm", "pack", "--dry-run", "--json"],
            cwd=stage,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        return validate_pack_output(json.loads(result.stdout), version)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    parser.add_argument("--tag", help="candidate Whale product tag")
    args = parser.parse_args()
    try:
        package = validate(args.repo_root.resolve(), args.tag)
    except (NpmCandidateError, check_release_identity.IdentityError) as exc:
        print(f"npm release candidate check FAILED: {exc}", file=sys.stderr)
        return 1
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        print(f"npm release candidate check FAILED: {exc}", file=sys.stderr)
        return 1
    print(
        "npm release candidate check OK: "
        f"{package['name']}@{package['version']}; "
        f"files={len(package['files'])}; integrity={package['integrity']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
