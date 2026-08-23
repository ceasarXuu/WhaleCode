# Problem P-001: v0.0.6 多 Provider 发布被仓库基线失败阻断
- Status: investigating
- Created: 2026-08-24 04:32 +0800
- Updated: 2026-08-24 04:51 +0800
- Objective: 在不改写已确认产品逻辑的前提下，找到并修复阻断 v0.0.6 multi-provider 发布门禁的仓库基线根因。
- Symptoms:
  - 受影响六 crate 的隔离 nextest 矩阵执行 9284 项，8928 通过、356 失败。
  - `just fmt-check` 和 `just clippy -p codex-tui` 也被 multi-provider 改动外的问题阻断。
- Expected behavior:
  - multi-provider 相关实现与仓库发布门禁一致，六 crate 隔离矩阵、fmt 和 Clippy 可通过。
- Actual behavior:
  - 失败分布于 core 257、app-server 52、TUI 46、protocol 1，涉及 code mode、MCP、plugins、Guardian、status、pets 与 provider/model 兼容测试。
- Impact:
  - v0.0.6 multi-provider 功能定向测试通过，但无法证明 release-ready。
- Reproduction:
  - `python3 scripts/codex-upstream/run_isolated_tests.py -p codex-login -p codex-models-manager -p codex-protocol -p codex-core -p codex-app-server -p codex-tui`
- Environment:
  - Ubuntu 24.04 x86_64；branch `whalecode-alpha`；commit `09d8d4fa1`；Rust stable 1.95 工具链；Asia/Shanghai。
- Known facts:
  - login 197/197 和 models-manager 54/54 通过。
  - 失败集合的 72.2% 在 core，其中 code mode 80 项为最大单一簇。
  - 至少两个 provider/model 名称相关失败使用默认 DeepSeek 配置，但断言 OpenAI 目录或 capability。
- Ruled out:
  - none
- Fix criteria:
  - 确认的根因与修复一一对应；原始失败簇的定向复现通过；六 crate 隔离矩阵、`just fmt-check`、相关 Clippy 和 cache gate 通过；无未授权真实模型请求。
- Current conclusion: 失败明显不是 356 个独立问题，但共享根因的数量和边界尚未通过证据门禁。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: legacy 模型目录接口对 OpenAI 调用错误应用 DeepSeek-only 过滤
- Status: confirmed
- Parent: P-001
- Claim: 一批 model、capability、code-mode 和 Guardian 失败由 OpenAI fixture/兼容调用仍使用 legacy `build_available_models`，而该接口默认只保留 `deepseek-*` 引起。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Whale 产品默认 Provider 已是 DeepSeek，而上游大量测试默认使用 OpenAI 能力；legacy `ModelsManager` 无 route 参数，却默认执行 DeepSeek-only 过滤。
- Falsifiable predictions:
  - If true: 临时关闭 legacy DeepSeek-only 过滤后，OpenAI 代表测试恢复，且不需要改 capability 或 tool 实现。
  - If false: 关闭过滤不改变代表测试失败信号。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败由 effective provider 与断言 provider 不同导致。
  - Signal: 定向测试的 config/provider 事实、失败断言和临时关闭过滤的对照实验。
  - Capture method: 读取 fixture 与 manager 构建路径，做可立即回滚的单点诊断改动。
  - Event name or marker:
    - none
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-003
  - Supports if:
    - 临时关闭 legacy 过滤单独恢复代表性失败断言。
  - Refutes if:
    - 临时关闭 legacy 过滤不改变失败信号。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-005
  - E-006
- Conclusion: 关闭 legacy DeepSeek-only 过滤可单独恢复模型切换失败，且撤销 provider ID 同步后仍通过；该机制已确认，但不解释 Code Mode 与 Guardian。
- Repair design readiness: implemented and verified
- Next step: none for H-001
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 隔离环境缺失资产或工具导致批量基础设施失败
- Status: confirmed
- Parent: P-001
- Claim: MCP、plugins、pets、fmt 和部分 app-server 失败由隔离 runner 未提供测试需要的资产、可执行文件、配置或环境变量引起。
- Layer: environment
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - `dotslash` 明确缺失，且 MCP/plugin/pets 失败具有资产与子进程依赖特征。
- Falsifiable predictions:
  - If true: 失败输出会聚类为 missing executable/file/config/server metadata，补齐隔离环境后多项同时恢复。
  - If false: 失败在资产完整的定向宿主测试中以同样业务断言稳定复现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败信号是环境/资产缺失而非业务状态错误。
  - Signal: JUnit failure message、定向宿主复现、隔离 runner 复制清单与运行环境。
  - Capture method: 抽取 MCP、plugin、pets 各一个失败的 raw failure，对照宿主定向测试。
  - Event name or marker:
    - none
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-004
  - Supports if:
    - raw failure 指向共享缺失资产/工具且宿主对照改变结果。
  - Refutes if:
    - 宿主与隔离结果一致且断言为稳定业务语义差异。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Related evidence:
  - E-001
  - E-004
  - E-007
  - E-008
- Evidence gate: satisfied
- Conclusion: Code Mode 与 MCP 代表失败分别缺少 `codex-code-mode-host` 和 `test_stdio_server`；前者当前还受 rusty_v8 上游缺失预构建资产阻断。
- Repair design readiness: MCP helper repair implemented and verified; code-mode host dependency remains externally blocked
- Next step: 将 code-mode host 404 作为独立上游依赖阻断，不混入产品回归；继续盘点其他失败簇。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: multi-provider 路由改动破坏了显式 route 的运行时行为
- Status: unverified
- Parent: P-001
- Claim: 至少一部分失败由 route-bound manager、capability 或 transition 实现在显式 route 下返回错误运行时导致。
- Layer: regression-window
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 失败集合包含 model switching、models cache 和 provider capabilities 名称相关测试。
- Falsifiable predictions:
  - If true: 显式 OpenAI/DeepSeek route 的新 contract test 或最小运行时复现也会失败。
  - If false: 新 route-bound contract test 均通过，失败只发生在未迁移的 legacy fixture/caller。
- Diagnostic evidence plan:
  - Prediction or clause under test: 显式 route 行为本身是否失败。
  - Signal: route-bound auth/catalog/transition/history/capability 定向测试结果与失败 caller 的 route 使用方式。
  - Capture method: 重跑最小显式 route 套件，并检查相关失败是否调用 legacy accessor。
  - Event name or marker:
    - none
  - Correlation keys:
    - route and test name
  - Differentiates from:
    - H-001
  - Supports if:
    - 显式 route 套件可复现同一错误。
  - Refutes if:
    - 显式 route 套件通过且失败 caller 未携带 route。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-002
  - E-003
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 复验 route-bound 套件并对照 legacy accessor 失败。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: 快照与断言未跟随 Whale 产品基线演进
- Status: unverified
- Parent: P-001
- Claim: TUI status/Guardian/exec/feedback 以及部分 core 失败是预期文本、模型目录、品牌或布局快照落后于已接受产品行为。
- Layer: regression-window
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - multi-provider 收口时已发现 model popup 快照缺少既有 vision model，更新预期后单测通过。
- Falsifiable predictions:
  - If true: `.snap.new` 与当前渲染的差异将与已接受的模型/品牌/布局变化一致，不显示数据丢失或错误状态。
  - If false: 快照差异暴露未预期的交互、状态或信息丢失。
- Diagnostic evidence plan:
  - Prediction or clause under test: 快照差异是预期演进还是真实行为回归。
  - Signal: 代表性 snapshot diff 及引入变化的 commit/code path。
  - Capture method: 选 status、Guardian、exec/feedback 各一项定向运行并审查 `.snap.new`。
  - Event name or marker:
    - none
  - Correlation keys:
    - snapshot name
  - Differentiates from:
    - H-002
  - Supports if:
    - 差异可与已接受的实现变更直接对应。
  - Refutes if:
    - 差异含不可解释的状态丢失或错误行为。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - remove `.snap.new` after diagnosis
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 定向生成代表性 snapshot diff。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 失败按 crate 与模块聚类
- Related hypotheses:
  - H-002
  - H-004
- Direction: supports
- Type: test
- Source: `third_party/codex-cli/codex-rs/target/nextest/local/junit.xml`
- Prediction or plan link:
  - H-002/H-004 的失败应聚类而非均匀分布预测。
- Matched signal:
  - core 257、app-server 52、TUI 46、protocol 1；core code_mode 80、tools 26、RMCP 24；app-server plugin list/share 24；TUI status 14、Guardian 8。
- Correlation keys:
  - nextest UUID `2c80ad31-b935-4c0a-9ea8-bae8f614a1a8`
- Raw content:
  ```text
  tests=9284 failures=356 time=244.568
  codex-core failures=55; codex-core::all failures=202
  codex-app-server::all failures=52
  codex-tui failures=46
  codex-protocol failures=1
  ```
- Interpretation: 少数模块占据大多数失败，值得优先查找共享 fixture、环境或快照根因。
- Time: 2026-08-24 04:32 +0800

## Evidence E-002: 两个 provider 相关失败的 effective default 为 DeepSeek
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source: 定向 `cargo test` 与对应 fixture/code path
- Prediction or plan link:
  - H-001 的 provider 前提与断言 provider 不同预测。
- Matched signal:
  - model switching 测试在 legacy/default manager 中找不到注入的 OpenAI 模型；capability read 实际返回 DeepSeek false/false/true，测试期待 OpenAI true/true/true。
- Correlation keys:
  - `model_switch_to_smaller_model_updates_token_context_window`
  - `read_default_provider_capabilities`
- Raw content:
  ```text
  expected test-text-only-model to be available in remote model list
  actual capabilities: namespace_tools=false, image_generation=false, web_search=true
  expected capabilities: namespace_tools=true, image_generation=true, web_search=true
  ```
- Interpretation: 证明了存在 legacy fixture/provider 前提冲突；尚未证明该机制能解释多少失败。
- Time: 2026-08-24 04:32 +0800

## Evidence E-003: multi-provider 显式 route 定向套件通过
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: test
- Source: Phase 1–4 定向测试记录与 `docs/releases/v0.0.6/multi-provider/plan.md`
- Prediction or plan link:
  - H-003 的显式 route 行为也会失败预测。
- Matched signal:
  - route-bound auth/catalog/cache/transition/history/lifecycle/TUI selection/login recovery 定向套件均通过。
- Correlation keys:
  - `openai/chatgpt`
  - `openai/api-key`
  - `deepseek/api-key`
- Raw content:
  ```text
  codex-login 197/197 passed
  codex-models-manager 54/54 passed
  provider transition, history projection, lifecycle and TUI routed selection focused tests passed
  ```
- Interpretation: 降低了显式 route 实现普遍回归的可能性，但需要在当前 HEAD 复验关键套件才能关闭 H-003。
- Time: 2026-08-24 04:32 +0800

## Evidence E-004: fmt 与 Clippy 阻断来自环境和未触及模块
- Related hypotheses:
  - H-002
- Direction: supports
- Type: environment
- Source: `just fmt-check`、`cargo fmt --all -- --check`、`just clippy -p codex-tui`
- Prediction or plan link:
  - H-002 的工具/环境缺失会独立阻断门禁预测。
- Matched signal:
  - `cargo fmt --all -- --check` 通过；`just fmt-check` 因 `dotslash` 不存在且 stable rustfmt 不支持 nightly import granularity 而失败；Clippy 在未触及的 codex-state `expect_used` 失败。
- Correlation keys:
  - none
- Raw content:
  ```text
  [Errno 2] No such file or directory: 'dotslash'
  error: used expect() on an Option value
  state/src/runtime/taskspace_action_settlements.rs:35:52
  ```
- Interpretation: 至少静态门禁不能作为 multi-provider 生产逻辑回归证据；工具链和独立 lint 需分开修复。
- Time: 2026-08-24 04:32 +0800

## Evidence E-005: 同步 fixture provider ID 未改变三项代表失败
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: diagnostic
- Source: `core/tests/common/test_codex.rs` 单行诊断改动与三项定向 `cargo test`
- Prediction or plan link:
  - 原 H-001 中“仅 provider ID 未同步”的预测。
- Matched signal:
  - fixture 将 `model_provider_id` 同步为 `openai` 后，Code Mode 仍只发出一次请求、模型切换仍找不到远端模型、Guardian 仍收不到 assessment。
- Correlation keys:
  - `code_mode_can_return_exec_command_output`
  - `model_switch_to_smaller_model_updates_token_context_window`
  - `guardian_review_uses_preferred_review_model_without_model_catalog_override`
- Raw content:
  ```text
  expected two output items, got one
  expected test-text-only-model to be available in remote model list
  expected guardian assessment
  ```
- Interpretation: provider ID 不一致不是充分根因；代码检查显示 legacy manager 即使由 OpenAI provider 创建，仍默认执行 DeepSeek-only 目录过滤，需单独验证该机制。
- Time: 2026-08-24 04:51 +0800

## Evidence E-006: 关闭 legacy DeepSeek-only 过滤单独恢复模型切换
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic
- Source: `models-manager/src/manager.rs` 单行可回滚诊断改动与定向 nextest
- Prediction or plan link:
  - H-001 的过滤机制预测。
- Matched signal:
  - 临时令 legacy `restrict_to_whale_models` 返回 false 后，`model_switch_to_smaller_model_updates_token_context_window` 从稳定失败转为通过；撤销 fixture provider ID 同步后仍通过。Code Mode 与 Guardian 仍失败，划清了机制边界。
- Correlation keys:
  - nextest run `1860845b-48c4-4634-952a-8eba6d0dbba9`
  - nextest run `406ca047-1334-49f4-8a8f-d31e97379711`
- Raw content:
  ```text
  PASS suite::model_switching::model_switch_to_smaller_model_updates_token_context_window
  FAIL suite::code_mode::code_mode_can_return_exec_command_output
  FAIL guardian::tests::guardian_review_uses_preferred_review_model_without_model_catalog_override
  ```
- Interpretation: H-001 已满足证据门禁；修复应只让 OpenAI legacy manager 不过滤，不能全局关闭 DeepSeek-only 视图。
- Time: 2026-08-24 04:55 +0800

## Evidence E-007: Code Mode 与 MCP 失败缺少运行时 helper
- Related hypotheses:
  - H-002
- Direction: supports
- Type: environment
- Source: 定向测试、helper 查找路径、`cargo build` 与 rusty_v8 官方发布资产
- Prediction or plan link:
  - H-002 的失败由缺失 executable 而非业务状态引起预测。
- Matched signal:
  - Code Mode 原始响应为 `unsupported custom tool call: exec`；`effective_tool_mode` 在 host 不可用时退回 Direct。MCP raw failure明确找不到 `test_stdio_server`。两者均不在六 crate 选择集的构建产物中。
- Correlation keys:
  - `code_mode_can_return_exec_command_output`
  - `test_stdio_server`
  - rusty_v8 `v150.4.0`
- Raw content:
  ```text
  unsupported custom tool call: exec
  could not locate binary "test_stdio_server"
  Failed to download .../v150.4.0/librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz (HTTP 404)
  ```
- Interpretation: H-002 已满足证据门禁。Code Mode 不是 provider 行为回归；当前 host 构建还命中 OpenAI Codex 上游已报告的 rusty_v8 sandbox asset 404，不能用修改业务断言掩盖。
- Time: 2026-08-24 05:13 +0800

## Evidence E-008: 隔离 runner 预构建 MCP helper 后代表测试通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: verification
- Source: `scripts/codex-upstream/run_isolated_tests.py` 与定向隔离 nextest
- Prediction or plan link:
  - H-002 的补齐 helper 后原失败恢复预测。
- Matched signal:
  - runner 在包含 `codex-core` 的选择范围内先构建 `test_stdio_server`；脚本单测 7/7 通过，原 MCP 代表测试在隔离环境中通过。
- Correlation keys:
  - nextest run `e35e1c48-ffaa-458c-b87f-afae89e06110`
- Raw content:
  ```text
  Ran 7 tests ... OK
  PASS suite::mcp_refresh_cleanup::refresh_keeps_superseded_mcp_server_alive_for_in_flight_calls
  ```
- Interpretation: MCP helper 缺失的 runner 工程缺口已修复；Code Mode host 仍需等待或绕开上游 rusty_v8 资产问题。
- Time: 2026-08-24 05:25 +0800
