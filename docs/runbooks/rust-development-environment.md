# Rust Development Environment Runbook

Date: 2026-04-25

## Context

2026-04-27 update: the from-scratch Rust demo workspace has been archived under
`archive/deprecated/2026-04-27-rust-demo/`. The active direction is Codex CLI
upstream substrate plus Whale bridge/overlay. This runbook remains useful for
building Rust code after the Codex substrate import, but the repo root no longer
has an active `rust-toolchain.toml` or `Cargo.toml`.

The archived demo pinned the toolchain through `rust-toolchain.toml`:

- channel: `stable`
- components: `rustfmt`, `clippy`

## macOS Setup

Install rustup through Homebrew:

```bash
brew install rustup-init
/opt/homebrew/opt/rustup/bin/rustup default stable
/opt/homebrew/opt/rustup/bin/rustup component add rustfmt clippy
```

Homebrew installs `rustup` as keg-only because it can conflict with the `rust` formula. Add the rustup bin path to zsh:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"
```

If Homebrew is unavailable but network access is enabled, the standard rustup
installer is the fallback:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup component add rustfmt clippy
```

In this environment the paths are appended to `~/.zshrc`, not `~/.zprofile`, so new interactive zsh sessions can find `cargo`, `rustc`, `rustup`, and locally installed CLI binaries such as `whale`.

Non-interactive Codex shell calls may not read `~/.zshrc`. Prefix verification commands when needed:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH" cargo test --workspace
```

## Verification

Before the Codex substrate import, verify only the Rust toolchain itself:

```bash
cargo --version
rustc --version
rustup show active-toolchain
```

To inspect the archived demo, enter the archive explicitly:

```bash
cd archive/deprecated/2026-04-27-rust-demo
cargo test --workspace --locked
```

After Codex import, verify the active Codex-derived workspace directly:

```bash
cd third_party/codex-cli/codex-rs
cargo check -p codex-cli --locked
cargo test -p codex-linux-sandbox --lib --locked
cargo build -p codex-cli --bin whale --locked
cargo run --quiet -p codex-cli --bin whale -- --version
```

## Linux Vendored Bubblewrap Dependency

健康的 Linux 开发构建默认不设置 `CODEX_SKIP_VENDORED_BWRAP`。这样
`codex-linux-sandbox` 会编译 vendored bubblewrap，并覆盖完整 sandbox
构建链路。该路径需要 `pkg-config` 能找到 `libcap.pc`。

先检查依赖：

```text
pkg-config --libs --cflags libcap
```

Ubuntu/Debian 系统级安装：

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libcap-dev
```

无 sudo 但已有 Linuxbrew 时，可以走用户级依赖：

```bash
brew install libcap
pkg-config --libs --cflags libcap
```

2026-07-08 本机 Ubuntu 24.04 使用 Linuxbrew 修复了该依赖：
`brew install libcap` 后 `pkg-config` 输出
`-I/home/zhangxu/.linuxbrew/Cellar/libcap/2.78/include ... -lcap`，默认
`cargo build -p codex-cli --bin whale --locked` 和
`cargo test -p codex-linux-sandbox --lib --locked` 均通过。

如果缺少 `libcap.pc`，默认构建会在 vendored bubblewrap 阶段失败：

```text
The system library `libcap` required by crate `codex-linux-sandbox` was not found.
```

只有在 focused `codex-core` 单测确认不覆盖 Linux sandbox/bubblewrap 时，才跳过
vendored bubblewrap：

```bash
cd third_party/codex-cli/codex-rs
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core <test_name> --lib
```

这只适用于 TaskSpace normalizer、ActionMap summary 等非 sandbox 单元覆盖。
release/full gate、CLI attestation、sandbox 相关变更和健康开发环境检查必须使用
不带 `CODEX_SKIP_VENDORED_BWRAP` 的默认构建。

For low-disk machines, follow `docs/runbooks/cross-system-restore.md` and set
`CARGO_TARGET_DIR` outside the repo before building.

Archived-demo expected active toolchain:

```text
stable-aarch64-apple-darwin (overridden by '<repo>/rust-toolchain.toml')
```

## Notes

- The archived demo keeps its `Cargo.lock` because it contained a CLI binary.
- `target/` is ignored and should not be committed.
- If a future shell cannot find `cargo`, first check whether `/opt/homebrew/opt/rustup/bin` is on `PATH`.
- If `~/.rustup/settings.toml` already exists, the rustup installer may restore
  the previously configured default toolchain even when the current shell cannot
  find `cargo`; source `~/.cargo/env` before reinstalling or debugging build
  failures.
- Do not install the archived `whale` demo as the active CLI. The next active
  CLI should be rebuilt from the Codex substrate migration.
