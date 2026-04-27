# Codex Sync: Whale Brand And DeepSeek Overlay

Date: 2026-04-27

## Source

| Field | Value |
| --- | --- |
| Previous Codex commit | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` |
| New Codex commit | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` |
| Upstream ref | `refs/heads/main` |
| Upstream repository | https://github.com/openai/codex |
| Import commit | `8991de2` |
| Vendor path | `third_party/codex-cli/` |

## Overlay Scope

This step keeps the latest Codex CLI tree as the substrate and applies the
minimum Whale runtime overlay needed to make the vendored CLI build and run as a
separate product.

- CLI binary: `codex` is exposed as `whale`.
- Runtime home: `WHALE_HOME` and `~/.whale` replace `CODEX_HOME` / `~/.codex`
  for user-visible configuration and logs.
- User-facing text: help, login, onboarding, TUI labels, tooltips, resume
  instructions, MCP examples, attribution, and keyring labels use Whale naming.
- Provider default: DeepSeek is the default built-in provider.
- Default models: `deepseek-v4-pro` for normal execution and
  `deepseek-v4-flash` for fast low-latency roles.
- Wire protocol: DeepSeek uses OpenAI-compatible Chat Completions streaming,
  including `reasoning_content` deltas and streamed tool-call assembly.
- Upstream mergeability: internal crate names and many `codex_*` APIs remain
  unchanged unless they leak into Whale user experience or runtime storage.

## Disabled Or Hidden Upstream Features

The following OpenAI-specific capabilities are intentionally hidden or disabled
until Whale has compatible implementations.

- Native image input: command flags are hidden, DeepSeek models are text-only,
  and any accidental image payload is converted to a text placeholder before
  the Chat Completions request.
- Native web search: search flags and bundled model capability defaults are
  hidden for DeepSeek.
- Desktop app / cloud / app-server integration: command surfaces are hidden and
  the app integration entry point returns an explicit Whale-disabled error.
- Remote auth and device-code login: hidden from normal Whale help surfaces.
- Self-update: disabled so a Whale binary cannot install upstream Codex over
  itself.

## Tests Run

All commands were run from `third_party/codex-cli/codex-rs` unless noted.

```bash
cargo check -p codex-cli --locked
cargo test -p codex-api --locked chat_completions
cargo test -p codex-model-provider-info --locked
cargo test -p codex-core --locked defaults_to_deepseek_pro_provider
cargo test -p codex-core --locked responses_websocket_features_do_not_change_wire_api
cargo test -p codex-core --locked config_schema_matches_fixture
cargo run --quiet -p codex-cli --bin whale -- --version
cargo run --quiet -p codex-cli --bin whale -- --help
cargo run --quiet -p codex-cli --bin whale -- debug models --bundled
```

`cargo run` smoke tests used isolated runtime directories so the local installed
Codex and local Codex configuration were not touched:

```bash
HOME=/tmp/whalecode-whale-smoke.igAqqq/home
WHALE_HOME=/tmp/whalecode-whale-smoke.igAqqq/whale-home
unset CODEX_HOME OPENAI_API_KEY DEEPSEEK_API_KEY
```

`CARGO_HOME=/Users/xuzhang/.cargo` and `RUSTUP_HOME=/Users/xuzhang/.rustup`
were kept so the smoke test reused the Rust toolchain and crate cache without
reading Codex product configuration.

## Operational Notes

- `just` was unavailable in this checkout, so `cargo fmt` was used as the
  formatter fallback. Rustfmt emitted the existing nightly-only warning for
  `imports_granularity = Item` but exited successfully.
- The shared volume ran out of space during incremental builds. To avoid
  irreversible deletion, regenerable Cargo build-cache directories were moved
  into recoverable `/tmp/whalecode-target-backup-*` directories and builds were
  rerun with `CARGO_INCREMENTAL=0`.
- If network access is unstable during future syncs, use the configured local
  proxy (`127.0.0.1:7890`) for GitHub and Cargo traffic before falling back to
  codeload tarball imports.

## Future Upstream Sync Strategy

1. Fetch or download the new `openai/codex` upstream snapshot and record its
   commit in `third_party/codex-cli/UPSTREAM.md`.
2. Compare the new snapshot against this baseline before replaying local Whale
   changes.
3. Replay the Whale overlay in narrow groups: brand/home isolation, DeepSeek
   provider defaults, Chat Completions adapter, unsupported feature hiding.
4. Prefer keeping upstream crate names and internal APIs when they do not leak
   into Whale runtime behavior.
5. Rerun the tests listed above, then add a new dated sync log with conflicts,
   adopted upstream features, disabled features, and residual risks.

## Residual Risks

- Full workspace `cargo test` was not run because this change touched a large
  vendored Rust workspace and local disk pressure was high.
- Some internal upstream names and snapshot filenames still include `codex` by
  design to preserve future upstream mergeability.
- DeepSeek V4 exact production feature flags should be checked with a provider
  probe before enabling image, search, or additional tool modalities.
