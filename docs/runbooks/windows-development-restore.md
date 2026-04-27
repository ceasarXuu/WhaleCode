# Windows Development Restore Runbook

Date: 2026-04-27

Use this to move WhaleCode development to a native Windows machine. This is for
Windows host development with the MSVC Rust toolchain, not WSL. WSL can be a
useful fallback for Linux parity, but it does not validate native Windows path,
process, terminal, keyring, or sandbox behavior.

## Current Windows Status

WhaleCode is intended to be cross-platform, but the current Whale overlay has
been built and smoke-tested on macOS first. Treat the first Windows machine as a
bring-up environment and keep the verification log explicit.

Known status:

- source of truth is `origin/main`
- active Rust workspace is `third_party/codex-cli/codex-rs`
- CLI binary target is `whale.exe`
- default runtime home is `%USERPROFILE%\.whale` unless `WHALE_HOME` is set
- default model provider is DeepSeek through `DEEPSEEK_API_KEY`
- OpenAI-specific app/cloud/remote/image/search surfaces are hidden or disabled

Windows-specific areas that still need real validation:

- process spawning and shell behavior under PowerShell and `cmd.exe`
- path normalization and long path behavior
- terminal rendering and input
- keyring behavior
- sandbox behavior
- file permission and symlink behavior

## Machine Requirements

Recommended baseline:

- Windows 11 x64
- at least 50 GB free disk for first builds and test runs
- a short checkout path such as `D:\dev\WhaleCode`
- PowerShell 7 or Windows PowerShell 5.1
- Git for Windows
- Visual Studio 2022 Build Tools with C++ tools
- Rust MSVC toolchain
- ripgrep
- Node.js 22 or newer plus `pnpm`
- `just` for local repository maintenance tasks
- `cargo-insta` for TUI and snapshot-test review
- Bazelisk when running Bazel-backed lockfile or lint tasks

Avoid deep paths under OneDrive, Desktop, or synced folders. They increase path
length, file-locking, and antivirus interference.

## Install Prerequisites

Open PowerShell as Administrator for package installation.

```powershell
winget install --id Git.Git -e
winget install `
  --id Microsoft.VisualStudio.2022.BuildTools `
  -e `
  --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup -e
winget install --id BurntSushi.ripgrep.MSVC -e
winget install --id Casey.Just -e
winget install --id Bazel.Bazelisk -e
```

Restart PowerShell after installation, then configure Rust:

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy
rustc --version
cargo --version
git --version
rg --version
just --version
```

If `link.exe` or Windows SDK files are not found during build, open a fresh
"Developer PowerShell for VS 2022" shell and retry the build there.

Install JavaScript and snapshot-test helpers after Node and Rust are available:

```powershell
corepack prepare pnpm@10.33.0 --activate
cargo install cargo-insta --locked
```

If `corepack enable` fails with `EPERM` while trying to write shims under
`C:\Program Files\nodejs`, use a user-level npm prefix instead:

```powershell
$UserNpm = Join-Path $env:APPDATA "npm"
New-Item -ItemType Directory -Force $UserNpm | Out-Null
npm config set prefix $UserNpm
npm install -g pnpm@10.33.0
$env:Path = "$UserNpm;$env:Path"
pnpm --version
```

Make sure future shells can resolve user-installed tools:

```powershell
$Need = @(
  "$env:USERPROFILE\.cargo\bin",
  "$env:APPDATA\npm",
  "$env:LOCALAPPDATA\Microsoft\WinGet\Links"
)
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$Parts = @()
if ($UserPath) {
  $Parts = $UserPath -split ";" | Where-Object { $_ }
}
foreach ($Path in $Need) {
  if ($Parts -notcontains $Path) {
    $Parts += $Path
  }
}
[Environment]::SetEnvironmentVariable("Path", ($Parts -join ";"), "User")
```

## Git And Path Setup

Enable Git long path support before cloning:

```powershell
git config --global core.longpaths true
```

If Windows still rejects long paths, enable OS long paths from an elevated
PowerShell and reboot:

```powershell
New-ItemProperty `
  -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
  -Name "LongPathsEnabled" `
  -Value 1 `
  -PropertyType DWord `
  -Force
```

Create a short development root:

```powershell
New-Item -ItemType Directory -Force D:\dev | Out-Null
Set-Location D:\dev
```

## Optional Proxy

If GitHub or Cargo traffic is unstable and the local proxy is available:

```powershell
$env:HTTPS_PROXY = "http://127.0.0.1:7890"
$env:HTTP_PROXY = "http://127.0.0.1:7890"
$env:ALL_PROXY = "socks5h://127.0.0.1:7890"
```

Unset the proxy when it is not available:

```powershell
Remove-Item Env:HTTPS_PROXY -ErrorAction SilentlyContinue
Remove-Item Env:HTTP_PROXY -ErrorAction SilentlyContinue
Remove-Item Env:ALL_PROXY -ErrorAction SilentlyContinue
```

Prefer process-scoped proxy variables first. Do not write proxy settings into
Git global config unless the Windows machine always uses that proxy.

## Clone And Verify

```powershell
git clone https://github.com/ceasarXuu/WhaleCode.git
Set-Location D:\dev\WhaleCode
git checkout main
git pull --ff-only
git status --short --branch
git log --oneline -3
Get-Content .\third_party\codex-cli\UPSTREAM.md -TotalCount 80
```

Expected:

- working tree is clean
- branch tracks `origin/main`
- latest commits include the Windows runbook commit, the Whale overlay commit,
  and the Codex import commit
- `UPSTREAM.md` records Codex upstream commit
  `fed0a8f4faa58db3138488cca77628c1d54a2cd8`

Verify no nested Git metadata was imported:

```powershell
Get-ChildItem .\third_party\codex-cli -Force -Recurse -Directory -Filter .git
```

Expected: no output.

## Move Build Output Off The Repo

The Codex workspace can create large build output. Set a dedicated target
directory before the first build:

```powershell
$env:WHALE_CACHE_ROOT = "D:\BuildCache\whalecode"
New-Item -ItemType Directory -Force $env:WHALE_CACHE_ROOT | Out-Null
$env:CARGO_TARGET_DIR = Join-Path $env:WHALE_CACHE_ROOT "cargo-target"
$env:CARGO_INCREMENTAL = "0"
```

Keep these variables in the same PowerShell session for all build and test
commands. If you want them persisted for your user later:

```powershell
[Environment]::SetEnvironmentVariable("WHALE_CACHE_ROOT", "D:\BuildCache\whalecode", "User")
[Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", "D:\BuildCache\whalecode\cargo-target", "User")
[Environment]::SetEnvironmentVariable("CARGO_INCREMENTAL", "0", "User")
```

## Build Whale

When using a normal PowerShell instead of "Developer PowerShell for VS 2022",
wrap build commands with `VsDevCmd.bat` so `cl.exe`, `link.exe`, and the Windows
SDK are available:

```powershell
$VsDevCmd = "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
cmd /d /s /c "call `"$VsDevCmd`" -arch=x64 -host_arch=x64 >nul && set `"PATH=%USERPROFILE%\.cargo\bin;%APPDATA%\npm;%LOCALAPPDATA%\Microsoft\WinGet\Links;%PATH%`" && set `"CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target`" && set `"CARGO_INCREMENTAL=0`" && cd /d D:\dev\WhaleCode\third_party\codex-cli\codex-rs && cargo check -p codex-cli --locked"
```

In `cmd.exe`, always use `set "NAME=value"` when chaining with `&&`. Plain
`set NAME=value && ...` can put the space before `&&` into the environment
variable value; for `CARGO_TARGET_DIR`, that produces a confusing path such as
`cargo-target ` and Cargo may fail before creating the target directory.

```powershell
Set-Location D:\dev\WhaleCode\third_party\codex-cli\codex-rs
cargo check -p codex-cli --locked
cargo build -p codex-cli --bin whale --locked
```

If `CARGO_TARGET_DIR` is set:

```powershell
& "$env:CARGO_TARGET_DIR\debug\whale.exe" --version
```

If `CARGO_TARGET_DIR` is not set:

```powershell
.\target\debug\whale.exe --version
```

Expected:

```text
whale 0.0.0
```

## Isolated Smoke Test

Run the first smoke test with isolated runtime variables. This avoids reading
or writing any existing Codex or Whale state.

```powershell
$WhaleBin = "$env:CARGO_TARGET_DIR\debug\whale.exe"
if (-not (Test-Path $WhaleBin)) {
  $WhaleBin = ".\target\debug\whale.exe"
}

$SmokeRoot = Join-Path $env:TEMP ("whalecode-smoke-" + [guid]::NewGuid())
$SmokeHome = Join-Path $SmokeRoot "home"
$SmokeWhaleHome = Join-Path $SmokeRoot "whale-home"
New-Item -ItemType Directory -Force $SmokeHome, $SmokeWhaleHome | Out-Null

$SavedEnv = @{
  HOME = $env:HOME
  USERPROFILE = $env:USERPROFILE
  WHALE_HOME = $env:WHALE_HOME
  CODEX_HOME = $env:CODEX_HOME
  OPENAI_API_KEY = $env:OPENAI_API_KEY
  DEEPSEEK_API_KEY = $env:DEEPSEEK_API_KEY
}

$env:HOME = $SmokeHome
$env:USERPROFILE = $SmokeHome
$env:WHALE_HOME = $SmokeWhaleHome
Remove-Item Env:CODEX_HOME -ErrorAction SilentlyContinue
Remove-Item Env:OPENAI_API_KEY -ErrorAction SilentlyContinue
Remove-Item Env:DEEPSEEK_API_KEY -ErrorAction SilentlyContinue

& $WhaleBin --version
& $WhaleBin --help

foreach ($Name in $SavedEnv.Keys) {
  if ($null -eq $SavedEnv[$Name]) {
    Remove-Item "Env:$Name" -ErrorAction SilentlyContinue
  } else {
    Set-Item "Env:$Name" $SavedEnv[$Name]
  }
}
```

Expected:

- `--version` prints `whale 0.0.0`
- `--help` shows `Whale CLI`
- normal help does not expose `app-server`, `cloud`, `--remote`, `--image`,
  or OpenAI-specific login as the default path

Optional help-surface guard:

```powershell
& $WhaleBin --help |
  Select-String -Pattern "codex|--image|cloud|app-server|--remote|chatgpt|openai"
```

Expected: no output for the normal help surface.

## Configure DeepSeek

For the first Windows bring-up, prefer process-scoped secrets:

```powershell
$env:DEEPSEEK_API_KEY = "replace-with-real-key"
$env:WHALE_HOME = "$env:USERPROFILE\.whale"
New-Item -ItemType Directory -Force $env:WHALE_HOME | Out-Null
```

Persist only after the development machine is trusted:

```powershell
[Environment]::SetEnvironmentVariable("DEEPSEEK_API_KEY", "replace-with-real-key", "User")
[Environment]::SetEnvironmentVariable("WHALE_HOME", "$env:USERPROFILE\.whale", "User")
```

Run a live model smoke only when network access and billing are expected:

```powershell
& $WhaleBin exec "Reply with one short sentence."
```

Do not copy `%USERPROFILE%\.codex` into `%USERPROFILE%\.whale`. If you must
move Whale state from another machine, copy only reviewed and sanitized files
from `.whale`.

## Regression Commands

Run targeted tests after the first successful build:

```powershell
cargo test -p codex-api --locked chat_completions
cargo test -p codex-model-provider-info --locked
cargo test -p codex-core --locked defaults_to_deepseek_flash_provider
cargo test -p codex-core --locked responses_websocket_features_do_not_change_wire_api
cargo test -p codex-core --locked config_schema_matches_fixture
```

Full workspace tests can be very large. Run them only after the targeted gates
pass and the machine has enough free disk.

## Failure Recovery

MSVC linker missing:

```powershell
where.exe link
rustup show active-toolchain
```

Fix by installing Visual Studio Build Tools with the C++ workload, then rebuild
from "Developer PowerShell for VS 2022".

Out of disk:

```powershell
$BuildPaths = @(".\target", $env:CARGO_TARGET_DIR) | Where-Object { $_ }
foreach ($Path in $BuildPaths) {
  if (Test-Path $Path) {
    $Bytes = (Get-ChildItem $Path -Recurse -ErrorAction SilentlyContinue |
      Measure-Object -Property Length -Sum).Sum
    "{0}: {1:n2} GB" -f $Path, ($Bytes / 1GB)
  }
}

$env:CARGO_TARGET_DIR = "D:\BuildCache\whalecode\cargo-target"
$env:CARGO_INCREMENTAL = "0"
```

Move regenerable build output to a recoverable backup before deleting anything:

```powershell
$BackupRoot = "D:\BuildCache\whalecode-backups"
New-Item -ItemType Directory -Force $BackupRoot | Out-Null
Move-Item .\target (Join-Path $BackupRoot ("codex-rs-target-" + (Get-Date -Format "yyyyMMddHHmmss"))) -ErrorAction SilentlyContinue
```

Network failure:

```powershell
git ls-remote https://github.com/ceasarXuu/WhaleCode.git HEAD
cargo fetch --locked
```

Retry with the proxy variables above if either command times out.

Path errors:

```powershell
git config --global core.longpaths true
Get-Location
```

Move the checkout to a shorter path such as `D:\dev\WhaleCode` before changing
code.

Antivirus slowdown or file locks:

- keep the checkout outside OneDrive and Desktop
- keep `CARGO_TARGET_DIR` under a dedicated build-cache directory
- if the organization allows it, exclude only the build-cache directory from
  realtime scanning, not the source repository

## Handoff Log

After Windows bring-up, record these outputs in the next migration or runbook
update:

```powershell
git status --short --branch
git rev-parse HEAD
git rev-list --left-right --count "@{u}...HEAD"
rustc --version
cargo --version
& $WhaleBin --version
```

Also record:

- Windows version
- shell used: PowerShell, Developer PowerShell, or `cmd.exe`
- whether `CARGO_TARGET_DIR` was externalized
- which tests passed
- any failing Windows-specific behavior
