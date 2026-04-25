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
```

In this environment the path is appended to `~/.zshrc`, not `~/.zprofile`, so new interactive zsh sessions can find `cargo`, `rustc`, and `rustup`.

## Verification

From the repo root:

```bash
cargo --version
rustc --version
rustup show active-toolchain
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p whalecode-cli -- status
```

Expected active toolchain:

```text
stable-aarch64-apple-darwin (overridden by '<repo>/rust-toolchain.toml')
```

## Notes

- `Cargo.lock` is committed because the workspace contains a CLI binary.
- `target/` is ignored and should not be committed.
- If a future shell cannot find `cargo`, first check whether `/opt/homebrew/opt/rustup/bin` is on `PATH`.
