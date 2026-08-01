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

## Version And Build Number

Whale reuses the Codex CLI release-version flow instead of adding a parallel
version source:

- Release semver lives in
  `third_party/codex-cli/codex-rs/Cargo.toml` under
  `[workspace.package].version`.
- Rust release tags must stay `rust-vX.Y.Z`; the release workflow validates
  that the tag matches the Cargo workspace version.
- npm staging uses the same release semver through
  `scripts/stage_npm_packages.py --release-version`.

Whale adds one checked-in monotonic build number at
`third_party/codex-cli/BUILD_NUMBER`. Increment it when preparing a release
build or handing off a locally installed build for user verification. Keep it a
positive integer, and commit it with the version bump. The TUI embeds it at
compile time and renders startup/status headers as
`vX.Y.Z build N`. GitHub Release display names include the build number, while
artifact names, npm versions, and WinGet versions keep the semver-only Codex
flow.

Run this guard after changing version, build, release workflow, or packaging
files:

```powershell
Set-Location D:\WhaleCode
.\scripts\check-build-profile-policy.ps1
```

## Worktree Branches

Feature work that should be isolated from `main` can use a sibling Git
worktree. The first alpha feature branch was created as:

```powershell
git worktree add -b whalecode-alpha D:\whalecode-alpha
Set-Location D:\whalecode-alpha
git commit --allow-empty -m "chore: initialize whalecode alpha worktree"
git push -u origin whalecode-alpha
```

On Windows sandboxed shells, writes for a sibling worktree may still touch the
main repository metadata under `D:\WhaleCode\.git\worktrees\...`. If a commit
fails while creating `index.lock`, rerun the Git command in an approved host
shell instead of deleting lock files by hand. Pushes can also fail with
`SEC_E_NO_CREDENTIALS` when the sandbox cannot access the normal Windows Git
credential context; rerun the same `git push` from the host shell.

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

## Terminal-Bench Linux 复验前置

R4 接手时在 Linux 主机复验 `organization-json-generator` 暴露了几类前置问题。后续复用同类流程时先检查这些项，避免把 harness 或环境问题误记为 TaskSpace utility 失败：

```bash
git clone --filter=blob:none --sparse --branch dataset/terminal-bench-core/v0.1.x \
  https://github.com/laude-institute/terminal-bench \
  target/external-sources/terminal-bench-core-0.1.1
git -C target/external-sources/terminal-bench-core-0.1.1 sparse-checkout set tasks/organization-json-generator
git -C target/external-sources/terminal-bench-core-0.1.1 rev-parse HEAD
```

期望 commit：

```text
91e10457b5410f16c44364da1a34cb6de8c488a5
```

先跑 plan-only，确认 adapter 和 prompt guard 可用：

```bash
powershell -NoProfile -ExecutionPolicy Bypass \
  -File scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1 \
  -Benchmark terminal-bench \
  -TaskDir target/external-sources/terminal-bench-core-0.1.1/tasks/organization-json-generator \
  -SampleId organization-json-generator \
  -SourceVersion 91e10457b5410f16c44364da1a34cb6de8c488a5 \
  -RunRoot target/r4-org-json-plan-YYYYMMDD \
  -WhaleBin /home/zhangxu/.local/bin/whale \
  -Model deepseek-v4-flash \
  -SandboxMode workspace-write \
  -PlanOnly
```

真实运行前必须确认：

- `DEEPSEEK_API_KEY` 已设置；缺失时 benchmark 会在 `provider_credential_preflight` 阶段以 `provider_credential_missing` fail-fast。
- 凭证 preflight 回归由 `scripts/taskspace-benchmark/test-external-wrapper-harness.ps1` 覆盖；该 harness 会临时清空 `DEEPSEEK_API_KEY` 并验证缺 key 时不会进入 paired execution。
- 若刚提交过 Rust/source 变更，先重建 whale 并刷新二进制 attestation；否则 preflight 会以 `whale_binary_stale_for_codex_source` 或 attestation mismatch fail-fast，不能把它误记为 TaskSpace utility 失败：

```bash
cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
powershell -NoProfile -ExecutionPolicy Bypass \
  -File scripts/taskspace-benchmark/write-whale-binary-attestation.ps1 \
  -WhaleBin third_party/codex-cli/codex-rs/target/debug/whale \
  -BuildCommand "cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked"
```

- 默认健康构建不设置 `CODEX_SKIP_VENDORED_BWRAP`，这样能覆盖 vendored bubblewrap / `codex-linux-sandbox` 编译链路；如果缺 `libcap.pc`，先按 `docs/runbooks/rust-development-environment.md` 补齐 `libcap` 开发依赖。
- 如果 shell 当前目录已经是 `third_party/codex-cli/codex-rs`，可以省略 `--manifest-path`；从 repo 根目录执行时必须带上。
- attestation 刷新后再启动真实 run；如果 preflight 已经报 stale，不要复用该 run root 作为 utility 证据，换新的 run root 重跑。
- Docker build 能访问 Python package 源；`organization-json-generator` 的 validator image 会执行 `pip install jsonschema`。
- Linux native Docker 如果使用宿主 loopback proxy，例如 `127.0.0.1:7890`，generated validator 必须对 build/run 使用 `--network host`，不能只把 proxy 改成 `host.docker.internal`。
- Linux runner 不应依赖 Windows-only primitives：`WindowsIdentity`、`icacls`、`curl.exe`、`cmd.exe`、`subst`、`USERPROFILE` 都必须有跨平台分支或 no-op 记录。
- 从 Bash 用 `pwsh -Command` 批量重算报告时，不要同时依赖 Bash `$name` 与 PowerShell `$args` 的
  隐式转义。把路径放入临时环境变量，再在 PowerShell 中通过 `$env:NAME` 读取，可避免 Bash 提前
  展开 PowerShell 变量。派生报告重算只覆盖 observation/report，不改原始 rollout、wire trace 或
  模型运行证据。
- Skill 的 `quick_validate.py` 依赖 `PyYAML`。系统 Python 缺依赖时不要污染全局环境，使用已有
  `uv` 缓存执行：

```bash
uv run --with pyyaml python \
  /home/zhangxu/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  .agents/skills/observe-taskspace-performance
```

## R4 Tools Feedback 调试内循环（历史）

本节只适用于 R4 历史复盘，不得用于 R7.1 TaskSpace 实现或验收。R7.1 Map Store、终态和 reopen 的现行操作见
[`r7-taskspace-map-store.md`](r7-taskspace-map-store.md)。

R4 tools 链路问题优先按 feedback semantics 分类，不要直接归因为模型策略。常见判断：

- raw tool output 完整但下一轮继续错误动作：优先检查 `failure_kind`、`next_valid_actions`、recent tool feedback 和 active projection。
- action-contract gate 正确拒绝但模型继续同类动作：检查 gate recovery 是否带 repeat state，是否缺少 exact required command。
- inspect 过早进入 implement：检查是否有声明 `fact_sources` artifact 未被 successful read/search 覆盖。
- validation 失败后进入 implement rework：先区分 validation command error、validator infra error、业务断言失败和实现代码失败。
- schema validation 命令若因 `ModuleNotFoundError: No module named 'jsonschema'` 失败，先按 validator dependency recovery 处理，不要直接路由到 implementation rework；在本机 Linux 复验中 `python3` 可能无 `jsonschema`，但默认 `python -m jsonschema -i organization.json schema.json` 可用。
- rework 中同一个 `read_file` 反复成功但 duplicate gate 不触发时，检查 rollout 的 `main_tool_result.artifactRefs`。Linux action-contract `read_file` 会表现为 `sed -n '1,240p' -- path`；该结果必须带 target artifact ref，否则 runtime 无法把“已读 target”传给 `validation_rework_duplicate_artifact_read`。

本地 Rust focused tests 默认使用系统或当前构建的 sandbox 行为。只有确认用例不覆盖 Linux sandbox/bubblewrap 时，才显式跳过 vendored bwrap：

```bash
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt --lib
```

Linux sandbox 变更后至少做两个 smoke：

```bash
cargo test -j1 -p codex-linux-sandbox --lib --locked
cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
```

如果变更涉及 restricted network fallback，还要人工确认：

- fallback 后普通文件写入能完成，例如写入 `target/linux-sandbox-netns-fallback-smoke.txt`。
- restricted network 仍被 seccomp 拦住，socket probe 应返回 `PermissionError: [Errno 1] Operation not permitted` 或等价 EPERM。

长跑 real benchmark 不要只等到 900s timeout。出现以下组合时应先中断并转入 CoE/focused test：

- `TaskSpaceProviderRequestBudgetEventV1` 的 `request_count` 已显著超过 `max_requests`。
- `TaskSpaceNoActionRecoveryV1` 的 recovery attempt 多次增长但 current node / result count 没有实质变化。
- trace 中反复出现同一个 gate reason 或同一个 tool command。

定位 trace 时使用有界读取，避免把巨大 rollout 打进终端：

```bash
rg -n "TaskSpaceForcedInspectTransitionV1|TaskSpaceNoActionRecoveryV1|failure_kind|bwrap:" target/<run-root> -g '*.jsonl' | tail -n 80
```

记录结论时同步更新：

- `/coe/<active-r4-case>.md` 的 Hypothesis/Evidence。
- `docs/v0.0.5/build-R4/01-static-tool-chain-map.md` 的问题类型。
- `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md` 的 focused evidence。
- `docs/v0.0.5/build-R4/09-r4-takeover-progress-audit-20260703.md` 的当前 open items。

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
| App-server CLI/helper | `cargo check -p codex-app-server --bin whale-app-server --locked` | VS Code/app-server protocol behavior changed. |
| Forwarded helper command | `cargo check -p <helper-crate> --bin <helper-binary> --locked` | Local install or npm/release packaging changed. |

Prefer package-level tests before building the full CLI. A full CLI build is a
smoke gate, not the first response to every small Rust edit.

For app-server integration tests in the Whale fork, isolate child processes with
`WHALE_HOME`, not only `CODEX_HOME`. `CODEX_HOME` is kept only as a Codex
compatibility boundary and Whale runtime config loads from `WHALE_HOME`.
If a config RPC test unexpectedly reports `C:\Users\<user>\.whale\config.toml`
as its user layer or writes a value like `model = "gpt-new"` into the real local
config, restore the user config and fix the test harness before trusting the
result.

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
`whale --version` reports the semver only; the monotonic build number is
embedded in the TUI/status version display (`vX.Y.Z build N`). When bumping
`BUILD_NUMBER`, update and run the status snapshot gate so the installed build
number is covered by tests as well as manual smoke.
Do not copy Whale into `%USERPROFILE%\.cargo\bin`, `%USERPROFILE%\.local\bin`,
`%APPDATA%\npm`, or WindowsApps. Those are shared tool locations and can make
Whale appear coupled to official Codex or npm-installed CLIs.

Verify the resolved binary and CLI separation:

```powershell
where.exe whale
where.exe codex
.\scripts\check-cli-isolation.ps1
```

`check-cli-isolation.ps1` intentionally runs both `whale --version` and
`codex --version`. Treat any stderr from either command as a failed smoke test
even if PowerShell reports script exit code `0`. On Windows, a
`thread 'main' has overflowed its stack` message means the freshly built
`whale.exe` itself is unhealthy; verify both
`D:\BuildCache\whalecode\cargo-target\debug\whale.exe --version` and the
installed `%USERPROFILE%\.whale\bin\whale.exe --version` before accepting the
install.

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

Windows cannot overwrite an executable while that exact `whale.exe` is running.
When the active agent process locks `%USERPROFILE%\.whale\bin\whale.exe`, stop
that process and rerun the normal installer:

```powershell
Stop-Process -Id <pid>
.\scripts\install-whale-local.ps1 -PersistUserPath
.\scripts\check-cli-isolation.ps1
```

Do not create a second Whale bin directory for normal local installs. It makes
PATH order and future verification harder to reason about.

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

## App Server Schema 与跨平台回归

修改 `codex-protocol` 中会暴露给 App Server 的类型后，必须在 Codex vendor 根目录刷新生成物，再运行
fixture 测试：

```bash
cd third_party/codex-cli
just write-app-server-schema
cd codex-rs
cargo test -p codex-app-server-protocol --test schema_fixtures --locked
```

不能只刷新 JSON/TypeScript 文件；`schema_fixtures.rs` 中的结构断言也必须同步到现行合同。旧字段断言通过失败
来提示 wire 残留，不应增加兼容字段让测试通过。

默认 Action Map 回归在 Linux/Docker 中运行：

```bash
pwsh scripts/run-action-map-regression.ps1
```

Windows Application Event Log 只在 Windows 且 `Get-WinEvent` 可用时采集；Linux/Docker 返回空 crash-event
集合。每个 Cargo filter 必须至少命中一个测试，零命中不能算通过。

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
