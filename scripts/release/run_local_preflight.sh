#!/usr/bin/env bash
set -euo pipefail

version="${1:-0.0.6}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

python3 scripts/release/check_release_identity.py --tag "v${version}"
python3 scripts/release/check_distribution_identity.py
python3 scripts/release/check_brand_identity.py
python3 scripts/release/check_npm_release_candidate.py --tag "v${version}"
python3 scripts/release/check_native_artifact_workflow.py
python3 scripts/release/check_npm_publish_workflow.py
python3 scripts/release/check_manual_actions_only.py
python3 -m unittest discover -s scripts/release/tests -p 'test_*.py'
