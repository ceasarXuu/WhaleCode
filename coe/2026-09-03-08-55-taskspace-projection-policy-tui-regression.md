# Problem P-001: TaskSpace projection policy TUI commands disappeared
- Status: repair-validating
- Created: 2026-09-03 08:55
- Updated: 2026-09-03 09:25
- Objective: 在 v0.0.7 开发版恢复 TaskSpace 会话内切换 `map-request`、`map-append`、`map-always` 的 TUI 命令，并保持选择持久化。
- Symptoms:
  - `/taskspace` 可启用 TaskSpace，但当前命令面板和 dispatch 中没有三个 projection policy 命令。
- Expected behavior:
  - TaskSpace 激活后显示并接受 `/map-request`、`/map-append`、`/map-always`，切换立即生效并持久化。
- Actual behavior:
  - 当前 TUI 只提供 `/taskspace` 和 `/task-show`，projection policy 固定落到默认 `map-request`。
- Impact:
  - v0.0.7 dev 用户无法在交互会话中选择 TaskSpace Map 上下文投影策略。
- Reproduction:
  - 启动 `whale-dev`，输入 `/taskspace`，检查 slash command 列表或尝试 `/map-append`。
- Environment:
  - Linux；branch `whalecode-alpha`；HEAD `da3af5f75`；workspace `whalecode-alpha-48d2219088`。
- Known facts:
  - 历史提交 `5ce2cfaf8` 曾实现三个 TUI 命令与持久化，`f881e71b2` vendor rebase 是 `MapRequest` 从 TUI 消失的回归窗口。
  - 当前 protocol/core/app-server 仍定义 `TaskSpaceProjectionPolicy`、`SetTaskSpaceProjectionPolicy` 与 `thread/mapRuntimeMode/set.projectionPolicy`。
- Ruled out:
  - none
- Fix criteria:
  - 回归窗口与断链位置有直接证据；三个命令仅在 TaskSpace active 时可见；每个命令发送正确 policy、持久化配置并保持 TaskSpace active；相关 TUI/core 测试通过；安装后的 `whale-dev` 可完成无模型请求的真实 TUI 切换验证。
- Current conclusion: H-001 已确认并完成局部修复；代码级测试与免费 cache final-wire 门禁均通过，待 clean-commit 安装后的 `whale-dev` PTY 验证后关闭。
- Related hypotheses:
  - H-001
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: vendor rebase 漏回放 TUI policy 控件但保留后端协议
- Status: confirmed
- Parent: P-001
- Claim: `5ce2cfaf8` 引入的 TUI slash command、可见性、dispatch 和配置持久化在后续 vendor cutover 中被覆盖，而 protocol/core/app-server 的 policy enum 与设置路径仍保留，导致用户入口断链。
- Layer: regression-window
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 当前源码可见后端 `SetTaskSpaceProjectionPolicy`，但 TUI `SlashCommand` 不包含对应枚举。
- Falsifiable predictions:
  - If true: 历史提交包含完整 TUI 命令链；当前树缺少该链但仍保留后端设置路径；首次丢失发生在 vendor cutover。
  - If false: 当前树已有等价用户入口，或后端也已删除该策略，或历史命令从未进入主线。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较 `5ce2cfaf8`、后续 vendor cutover 与当前 TUI/backend 路径。
  - Signal: `git show/log/diff` 的命令枚举、可见性、dispatch、app command、持久化 edit 和 app-server protocol 路径。
  - Capture method: 只读 Git 对比和当前源码路径检查；再以当前 TUI 定向测试证明命令集合缺失。
  - Event name or marker:
    - `SetTaskSpaceProjectionPolicy`
    - `PersistTaskSpaceProjectionPolicy`
  - Correlation keys:
    - commit `5ce2cfaf8`
    - vendor cutover `0044ffee5` / `6c2203c5c`
  - Differentiates from:
    - 策略产品能力被整体删除；命令仅被改名；feature flag 隐藏。
  - Supports if:
    - 后端能力存在，历史 TUI 链存在，当前 TUI 链消失且无替代入口。
  - Refutes if:
    - 当前存在可工作的等价命令，或后端已不支持 policy 设置。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: `f881e71b2` 首次移除了 `5ce2cfaf8` 的 TUI `MapRequest/MapAppend/MapAlways` 用户入口；当前后端继续支持三种 policy，因此故障是 vendor replay 的局部 UI 断链。
- Repair design readiness: ready
- Next step: 按当前 app-server/TUI 架构恢复命令链及持久化测试。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Git 回归窗口定位到 0.147 vendor rebase
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `git log -S'MapRequest' -- tui/src/slash_command.rs`；`git show 5ce2cfaf8`
- Prediction or plan link:
  - H-001：历史提交包含命令，而 vendor cutover 首次使其消失。
- Matched signal:
  - `5ce2cfaf8` 增加 `MapRequest`、`MapAppend`、`MapAlways` 及说明；`f881e71b2` 是唯一后续删除 `MapRequest` 的提交。
- Correlation keys:
  - `5ce2cfaf8`
  - `f881e71b2`
- Raw content:
  ```text
  f881e71b2 chore(sync): rebase Whale overlay onto codex 0.147 main
  5ce2cfaf8 feat(taskspace): add persistent projection mode controls
  MapRequest / MapAppend / MapAlways
  ```
- Interpretation: 命令曾进入主线，回归窗口精确落在 vendor rebase，而不是用户记忆或命令改名问题。
- Time: 2026-09-03 08:57

## Evidence E-002: 当前后端支持 policy 但 TUI 无入口
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: 当前 `protocol.rs`、`core/src/session/handlers.rs`、`app-server-protocol/src/protocol/v2/taskspace.rs`、`tui/src/slash_command.rs`
- Prediction or plan link:
  - H-001：后端设置路径保留，TUI 枚举与 dispatch 消失，且没有等价替代入口。
- Matched signal:
  - protocol 包含三值 enum 和 `SetTaskSpaceProjectionPolicy`；app-server mode-set 参数包含 `projection_policy`；TUI 仅含 `TaskSpace`/`TaskShow`，没有三个 policy 命令。
- Correlation keys:
  - `SetTaskSpaceProjectionPolicy`
  - `ThreadMapRuntimeModeSetParams.projection_policy`
- Raw content:
  ```text
  pub enum TaskSpaceProjectionPolicy { MapAlways, MapAppend, MapRequest }
  pub projection_policy: Option<TaskSpaceProjectionPolicy>
  SlashCommand::TaskSpace
  SlashCommand::TaskShow
  ```
- Interpretation: 能力执行层没有删除，故障是 TUI 到现有后端的局部断链；恢复入口足以修复，无需改 TaskSpace runtime。
- Time: 2026-09-03 08:58

## Evidence E-003: 修复后的命令、配置和缓存门禁通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test-result
- Source: `codex-tui`、`codex-config`、`codex-core`、`codex-exec` 定向测试；cache regression gate
- Prediction or plan link:
  - H-001 repair：恢复的 UI 接线应把三种 policy 传给 app-server，持久化合法配置，并保持默认 final-wire 语义。
- Matched signal:
  - 5 个定向测试通过；生成后的 config schema 包含三值枚举；免费 final-wire 门禁通过。
- Correlation keys:
  - `projection_policy_commands_are_visible_only_in_taskspace`
  - `taskspace_projection_commands_set_policy_and_request_persistence`
  - `taskspace_projection_policy_is_persisted`
  - `35c81cf0c68c3ca38665b448b0953ae5ca8472ea32461d5fa5e4b0e6903aff8e`
- Raw content:
  ```text
  codex-tui: 3 passed
  codex-config: 1 passed
  codex-core: 1 passed
  codex-exec: 1 passed
  cache regression gate: PASS（免费 final-wire 验证通过）
  ```
- Interpretation: 修复覆盖用户入口、运行时路由、配置 schema/写回与 exec 读取，同时默认请求投影未发生缓存敏感回归。
- Time: 2026-09-03 09:25
