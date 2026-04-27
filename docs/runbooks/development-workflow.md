# Whale Development Workflow Manual

Date: 2026-04-28

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

Some spawned automation shells may not inherit the user PATH immediately. If
`cargo` is not recognized but Rust is installed for the user, repair only the
current process before running tests:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Why Full Builds Are Slow

The first measured Windows bottleneck was not a single slow command. It was
dependency fan-out.

`codex-models-manager` is on the path into `codex-core`,
`codex-app-server`, `codex-tui`, `codex-exec`, and finally `codex-cli`.
Changing model catalog or default-model code can therefore invalidate much of
the CLI stack. With `CARGO_INCREMENTAL=0`, Cargo cannot reuse the usual local
incremental state, so even debug rebuilds can stay slow.

Release installs used to be slower again because the old release profile used
expensive final optimization and link settings. The old settings in
`third_party/codex-cli/codex-rs/Cargo.toml` are:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
```

`fat` LTO plus `codegen-units = 1` intentionally optimizes across the whole
program, but it also collapses the final codegen and link path into one or a few
long CPU-bound units. On Windows this can look like Cargo is stuck even while
`rustc.exe` is still consuming CPU. This is a build-profile bottleneck, not a
sign that the machine is too slow.

The 2026-04-28 release-build probe showed this shape clearly: helper binaries
finished quickly, `.fingerprint` timestamps advanced through
`codex-windows-sandbox`, `codex-app-server`, and `codex-tui`, but the final
`release\whale.exe` stayed stale while release `rustc.exe` work continued for
more than 20 minutes. The bottleneck is the `whale` release codegen/link path,
especially the `codex-tui` and final CLI dependency closure.

The corrected policy is:

- `release`: local optimized smoke profile, `opt-level = 1`, `lto = false`,
  `incremental = true`, `codegen-units = 256`, and no symbol stripping.
- `dist`: explicit production distribution profile, `opt-level = 3`,
  `lto = "fat"`, `incremental = false`, `codegen-units = 1`, and symbol
  stripping.

This follows Cargo's own profile model: `--release` is just
`--profile release`, custom profiles inherit from a named profile, and each
custom profile writes to its own target directory.

The corrected Windows measurements on 2026-04-28:

```text
cold cargo build -p codex-cli --bin whale --release --locked: 13m 06s
warm cargo build -p codex-cli --bin whale --release --locked: 3.2s
cold-ish cargo build -p codex-cli --bin whale --locked after profile/helper churn: 2m 55s
warm cargo build -p codex-cli --bin whale --locked: 3.0s
cold-ish cargo build -p codex-cli --bin whale --release --locked after helper split: 14m 16s
warm cargo build -p codex-cli --bin whale --release --locked: 3.4s
steady warm cargo build --release --locked --bin whale plus all forwarded helpers: 2.5s
```

The next dependency split moved hidden and non-primary command ownership out of
the top-level CLI. `whale` now forwards these surfaces to sibling helpers:

- `whale app-server ...` -> `whale-app-server`
- `whale mcp-server` -> `whale-mcp-server`
- `whale cloud ...` / `whale cloud-tasks ...` -> `whale-cloud-tasks`
- `whale responses-api-proxy ...` -> `whale-responses-api-proxy`
- `whale stdio-to-uds ...` -> `whale-stdio-to-uds`
- `whale exec-server ...` -> `whale-exec-server`
- `whale debug app-server send-message-v2 ...` ->
  `whale-app-server-test-client`

Helpers that need to re-enter the agent CLI receive the original `whale`
binary path via hidden runtime flags, so the split does not accidentally make a
helper spawn itself. Keep those runtime flags private implementation detail.

This removes app-server, MCP server, cloud task UI, exec-server, stdio bridge,
proxy, and app-server test-client implementation crates from the main CLI
dependency closure. The main binary still carries the core agent stack, TUI, and
non-interactive exec path. The remaining heavy transitive app-server cost now
enters through `codex-app-server-client` in `codex-tui` and `codex-exec`, not
through hidden slash or debug helper command ownership. Further cold-build cuts
must split that public TUI/exec app-server transport boundary; do not put helper
crates back into `codex-cli`.

The cloud-task mock backend is also now a dev-dependency, so normal local and
release builds do not compile the test-only mock client.

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
| TUI footer/status line | `cargo test -p codex-tui footer_` and `cargo test -p codex-tui status_line_` | Composer layout, token budget, or snapshot baselines are affected. |
| App-server CLI/helper | `cargo check -p codex-app-server --bin whale-app-server --locked` | VS Code/app-server protocol behavior changed. |
| Forwarded helper command | `cargo check -p <helper-crate> --bin <helper-binary> --locked` | Local install or npm/release packaging changed. |

Prefer package-level tests before building the full CLI. A full CLI build is a
smoke gate, not the first response to every small Rust edit.

On Windows, full `cargo test -p codex-tui` can overflow the default Rust test
thread stack before it reaches the actual changed surface. If you need the full
package suite, set a larger stack for the current shell first:

```powershell
$env:RUST_MIN_STACK = "8388608"
cargo test -p codex-tui
```

If that full suite emits unrelated `.snap.new` files while you are investigating
a narrow TUI change, move those generated files to a dated temp backup rather
than deleting them, then rerun the smallest matching snapshot tests without
`INSTA_UPDATE`.

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

Use the default release profile for local optimized builds, package smoke, and
performance checks:

```powershell
cargo build -p codex-cli --bin whale --release --locked
Set-Location D:\WhaleCode
.\scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\release\whale.exe -PersistUserPath -BackupLegacyCopies
```

Build helper binaries only when you need to exercise the forwarded helper
commands locally:

```powershell
cargo build -p codex-app-server --bin whale-app-server --release --locked
cargo build -p codex-app-server-test-client --bin whale-app-server-test-client --release --locked
cargo build -p codex-cloud-tasks --bin whale-cloud-tasks --release --locked
cargo build -p codex-exec-server --bin whale-exec-server --release --locked
cargo build -p codex-mcp-server --bin whale-mcp-server --release --locked
cargo build -p codex-responses-api-proxy --bin whale-responses-api-proxy --release --locked
cargo build -p codex-stdio-to-uds --bin whale-stdio-to-uds --release --locked
Set-Location D:\WhaleCode
.\scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\release\whale.exe -PersistUserPath -BackupLegacyCopies
```

The installer copies all forwarded helper binaries when they exist next to the
selected `whale.exe`. If a forwarded command reports that a helper is missing,
build the specific helper binary above and rerun the installer.

Use the explicit dist profile only for final distribution when binary size is
worth the extra compile time:

```powershell
cargo build -p codex-cli --bin whale --profile dist --locked
Set-Location D:\WhaleCode
.\scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\dist\whale.exe -PersistUserPath -BackupLegacyCopies
```

Do not use `cargo install` as the Whale local install path, because it writes
into shared Cargo bin directories instead of the isolated
`%USERPROFILE%\.whale\bin` directory.

If a build appears stuck, check the actual processes before assuming a hang:

```powershell
Get-Process cargo,rustc,link -ErrorAction SilentlyContinue |
  Select-Object Id,ProcessName,CPU,StartTime,Path
Get-CimInstance Win32_Process -Filter "name='rustc.exe'" |
  Select-Object ProcessId,CommandLine
```

Run the profile guard after changing Cargo profiles or this runbook:

```powershell
.\scripts\check-build-profile-policy.ps1
```

Cargo references:

- https://doc.rust-lang.org/cargo/reference/profiles.html
- https://doc.rust-lang.org/book/ch14-01-release-profiles.html

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

The Whale npm package under `third_party/codex-cli/codex-cli` is named
`@ceasarxuu/whalecode` and exposes only the `whale` command. It must not publish
or install `@openai/codex`, `codex.js`, or a `codex` command. See
`docs/runbooks/npm-publishing.md` before any npm release.

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
