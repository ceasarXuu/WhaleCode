# WhaleCode

DeepSeek-first terminal AI coding agent.

## V1 Goal

The first version is a generic coding agent CLI substrate comparable to mainstream
tools such as Codex CLI, Claude Code, OpenCode, and Pi. WhaleCode-specific
capabilities are added through pluggable Primitive Modules rather than being
hard-coded into the agent loop.

## Local Development

Requires Rust stable. On macOS, install with:

```bash
brew install rustup-init
/opt/homebrew/opt/rustup/bin/rustup default stable
/opt/homebrew/opt/rustup/bin/rustup component add rustfmt clippy
```

If `cargo` is not found in a new zsh session, add this to `~/.zshrc`:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

```bash
cargo fmt --check
cargo test --workspace
cargo run -p whalecode-cli --bin whale -- status
cargo run -p whalecode-cli --bin whale -- run "inspect this repo"
cargo run -p whalecode-cli --bin whale -- run --live "inspect this repo"
```

Current workspace status: bootstrap CLI loop plus a live DeepSeek tool loop. The
`whale` binary can start a replayable local bootstrap agent turn, run read-only
workspace tools, persist JSONL session events, stream DeepSeek text/reasoning
and tool-call deltas, apply `edit_file` through a patch-safe exact replacement
path when `--allow-write` is explicit, and run bounded verification commands
when `--allow-command` is explicit. Context compaction and primitive host
execution are still follow-up milestones.

Install the local CLI into your active Cargo bin directory:

```bash
cargo install --path crates/whalecode-cli --force --locked
whale status
whale run "inspect this repo"
```

Optional DeepSeek environment variables for the upcoming live adapter wiring:

```bash
export DEEPSEEK_API_KEY="..."
export DEEPSEEK_MODEL="deepseek-v4-flash"
export DEEPSEEK_BASE_URL="https://api.deepseek.com"
```

After setting `DEEPSEEK_API_KEY`, run a provider-only smoke test:

```bash
whale model-smoke --model deepseek-v4-flash "say hello"
```

`model-smoke` does not run tools or edit files; it only verifies live model auth,
streaming, and response aggregation.

Run the live agent against the current repository:

```bash
whale run --live "inspect this repo"
whale run --live --allow-write --allow-command "fix the bug in src/lib.rs and run the relevant test"
```

Without `--allow-write`, `edit_file` calls are rejected and recorded in the
session log. With `--allow-write`, edits still require an exact old-string match
and a fresh file snapshot before Whale writes to disk. Without
`--allow-command`, `run_command` calls are rejected and recorded; with
`--allow-command`, commands run in the workspace with argument arrays and a
timeout rather than through a shell string.

More setup details are in `docs/runbooks/rust-development-environment.md`.
