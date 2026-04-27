# Whale Development Workflow Manual

Date: 2026-04-27

Use this manual for day-to-day Whale development. It turns the first Windows
bring-up lessons into a repeatable inner loop.

## Workspace

The active Rust workspace is:

```powershell
Set-Location D:\WhaleCode\third_party\codex-cli\codex-rs
```

The repository root is not an active Cargo workspace. Run Rust build, test, and
install commands from `third_party/codex-cli/codex-rs`.

## Build Environment

On Windows, use MSVC Rust. If the shell is not already a Developer PowerShell,
load Visual Studio tools before Cargo commands:

```powershell
$VsDevCmd = "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
cmd /d /s /c "call `"$VsDevCmd`" -arch=x64 -host_arch=x64 >nul && cd /d D:\WhaleCode\third_party\codex-cli\codex-rs && cargo check -p codex-cli --locked"
```

Move build output out of the source tree:

```powershell
$env:WHALE_CACHE_ROOT = "D:\BuildCache\whalecode"
New-Item -ItemType Directory -Force $env:WHALE_CACHE_ROOT | Out-Null
$env:CARGO_TARGET_DIR = Join-Path $env:WHALE_CACHE_ROOT "cargo-target"
```

For normal local development, keep incremental compilation enabled:

```powershell
$env:CARGO_INCREMENTAL = "1"
```

Use `CARGO_INCREMENTAL=0` only for clean reproduction, CI-like checks, or when
you are deliberately trading rebuild speed for less incremental state.

## Why Full Debug Builds Are Slow

The first measured Windows bottleneck was not a single slow command. It was
dependency fan-out.

`codex-models-manager` is on the path into `codex-core`,
`codex-app-server`, `codex-tui`, `codex-exec`, and finally `codex-cli`.
Changing model catalog or default-model code can therefore invalidate much of
the CLI stack. With `CARGO_INCREMENTAL=0`, Cargo cannot reuse the usual local
incremental state, so even debug rebuilds can stay slow.

Release installs are slower again because the current release profile uses
expensive final optimization and link settings. Do not use release install as
the default edit-test loop.

## Inner Loop Rules

Choose the smallest valid gate for the files you changed.

| Change area | First gate | Escalate when |
| --- | --- | --- |
| Documentation only | `git diff --check` | Links, commands, or paths changed and need live validation. |
| Model catalog/default selection | `cargo test -p codex-models-manager --locked` | TUI or app-server model picker behavior is affected. |
| Core model defaults/config | `cargo test -p codex-core --locked defaults_to_deepseek_pro_provider` | Provider routing, auth, or config schema changed. |
| App-server model list | `cargo test -p codex-app-server --test all --locked model_list` | Web/API model selection behavior changed. |
| Provider/API transport | `cargo test -p codex-api --locked chat_completions` | SSE, streaming, auth, or usage parsing changed. |
| TUI/CLI surface | `cargo build -p codex-cli --bin whale --locked` | Manual TUI smoke or local install is needed. |

Prefer package-level tests before building the full CLI. A full CLI build is a
smoke gate, not the first response to every small Rust edit.

## DeepSeek Default Model Gate

After changing model catalog, default picker, provider visibility, or Whale
branding, run:

```powershell
cargo test -p codex-models-manager --locked
cargo test -p codex-core --locked defaults_to_deepseek_pro_provider
cargo test -p codex-app-server --test all --locked model_list
```

Build the CLI only after these pass:

```powershell
cargo build -p codex-cli --bin whale --locked
```

Install the debug binary for local TUI smoke:

```powershell
Set-Location D:\WhaleCode
.\scripts\install-whale-local.ps1 -PersistUserPath -BackupLegacyCopies
whale --version
whale debug models
```

The isolated local install path is `%USERPROFILE%\.whale\bin\whale.exe`.
Do not copy Whale into `%USERPROFILE%\.cargo\bin`, `%USERPROFILE%\.local\bin`,
`%APPDATA%\npm`, or WindowsApps. Those are shared tool locations and can make
Whale appear coupled to official Codex or npm-installed CLIs.

Verify the resolved binary and CLI separation:

```powershell
where.exe whale
where.exe codex
.\scripts\check-cli-isolation.ps1
```

Existing terminals and long-running agent processes may keep an old PATH until
they are restarted. `check-cli-isolation.ps1` refreshes PATH from the user and
machine environment by default to validate what a new terminal will see. Use
`-UseCurrentProcessPath` only when you intentionally want to diagnose the
currently running shell.

If install fails or a new terminal still shows old behavior, check for a
running TUI that is holding the old executable open:

```powershell
Get-Process whale -ErrorAction SilentlyContinue |
    Select-Object Id,Path,StartTime
```

Expected first picker entries:

```text
deepseek-v4-pro
deepseek-v4-flash
```

No GPT, ChatGPT, OpenAI, or Codex-branded model should appear in the picker.
`deepseek-v4-pro` should be marked as the default/current model unless the user
has explicitly selected another model in config.

## Release Build Policy

Use release install for packaging, performance checks, or final distribution:

```powershell
cargo build -p codex-cli --bin whale --release --locked
Set-Location D:\WhaleCode
.\scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\release\whale.exe -PersistUserPath -BackupLegacyCopies
```

Do not use it as the normal local smoke path. On Windows it can spend a long
time in final release optimization and linking even after codegen appears
mostly complete. Do not use `cargo install` as the Whale local install path,
because it writes into shared Cargo bin directories instead of the isolated
`%USERPROFILE%\.whale\bin` directory.

## Runtime Configuration Smoke

Use user or process environment variables for secrets. Do not commit secrets to
the repository:

```powershell
$env:DEEPSEEK_API_KEY = "replace-with-real-key"
$env:WHALE_HOME = "$env:USERPROFILE\.whale"
```

For an installed local debug build:

```powershell
whale --version
whale debug models
```

Use a live model smoke only when network access and billing are expected:

```powershell
whale exec "Reply with one short sentence."
```

When validating DeepSeek thinking mode with tools, use a prompt that forces at
least one read-only command:

```powershell
$env:DEEPSEEK_API_KEY = [Environment]::GetEnvironmentVariable("DEEPSEEK_API_KEY", "User")
whale exec "Run a read-only directory listing of D:\WhaleCode, then reply with exactly: OK"
```

This catches the DeepSeek protocol requirement that assistant messages with
tool calls must carry the matching `reasoning_content` back into subsequent
Chat Completions requests.

## Documentation And Log Discipline

Every repeated operational lesson should land in documentation before it is
forgotten. Update the closest runbook or migration log when you learn something
about:

- build setup;
- login or API-key configuration;
- local install paths;
- slow build bottlenecks;
- test gates;
- packaging and upload commands;
- failure recovery.

Runtime feature changes should also add structured logs or session events where
they help future diagnosis. Documentation is not a substitute for runtime
observability.

## Official Codex Isolation

Whale development must not mutate official Codex installation or runtime state.
Keep these boundaries:

- Whale binary: `%USERPROFILE%\.whale\bin\whale.exe`
- Whale runtime state: `%USERPROFILE%\.whale` or process-scoped `WHALE_HOME`
- official Codex npm package: `%APPDATA%\npm\node_modules\@openai\codex`
- official Codex app package: `%ProgramFiles%\WindowsApps\OpenAI.Codex_*`
- official Codex runtime state: `%USERPROFILE%\.codex`

Do not install Whale into npm global directories, WindowsApps, `.cargo\bin`, or
`.local\bin`. Do not copy `.codex` into `.whale`, and do not point
`CODEX_HOME` at `WHALE_HOME`. Whale also rejects `WHALE_HOME` values that point
at an official `.codex` state directory or the same path as `CODEX_HOME`.

Run the isolation guard after changing install scripts, PATH setup, wrapper
files, or local machine configuration:

```powershell
.\scripts\check-cli-isolation.ps1
.\scripts\check-codex-collision-risk.ps1
```

If official Codex reports a missing optional dependency, repair Codex itself
without changing Whale:

```powershell
npm install -g @openai/codex@latest --include=optional
codex --version
```

The vendored upstream npm package under `third_party/codex-cli/codex-cli` still
has upstream `@openai/codex` package metadata. Treat it as source/vendor input,
not as a Whale installer. Never run a global npm install from that package for
local Whale development unless the package metadata has first been renamed and
validated as a Whale package.

## Git Discipline

Stay on the current branch unless the user explicitly approves a new branch.
Commit and push small completed themes. Leave no uncommitted repository changes
after a finished task.

Before commit:

```powershell
git status --short --branch
git diff --check
```

After commit:

```powershell
git status --short --branch
git push origin main
```
