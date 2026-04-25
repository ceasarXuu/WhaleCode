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
```

Current workspace status: bootstrap CLI loop plus DeepSeek request/SSE adapter
foundation. The `whale` binary can start a replayable local bootstrap agent
turn, run read-only workspace tools, and persist JSONL session events. The model
crate can build DeepSeek chat-completion requests and parse streaming
`reasoning_content`, text, and tool-call deltas. Wiring live DeepSeek execution
into the AgentLoop, mutating tools, patch safety, context compaction, and
primitive host execution are still follow-up milestones.

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

More setup details are in `docs/runbooks/rust-development-environment.md`.
