# WhaleCode

DeepSeek-first terminal AI coding agent.

## V1 Goal

The first version is a generic coding agent CLI substrate comparable to mainstream
tools such as Codex CLI, Claude Code, OpenCode, and Pi. WhaleCode-specific
capabilities are added through pluggable Primitive Modules rather than being
hard-coded into the agent loop.

## Local Development

Requires Rust stable.

```bash
cargo fmt --check
cargo test --workspace
cargo run -p whalecode-cli -- status
```

Current workspace status: scaffolded. The first implementation target is the V1
generic agent CLI loop: model runtime, tools, permission, patch safety, session
replay, context management, and primitive host skeleton.
