#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
temp_root="$(mktemp -d)"
trap 'rm -rf "$temp_root"' EXIT

fake_home="$temp_root/home"
fake_bin_dir="$temp_root/fake-bin"
install_dir="$fake_home/.whale/bin"
capture_path="$temp_root/pwsh-argv.txt"
mkdir -p "$fake_home" "$fake_bin_dir"

fake_whale="$temp_root/whale"
cat >"$fake_whale" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' 'whale 0.1.0'
EOF
chmod +x "$fake_whale"

cat >"$fake_bin_dir/pwsh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$WHALE_TEST_PWSH_CAPTURE"
EOF
chmod +x "$fake_bin_dir/pwsh"

HOME="$fake_home" \
  PATH="$fake_bin_dir:$PATH" \
  WHALE_TEST_PWSH_CAPTURE="$capture_path" \
  "$repo_root/scripts/install-whale-local.sh" \
  --binary-path "$fake_whale" \
  --install-dir "$install_dir" >/dev/null

test -x "$install_dir/whale"
grep -Fx -- "-BuildCommand" "$capture_path" >/dev/null
grep -F -- "$fake_whale" "$capture_path" >/dev/null
grep -F -- "$install_dir/whale" "$capture_path" >/dev/null

echo "install-whale-local tests passed"
