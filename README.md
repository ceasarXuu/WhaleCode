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

Workspace behavior is intentionally simple in V1: the process current directory
is the default workspace. Run `whale` from a project root to make that folder the
workspace, or pass `--cwd <path>` to override it for `whale run`. Read/search,
patch-safe edits, session `cwd` events, and gated verification commands all use
the selected workspace as their root.

Current workspace status: live DeepSeek tool loop as the default agent path. The
`whale` binary sends every natural-language user input through the live Agent,
can run read-only workspace tools, persist JSONL session events, stream DeepSeek
text/reasoning and tool-call deltas, apply `edit_file` through a patch-safe exact
replacement path, and run bounded verification commands when `--allow-command`
is explicit. Session logs now include turn grouping, model stream events,
permission decisions, tool outputs, and patch results so a run can be replayed
from the terminal. Context compaction and primitive host execution are still
follow-up milestones. The old bootstrap-local runtime is only kept for explicit
`whale run --bootstrap` debugging.

Install the local CLI into your active Cargo bin directory:

```bash
cargo install --path crates/whalecode-cli --force --locked
whale status
whale run "inspect this repo"
```

Store the DeepSeek API key from inside Whale:

```text
whale
whale> /apikey
DeepSeek API key:
```

The key is saved under the user-level secret store
`~/.whale/secrets/deepseek_api_key` with private file permissions on Unix-like
systems. It is not written to the repository. `DEEPSEEK_API_KEY` still takes
priority when present, which is useful for temporary overrides or CI.

Optional DeepSeek environment variables:

```bash
export DEEPSEEK_API_KEY="..."
export DEEPSEEK_MODEL="deepseek-v4-flash"
export DEEPSEEK_BASE_URL="https://api.deepseek.com"
```

After storing the key or setting `DEEPSEEK_API_KEY`, run a provider-only smoke
test:

```bash
whale model-smoke --model deepseek-v4-flash "say hello"
```

`model-smoke` does not run tools or edit files; it only verifies live model auth,
streaming, and response aggregation.

Run the live agent against the current repository:

```bash
whale run "inspect this repo"
whale run --allow-write --allow-command "fix the bug in src/lib.rs and run the relevant test"
whale logs
whale logs --session ~/.whale/sessions/session-....jsonl
```

Without `--allow-write`, `edit_file` calls are rejected and recorded in the
session log. With `--allow-write`, edits still require an exact old-string match
and a fresh file snapshot before Whale writes to disk. Without
`--allow-command`, `run_command` calls are rejected and recorded; with
`--allow-command`, commands run in the workspace with argument arrays and a
timeout rather than through a shell string.

More setup details are in `docs/runbooks/rust-development-environment.md`.
