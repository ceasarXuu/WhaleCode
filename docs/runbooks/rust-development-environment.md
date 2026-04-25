# Rust Development Environment Runbook

Date: 2026-04-25

## Context

WhaleCode uses a Rust-first core. The repo pins the toolchain through `rust-toolchain.toml`:

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

In this environment the paths are appended to `~/.zshrc`, not `~/.zprofile`, so new interactive zsh sessions can find `cargo`, `rustc`, `rustup`, and locally installed CLI binaries such as `whale`.

Non-interactive Codex shell calls may not read `~/.zshrc`. Prefix verification commands when needed:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH" cargo test --workspace
```

## Verification

From the repo root:

```bash
cargo --version
rustc --version
rustup show active-toolchain
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p whalecode-cli --bin whale -- status
cargo run -p whalecode-cli --bin whale -- run "inspect this repo"
```

Install and verify the local CLI:

```bash
cargo install --path crates/whalecode-cli --force --locked
zsh -ic 'which whale && whale status'
```

Expected active toolchain:

```text
stable-aarch64-apple-darwin (overridden by '<repo>/rust-toolchain.toml')
```

## Notes

- `Cargo.lock` is committed because the workspace contains a CLI binary.
- `target/` is ignored and should not be committed.
- If a future shell cannot find `cargo`, first check whether `/opt/homebrew/opt/rustup/bin` is on `PATH`.
- If a future shell cannot find `whale` after `cargo install`, check whether `$HOME/.cargo/bin` is on `PATH`.
