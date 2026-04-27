# Cross-System Restore Runbook

Date: 2026-04-27

Use this when moving WhaleCode development to a new machine because the current
machine or volume does not have enough free space for the Codex-derived Rust
workspace.

## Restore Contract

The durable state is the Git remote plus separately managed secrets. Do not
restore from `tmp/`, `target/`, archived build caches, or local Codex
configuration directories.

Required durable inputs:

- repository remote: `https://github.com/ceasarXuu/WhaleCode.git`
- branch: `main`
- current baseline commits:
  - `8991de2` imports the upstream Codex CLI snapshot
  - `343e4a3` applies the Whale brand and DeepSeek overlay
- DeepSeek credential, preferably supplied as `DEEPSEEK_API_KEY`
- optional sanitized Whale runtime state from `~/.whale`

Regenerable local state:

- `third_party/codex-cli/codex-rs/target/`
- root `target/`
- Cargo incremental caches
- `/tmp/whalecode-target-backup-*`
- any old `tmp/whalecode-refs/` reference checkout

## Hardware And Disk Budget

The Codex Rust workspace is large. Plan for at least 30 GB of free space before
building locally. More is better if running broad test suites.

For low-space machines, move build output away from the repo before building:

```bash
export WHALE_CACHE_ROOT="/Volumes/BuildCache/whalecode"
mkdir -p "$WHALE_CACHE_ROOT"
export CARGO_TARGET_DIR="$WHALE_CACHE_ROOT/cargo-target"
export CARGO_INCREMENTAL=0
```

If `/Volumes/BuildCache` does not exist, replace it with a local path that has
enough space. Keeping `CARGO_INCREMENTAL=0` reduces incremental cache growth at
the cost of slower rebuilds.

## System Prerequisites

macOS:

```bash
xcode-select --install
brew install git ripgrep rustup-init
/opt/homebrew/opt/rustup/bin/rustup default stable
/opt/homebrew/opt/rustup/bin/rustup component add rustfmt clippy
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
```

Linux:

```bash
sudo apt-get update
sudo apt-get install -y build-essential curl git pkg-config ripgrep
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup component add rustfmt clippy
```

Windows is not the primary restore path yet. If needed, use rustup, Git for
Windows, and PowerShell equivalents for the environment variables below, then
start with `cargo check -p codex-cli --locked`.

## Network Recovery

If GitHub or Cargo traffic is unstable and the new machine has the same local
proxy available, use:

```bash
export HTTPS_PROXY="http://127.0.0.1:7890"
export HTTP_PROXY="http://127.0.0.1:7890"
export ALL_PROXY="socks5h://127.0.0.1:7890"
```

Unset these variables when the proxy is unavailable:

```bash
unset HTTPS_PROXY HTTP_PROXY ALL_PROXY
```

## Clone And Verify Source

```bash
git clone https://github.com/ceasarXuu/WhaleCode.git
cd WhaleCode
git checkout main
git pull --ff-only
git status --short --branch
git log --oneline -3
sed -n '1,80p' third_party/codex-cli/UPSTREAM.md
```

Expected:

- `git status --short --branch` is clean and tracks `origin/main`.
- `third_party/codex-cli/UPSTREAM.md` records upstream Codex commit
  `fed0a8f4faa58db3138488cca77628c1d54a2cd8`.
- no nested Git metadata exists under the vendored tree:

```bash
find third_party/codex-cli -name .git -print
```

The command above should print nothing.

## Build

```bash
cd third_party/codex-cli/codex-rs
cargo check -p codex-cli --locked
cargo build -p codex-cli --bin whale --locked
```

Default binary path:

```bash
./target/debug/whale --version
```

If `CARGO_TARGET_DIR` is set, use:

```bash
"$CARGO_TARGET_DIR/debug/whale" --version
```

Expected version output currently starts with:

```text
whale 0.0.0
```

## Isolated Smoke Test

Use an isolated runtime home before any real login or model call. This proves
the build works without touching local Codex or Whale state.

```bash
SMOKE_DIR="$(mktemp -d /tmp/whalecode-smoke.XXXXXX)"
mkdir -p "$SMOKE_DIR/home" "$SMOKE_DIR/whale-home"

HOME="$SMOKE_DIR/home" \
WHALE_HOME="$SMOKE_DIR/whale-home" \
env -u CODEX_HOME -u OPENAI_API_KEY -u DEEPSEEK_API_KEY \
  ./target/debug/whale --version

HOME="$SMOKE_DIR/home" \
WHALE_HOME="$SMOKE_DIR/whale-home" \
env -u CODEX_HOME -u OPENAI_API_KEY -u DEEPSEEK_API_KEY \
  ./target/debug/whale --help
```

If using `CARGO_TARGET_DIR`, replace `./target/debug/whale` with
`"$CARGO_TARGET_DIR/debug/whale"`.

The normal help output should not expose OpenAI-specific Whale-disabled entries
such as app-server, cloud, remote auth, or image flags.

## DeepSeek Runtime Setup

Prefer environment-based credentials on the first restore:

```bash
export DEEPSEEK_API_KEY="replace-with-real-key"
export WHALE_HOME="$HOME/.whale"
mkdir -p "$WHALE_HOME"
```

Then run a small model smoke only when network access and billing are expected:

```bash
./target/debug/whale exec "Reply with one short sentence."
```

Default provider/model expectations:

- provider: DeepSeek
- normal model: `deepseek-v4-flash`
- high-quality model: `deepseek-v4-pro`
- text-only capability until DeepSeek image/search support is explicitly
  verified and enabled

Do not copy `~/.codex` into `~/.whale`. If existing Whale runtime state must be
carried over, copy only after reviewing it for credentials and private project
paths:

```bash
tar -czf whalecode-runtime-state.tgz -C "$HOME" .whale
```

Restore on the new machine:

```bash
tar -xzf whalecode-runtime-state.tgz -C "$HOME"
```

Prefer not to include this archive in Git or issue attachments.

## Regression Commands

Run these after the first successful build:

```bash
cargo test -p codex-api --locked chat_completions
cargo test -p codex-model-provider-info --locked
cargo test -p codex-core --locked defaults_to_deepseek_flash_provider
cargo test -p codex-core --locked responses_websocket_features_do_not_change_wire_api
cargo test -p codex-core --locked config_schema_matches_fixture
```

Full workspace tests are expensive and can consume significant disk. Run them
only on a machine with enough free space.

## Failure Recovery

Out of disk space:

```bash
du -sh third_party/codex-cli/codex-rs/target target 2>/dev/null || true
export CARGO_TARGET_DIR="/path/with/space/whalecode-target"
export CARGO_INCREMENTAL=0
```

Do not use irreversible deletion for project artifacts. Move regenerable build
outputs to a recoverable backup path first:

```bash
mkdir -p /tmp/whalecode-build-backups
mv third_party/codex-cli/codex-rs/target /tmp/whalecode-build-backups/codex-rs-target-$(date +%Y%m%d%H%M%S)
```

Rust toolchain not found:

```bash
. "$HOME/.cargo/env" 2>/dev/null || true
rustup show active-toolchain
cargo --version
```

Network failure:

```bash
git ls-remote https://github.com/ceasarXuu/WhaleCode.git HEAD
cargo fetch --locked
```

Retry with the proxy variables from the Network Recovery section if either
command fails due to connection timeouts.

## Old Machine Handoff Checklist

Before leaving the old machine:

```bash
git status --short --branch
git rev-parse HEAD
git rev-list --left-right --count @{u}...HEAD
```

Expected:

- working tree clean
- `HEAD` is pushed to `origin/main`
- upstream ahead/behind count is `0 0`

Record local-only state intentionally:

```bash
rustc --version
cargo --version
du -sh third_party/codex-cli/codex-rs/target target 2>/dev/null || true
find "$HOME/.whale" -maxdepth 2 -type f 2>/dev/null | sort
```

Do not rely on local `/tmp` backups, `target/`, or old `tmp/whalecode-refs/`
directories as source of truth.
