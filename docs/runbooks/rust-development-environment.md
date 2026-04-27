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
cargo run --quiet -p codex-cli --bin whale -- --version
```

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
