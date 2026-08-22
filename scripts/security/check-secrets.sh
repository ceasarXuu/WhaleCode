#!/usr/bin/env bash
set -euo pipefail

readonly REQUIRED_GITLEAKS_VERSION="8.30.1"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
readonly GITLEAKS_COMMAND="${GITLEAKS_BIN:-gitleaks}"
readonly ARCHIVE_MANIFEST="scripts/security/tracked-archives.sha256"

if ! command -v "$GITLEAKS_COMMAND" >/dev/null 2>&1; then
  echo "gitleaks ${REQUIRED_GITLEAKS_VERSION} is required; set GITLEAKS_BIN to its executable path." >&2
  exit 2
fi

actual_version="$($GITLEAKS_COMMAND version | sed -E 's/^v//')"
if [[ "$actual_version" != "$REQUIRED_GITLEAKS_VERSION" ]]; then
  echo "expected gitleaks ${REQUIRED_GITLEAKS_VERSION}, found ${actual_version}" >&2
  exit 2
fi

cd "$REPO_ROOT"

tracked_archives="$({
  git -c core.quotePath=false ls-files \
    | grep -Ei '\.(zip|tar|tar\.gz|tgz|7z|rar|gz)$' \
    || true
} | sort)"
manifest_archives="$(sed -E 's/^[0-9a-f]{64}  //' "$ARCHIVE_MANIFEST" | sort)"
if [[ "$tracked_archives" != "$manifest_archives" ]]; then
  echo "tracked archive inventory changed; review its contents and update ${ARCHIVE_MANIFEST}" >&2
  diff <(printf '%s\n' "$manifest_archives") <(printf '%s\n' "$tracked_archives") || true
  exit 1
fi
sha256sum --check --strict "$ARCHIVE_MANIFEST"

exec "$GITLEAKS_COMMAND" git . \
  --log-opts=--all \
  --no-banner \
  --redact=100
