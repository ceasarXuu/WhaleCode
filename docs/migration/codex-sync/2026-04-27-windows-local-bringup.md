# Windows Local Bring-Up

Date: 2026-04-27

This records the first native Windows development-environment bring-up on the
current machine for the Codex-derived WhaleCode workspace.

## Source State

```text
cwd: D:\WhaleCode
branch: main
HEAD: 3c7f05a04edd8848545de548922e03377ecfd3bf
upstream ahead/behind: 0 0
working tree before doc update: clean
```

Codex upstream snapshot:

```text
upstream repository: https://github.com/openai/codex
upstream commit: fed0a8f4faa58db3138488cca77628c1d54a2cd8
local vendor path: third_party/codex-cli/
```

## Installed And Verified Tools

```text
rustup: installed through winget, rustup 1.29.0
active project toolchain: 1.93.0-x86_64-pc-windows-msvc
rustc: rustc 1.93.0 (254b59607 2026-01-19)
cargo: cargo 1.93.0 (083ac5135 2025-12-15)
node: v24.12.0
pnpm: 10.33.0
ripgrep: 15.1.0
just: 1.50.0
cargo-insta: 1.47.2
bazelisk: v1.28.1
bazel runtime selected by bazelisk: 9.0.0
```

Visual Studio Community 2022 was already installed. `cl.exe` and `link.exe`
were not present in a normal PowerShell path, so Rust build commands were run
through:

```text
C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat
```

Build output was externalized:

```text
WHALE_CACHE_ROOT=D:\BuildCache\whalecode
CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target
CARGO_INCREMENTAL=0
```

## Setup Notes

`corepack enable` failed with an `EPERM` when attempting to create pnpm shims
under `C:\Program Files\nodejs`. The working path was to set a user-level npm
prefix at `%APPDATA%\npm` and install `pnpm@10.33.0` globally there.

The first `cargo check` attempt failed because a `cmd.exe` environment variable
assignment used `set CARGO_TARGET_DIR=... && ...`. In `cmd.exe`, the space
before `&&` became part of the value. Using `set "CARGO_TARGET_DIR=..."` fixed
the issue.

`rg.exe` initially resolved to the Codex app's WindowsApps resource and failed
with `Access is denied`. Installing `BurntSushi.ripgrep.MSVC` through winget
provided a working alias under `%LOCALAPPDATA%\Microsoft\WinGet\Links`.

## Build And Smoke Results

JavaScript maintenance dependencies:

```text
pnpm install --frozen-lockfile
result: passed
side effect: sdk/typescript prepare built its dist output inside ignored node_modules workflow
```

Rust build gates:

```text
cargo check -p codex-cli --locked
result: passed

cargo build -p codex-cli --bin whale --locked
result: passed
binary: D:\BuildCache\whalecode\cargo-target\debug\whale.exe
```

Isolated runtime smoke:

```text
whale --version: whale 0.0.0
whale --help: starts with "Whale CLI"
forbidden normal help matches: none for codex, --image, cloud, app-server, --remote, chatgpt, openai
```

Targeted regression tests:

```text
cargo test -p codex-api --locked chat_completions
result: passed, 2 tests

cargo test -p codex-model-provider-info --locked
result: passed, 20 tests

cargo test -p codex-core --locked defaults_to_deepseek_pro_provider
result: passed, 1 test

cargo test -p codex-core --locked responses_websocket_features_do_not_change_wire_api
result: passed, 1 test

cargo test -p codex-core --locked config_schema_matches_fixture
result: passed, 1 test
```

No live DeepSeek model smoke was run in this bring-up, because that requires an
intentional `DEEPSEEK_API_KEY` and expected network/billing use.
