# 2026-05-01 Selective Codex Upstream Backports

## Scope

Compared Whale's vendored Codex snapshot at `fed0a8f4faa58db3138488cca77628c1d54a2cd8`
with official `openai/codex` `main` at `6014b6679ffbd92eeddffa3ad7b4402be6a7fefe`.
The official range contains 225 commits, so this was intentionally handled as
selective backports instead of a whole-vendor refresh.

## Backported

- `8426edf71` Stateful streaming `apply_patch` parser.
- `5cac3f896`, `cecca5ae0`, `9d1e5df4b`, `13dbcda28`, `8121710ff`,
  `06f3b4836`: Windows sandbox/process/env fixes.
- `3516cb975`: truncate large MCP tool outputs before rollout/app-server
  persistence.
- `4e0cf945b`: explicit MCP client shutdown/drain, adapted to Whale's
  `mcp_connection_manager.rs` path while preserving
  `list_all_tools_non_blocking()`.

## Deferred

- `e20391e56` plugin MCP approval policy was not absorbed in this pass. It
  conflicts with plugin/config-loader internals and should be handled as a
  dedicated plugin-policy migration, not mixed into MCP lifecycle cleanup.

## Local Adaptations

- Kept Whale's current `SandboxPolicy`-based `ToolsConfigParams`; did not pull
  upstream's newer permission-profile shape into this batch.
- Kept Whale's non-blocking MCP first-turn path. Shutdown support was added
  around the existing startup snapshot cache instead of replacing it.
- Kept DeepSeek/provider/model/brand surfaces out of these backports.
- Fixed the Windows env-var backport to reference
  `codex_config::shell_environment::WINDOWS_CORE_ENV_VARS`, which is where this
  checkout exposes the constant.

## Verification

- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-apply-patch`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core apply_patch --lib`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-tools tool_config --lib`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-windows-sandbox`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-mcp`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core mcp --lib`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core truncate_mcp_tool_result_for_event --lib`
- `cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core mcp_tool_output --lib`

`cargo test -p codex-rmcp-client` compiled but had three OAuth/keyring fallback
file assertion failures unrelated to the MCP shutdown backport:

- `oauth::tests::delete_oauth_tokens_file_mode_removes_keyring_only_entry`
- `oauth::tests::delete_oauth_tokens_removes_all_storage`
- `oauth::tests::save_oauth_tokens_prefers_keyring_when_available`

`rmcp-client/tests/process_group_cleanup.rs` is Unix-only, so it compiles on
Windows but runs zero tests here.
