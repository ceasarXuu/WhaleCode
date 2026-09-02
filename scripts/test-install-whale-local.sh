#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
temp_root="$(mktemp -d)"
trap 'rm -rf "$temp_root"' EXIT

fake_home="$temp_root/home"
fake_whale="$temp_root/whale"
fake_code_mode_host="$temp_root/codex-code-mode-host"
mkdir -p "$fake_home/.whale/bin" "$fake_home/.local/bin"
printf '%s\n' 'legacy-sentinel' >"$fake_home/.whale/bin/sentinel"
printf '%s\n' 'release-sentinel' >"$fake_home/.local/bin/whale"
chmod +x "$fake_home/.local/bin/whale"
cat >"$fake_whale" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' 'whale 0.1.0'
EOF
chmod +x "$fake_whale"
cp "$fake_whale" "$fake_code_mode_host"

prepare_repo() {
  local destination="$1"
  git clone -q --no-hardlinks "$repo_root" "$destination"
  cp "$repo_root/scripts/install-whale-local.sh" "$destination/scripts/install-whale-local.sh"
  cp "$repo_root/scripts/workspace-safety/write_binary_attestation.py" \
    "$destination/scripts/workspace-safety/write_binary_attestation.py"
  cp "$repo_root/scripts/workspace-safety/whale_dev_dispatcher.py" \
    "$destination/scripts/workspace-safety/whale_dev_dispatcher.py"
  git -C "$destination" config user.name "Installer Test"
  git -C "$destination" config user.email "installer@example.invalid"
  git -C "$destination" add scripts/install-whale-local.sh \
    scripts/workspace-safety/write_binary_attestation.py \
    scripts/workspace-safety/whale_dev_dispatcher.py
  git -C "$destination" commit -q -m "fixture installer"
}

bootstrap_repo() {
  local repository="$1"
  local plan fingerprint
  plan="$(HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
    python3 "$repository/scripts/workspace-safety/workspace_context.py" \
    bootstrap plan --repo-root "$repository" --json)"
  fingerprint="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["fingerprint"])' <<<"$plan")"
  HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
    python3 "$repository/scripts/workspace-safety/workspace_context.py" \
    bootstrap apply --repo-root "$repository" --expect "$fingerprint" >/dev/null
}

workspace_install() {
  local repository="$1"
  HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
    "$repository/scripts/install-whale-local.sh" --scope workspace \
    --binary-path "$fake_whale" >/dev/null
}

repo_a="$temp_root/repo-a"
repo_b="$temp_root/repo-b"
prepare_repo "$repo_a"
prepare_repo "$repo_b"
bootstrap_repo "$repo_a"
bootstrap_repo "$repo_b"
workspace_install "$repo_a"
workspace_install "$repo_b"

plan_a="$(HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  python3 "$repo_a/scripts/workspace-safety/workspace_context.py" bootstrap plan --repo-root "$repo_a" --json)"
plan_b="$(HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  python3 "$repo_b/scripts/workspace-safety/workspace_context.py" bootstrap plan --repo-root "$repo_b" --json)"
bin_a="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["context"]["resources"]["binary_dir"])' <<<"$plan_a")"
bin_b="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["context"]["resources"]["binary_dir"])' <<<"$plan_b")"
test "$bin_a" != "$bin_b"
test -x "$bin_a/whale"
test -x "$bin_b/whale"
test -x "$bin_a/codex-code-mode-host"
test -x "$bin_b/codex-code-mode-host"
test -f "$bin_a/whale.build-attestation.json"
test -f "$bin_b/whale.build-attestation.json"
test "$(cat "$fake_home/.whale/bin/sentinel")" = "legacy-sentinel"
test "$(cat "$fake_home/.local/bin/whale")" = "release-sentinel"
test -x "$fake_home/.local/bin/whale-dev"
version_a="$(cd "$repo_a" && HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" \
  XDG_DATA_HOME="$fake_home/data" "$fake_home/.local/bin/whale-dev" --version)"
version_b="$(cd "$repo_b" && HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" \
  XDG_DATA_HOME="$fake_home/data" "$fake_home/.local/bin/whale-dev" --version)"
test "$version_a" != "$version_b"
[[ "$version_a" == "whale-dev whale 0.1.0 ["*"]" ]]
[[ "$version_b" == "whale-dev whale 0.1.0 ["*"]" ]]

user_install="$fake_home/custom-user-bin"
HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  "$repo_a/scripts/install-whale-local.sh" --scope user --binary-path "$fake_whale" \
  --install-dir "$user_install" >/dev/null
test -x "$user_install/whale"
test -x "$user_install/codex-code-mode-host"
test -f "$user_install/whale.build-attestation.json"

repo_c="$temp_root/repo-c"
prepare_repo "$repo_c"
unbootstrapped_plan="$(HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  python3 "$repo_c/scripts/workspace-safety/workspace_context.py" bootstrap plan --repo-root "$repo_c" --json)"
unbootstrapped_bin="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["context"]["resources"]["binary_dir"])' <<<"$unbootstrapped_plan")"
if HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  "$repo_c/scripts/install-whale-local.sh" --scope workspace \
  --binary-path "$fake_whale" >/dev/null 2>&1; then
  echo "installer accepted an unbootstrapped workspace" >&2
  exit 1
fi
test ! -e "$unbootstrapped_bin"

workspace_override="$temp_root/workspace-override"
if HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  "$repo_a/scripts/install-whale-local.sh" --scope workspace \
  --binary-path "$fake_whale" --install-dir "$workspace_override" >/dev/null 2>&1; then
  echo "workspace scope accepted an explicit install directory" >&2
  exit 1
fi
test ! -e "$workspace_override"

blocked="$temp_root/blocked"
if HOME="$fake_home" XDG_STATE_HOME="$fake_home/state" XDG_DATA_HOME="$fake_home/data" \
  "$repo_a/scripts/install-whale-local.sh" --binary-path "$fake_whale" \
  --install-dir "$blocked" >/dev/null 2>&1; then
  echo "installer accepted a missing scope" >&2
  exit 1
fi
test ! -e "$blocked"

printf '%s\n' '#!/usr/bin/env python3' 'WHALE_DEV_DISPATCHER_SCHEMA = 999' \
  'print("newer-managed")' >"$fake_home/.local/bin/whale-dev"
chmod +x "$fake_home/.local/bin/whale-dev"
workspace_install "$repo_a"
test "$("$fake_home/.local/bin/whale-dev")" = "newer-managed"

printf '%s\n' '#!/bin/sh' 'echo unmanaged' >"$fake_home/.local/bin/whale-dev"
chmod +x "$fake_home/.local/bin/whale-dev"
if workspace_install "$repo_a" >/dev/null 2>&1; then
  echo "installer replaced an unmanaged whale-dev command" >&2
  exit 1
fi
test "$("$fake_home/.local/bin/whale-dev")" = "unmanaged"

echo "install-whale-local tests passed"
