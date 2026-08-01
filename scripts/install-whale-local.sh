#!/usr/bin/env bash
set -euo pipefail

binary_path=""
install_dir="${WHALE_INSTALL_DIR:-$HOME/.whale/bin}"
persist_user_path=0
backup_legacy_copies=0

usage() {
  cat <<'USAGE'
Usage: scripts/install-whale-local.sh [OPTIONS]

Install a locally built Whale binary into an isolated user directory.

Options:
  --binary-path PATH        Use this whale binary instead of auto-detecting one
  --install-dir DIR         Install directory (default: $WHALE_INSTALL_DIR or ~/.whale/bin)
  --persist-user-path       Add the install directory to ~/.profile if missing
  --backup-legacy-copies    Move old shared-path whale binaries into ~/.whale/backups
  -h, --help                Show this help
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary-path)
      binary_path="${2:?--binary-path requires a path}"
      shift 2
      ;;
    --install-dir)
      install_dir="${2:?--install-dir requires a directory}"
      shift 2
      ;;
    --persist-user-path)
      persist_user_path=1
      shift
      ;;
    --backup-legacy-copies)
      backup_legacy_copies=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

realpath_existing() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  else
    readlink -f "$1"
  fi
}

canonical_dir() {
  mkdir -p "$1"
  realpath_existing "$1"
}

assert_isolated_install_dir() {
  local resolved="$1"
  local forbidden=(
    "$HOME/.cargo/bin"
    "$HOME/.local/bin"
    "$HOME/.linuxbrew/bin"
    "/usr/local/bin"
    "/usr/bin"
    "/bin"
  )

  local root
  for root in "${forbidden[@]}"; do
    [ -e "$root" ] || continue
    root="$(realpath_existing "$root")"
    if [ "$resolved" = "$root" ] || [[ "$resolved" == "$root/"* ]]; then
      echo "Refusing to install Whale into shared or official CLI path: $resolved" >&2
      exit 1
    fi
  done
}

resolve_existing_file() {
  local candidate
  if [ -n "$binary_path" ] && [ -f "$binary_path" ]; then
    realpath_existing "$binary_path"
    return 0
  fi

  local candidates=()
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    candidates+=(
      "$CARGO_TARGET_DIR/dev-small/whale"
      "$CARGO_TARGET_DIR/debug/whale"
      "$CARGO_TARGET_DIR/release/whale"
      "$CARGO_TARGET_DIR/dist/whale"
    )
  fi
  candidates+=(
    "$repo_root/third_party/codex-cli/codex-rs/target/dev-small/whale"
    "$repo_root/third_party/codex-cli/codex-rs/target/debug/whale"
    "$repo_root/third_party/codex-cli/codex-rs/target/release/whale"
    "$repo_root/third_party/codex-cli/codex-rs/target/dist/whale"
  )

  for candidate in "${candidates[@]}"; do
    if [ -f "$candidate" ]; then
      realpath_existing "$candidate"
      return 0
    fi
  done

  echo "Cannot find whale. Build first or pass --binary-path." >&2
  exit 1
}

backup_file() {
  local path="$1"
  local backup_root="$2"
  [ -f "$path" ] || return 0

  mkdir -p "$backup_root"
  local name stamp destination index
  name="$(basename "$path")"
  stamp="$(date +%Y%m%d%H%M%S)"
  destination="$backup_root/${name}-${stamp}"
  index=1
  while [ -e "$destination" ]; do
    destination="$backup_root/${name}-${stamp}-${index}"
    index=$((index + 1))
  done
  mv "$path" "$destination"
}

persist_path() {
  local dir="$1"
  local profile="$HOME/.profile"
  local marker="# Whale local CLI"
  local line="export PATH=\"$dir:\$PATH\""

  touch "$profile"
  if ! grep -Fqx "$line" "$profile"; then
    {
      echo ""
      echo "$marker"
      echo "$line"
    } >> "$profile"
  fi
}

install_dir="$(canonical_dir "$install_dir")"
assert_isolated_install_dir "$install_dir"

source_path="$(resolve_existing_file)"
source_dir="$(dirname "$source_path")"
destination="$install_dir/whale"

mkdir -p "$install_dir"
cp "$source_path" "$destination"
chmod +x "$destination"

helper_binaries=(
  whale-app-server
  whale-app-server-test-client
  whale-cloud-tasks
  whale-exec-server
  whale-mcp-server
  whale-responses-api-proxy
  whale-stdio-to-uds
)
for helper in "${helper_binaries[@]}"; do
  if [ -f "$source_dir/$helper" ]; then
    cp "$source_dir/$helper" "$install_dir/$helper"
    chmod +x "$install_dir/$helper"
  fi
done

if [ "$backup_legacy_copies" -eq 1 ]; then
  backup_root="$HOME/.whale/backups/legacy-bin"
  for path in "$HOME/.cargo/bin/whale" "$HOME/.local/bin/whale"; do
    if [ "$(realpath_existing "$path" 2>/dev/null || true)" != "$destination" ]; then
      backup_file "$path" "$backup_root"
    fi
  done
fi

if [ "$persist_user_path" -eq 1 ]; then
  persist_path "$install_dir"
fi

echo "Installed Whale: $destination"
echo "Source: $source_path"
echo "Hash:"
sha256sum "$destination"

attestation_script="$repo_root/scripts/taskspace-benchmark/write-whale-binary-attestation.ps1"
if command -v pwsh >/dev/null 2>&1 && [ -f "$attestation_script" ]; then
  install_provenance="cp '$source_path' '$destination' via scripts/install-whale-local.sh"
  pwsh -NoLogo -NoProfile -File "$attestation_script" \
    -WhaleBin "$destination" \
    -BuildCommand "$install_provenance"
fi
