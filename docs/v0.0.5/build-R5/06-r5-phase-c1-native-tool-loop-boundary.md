# R5 Phase C1 Native Tool Loop 与 Projection 边界收敛记录

> Phase C1 目标：在 R5-C 薄投影基础上，切到 DeepSeek native tool loop
> 默认路径，保留 TaskSpace 作为不可绕过的 map/state-machine 工具，同时继续删除
> runtime/projection 的语义约束残留。

Version: v0.0.5 build-R5
Phase: R5-C1
Status: implemented, focused sample validation passed
Date: 2026-07-09
Owner: Codex
Related:
  - `docs/v0.0.5/build-R5/05-r5-phase-c-thin-projection-action-sequence.md`
  - `coe/2026-07-09-21-50-r5-native-tool-loop-agent-no-patch.md`

## 1. 本阶段边界

C1 只做三类收敛：

1. DeepSeek 默认 provider transport 回到 native tools；只有显式配置 `action_contract`
   时才走 action-contract。
2. Runtime 可以做语义无关的机械空 map 初始化，并明确告诉 Agent 当前 map/objective/node
   plan 是空白待完善；不注入任务策略。
3. 删除 model-visible 的 action-class contract。Runtime 不再告诉 Agent 某类 node
   允许或不允许 `read/search/edit/test`；只保留 map/node/status/event/ref 和硬状态机状态。

明确不做：

```text
不假设 DeepSeek native tool loop 不稳定。
不因为 Agent 一次做错动作就给 runtime 加语义纠错。
不让 projection 重新组织、压缩、解释任务策略。
不恢复 R4 的 next-valid-actions、coverage、validator rework 等策略提示。
```

## 2. 变更内容

| Area | Change | Boundary |
|---|---|---|
| provider transport | DeepSeek 默认使用 native tools；`action_contract` 仅在显式 env/config 下启用 | transport 选择，不改变 Agent 语义 |
| native alias | `exec_command` 和 `read_file` alias 归一到现有 shell/read 能力 | 工具 ABI 兼容，不新增 model-visible 工具策略 |
| mechanical blank map | 进入 Experiment mode 时如无 active map，创建 task/map/node/lease | 语义无关初始化；objective 明确为 Agent-authored pending |
| runtime gates | ordinary tool preflight 只保留 active map/node/lease 等硬底线 | 不再按 node kind/action class 拒绝 ordinary tool |
| projection | 删除 `hard action-class constraints`、`Current node contract`、`allowed action classes` | 防止 projection 暗示 Agent 不能 edit |
| contracts | 删除 `NodeContract.allowed_actions`，只保留仍被使用的机械 split hint | 去掉语义动作合同 |
| sentinel | 移除 `unclassified_shell_action` model-visible/engineering warning | shell 预览不再被 runtime 语义解释 |

## 3. C1 根因案例

失败样本：

```text
RunDir: target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
PairReport: target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987/pair-001/pair-report.md
standard: solved
taskspace: wrong
failure_taxonomy: agent_no_patch
engineering_unclean: False
```

证据结论：

```text
1. native exec_command/read_file alias 已执行并返回输出，工具链没有 unsupported/block。
2. Agent 已读取 README/tests/source，并明确识别 bug：实现输出 depth: ...，要求是 CALL_STACK_DEPTH=<integer>。
3. active projection 仍显示旧语义合同：
   Current node contract:
   - node: node-1 kind=inspect_code_context
   - allowed action classes: read, search, build, test, control
4. 因 inspect node 的可见 allowed actions 不含 edit，Agent 转去先跑测试/安装 pytest，
   最后被 budget hard stop 放大成 no-patch。
```

根因：

```text
Runtime 执行层已不再阻止 edit，但 projection 仍把旧 action-class contract
作为 model-visible 约束暴露给 Agent。该约束是 R4 语义控制残留，违反 R5 的
语义透传原则。
```

## 4. 修复验证

代码级验证：

```text
cargo fmt --all
cargo test -p codex-core developer_context_uses_active_projection_replacement_after_task_start -- --nocapture
cargo test -p codex-core ordinary_tool_call_is_not_blocked_by_node_action_contract -- --nocapture
cargo test -p codex-core subagent_tool_calls_are_recorded_under_assigned_lease -- --nocapture
cargo test -p codex-core normalizes_exec_command_alias -- --nocapture
cargo test -p codex-core normalizes_read_file_alias -- --nocapture
cargo test -p codex-core exec_command_alias_uses_shell_action_classification -- --nocapture
cargo test -p codex-core trace_event_does_not_parse_shell_preview_as_structured_semantics -- --nocapture
cargo test -p codex-core missing_action_class_for_non_shell_tool_is_not_counted_as_unclassified_shell -- --nocapture
cargo test -p codex-core active_projection -- --nocapture
cargo test -p codex-core taskspace_action_contract -- --nocapture
cargo check -p codex-core
cargo build -p codex-cli --bin whale
```

`cargo fmt --all` 仍只有 stable rustfmt 对 `imports_granularity = Item` 的既有警告。

样本验证：

```text
Command:
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 \
  -Scenario count-call-stack \
  -Repeats 1 \
  -RunSide both \
  -RunRoot target/r5c1-native-tool-loop-no-action-contract \
  -WhaleBin /home/zhangxu/whalecode-alpha/third_party/codex-cli/codex-rs/target/debug/whale \
  -Model deepseek-v4-flash \
  -SandboxMode workspace-write

RunDir:
target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052

PairReport:
target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052/pair-001/pair-report.md
```

结果：

| Side | Outcome | Wall Time | Tool Calls | Validation | Changed Paths |
|---|---|---:|---:|---|---|
| standard | solved | 22802ms | 15 | public=0, hidden=0 | `src/call_stack_counter.py` |
| taskspace | solved | 24664ms | 12 | public=0, hidden=0 | `src/call_stack_counter.py` |

TaskSpace 关键观测：

```text
failure_taxonomy: none
engineering_unclean: False
utility_direction: both_success
taskspace_tool_call_ratio: 0.8
taskspace_wall_time_ratio: 1.08
right rollout_trace.model_request_count: 7
taskspace_projection_count: 7
taskspace_projection_tokens_total: 9249
taskspace_control_count: 0
```

rollout 显示 Agent 在识别 bug 后直接执行 `apply_patch`：

```text
src/call_stack_counter.py:
- return f"depth: {count_stack_depth()}"
+ return f"CALL_STACK_DEPTH={count_stack_depth()}"
```

额外扫描：

```text
rg "allowed action classes|hard action-class constraints|Current node contract|TaskSpaceProviderBudgetHardStopV1|active_sentinel_warning|validator_failure|unclassified_shell_action" \
  target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052/pair-001/right/artifacts/rollout.jsonl \
  target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052/pair-001/right/artifacts/observability/action-map-observability.json \
  target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052/pair-001/right/artifacts/projection-events.jsonl

Result: no matches
```

## 5. 剩余问题

C1 关闭了 native tool-loop 进入后的 correctness blocker，但仍保留以下后续项：

1. `rollout_trace.model_request_count=7`，request count 仍高于理想的 native loop cadence。
   当前不通过 runtime 语义约束解决，后续只从上下文效率、反馈重复和预算口径继续分析。
2. `pytest` 缺失仍会产生普通失败反馈，但本次 Agent 已能在 validator 通过后收束；
   不再因为 runtime/projection 约束陷入环境安装路径。
3. 仍需继续专项审计其他 model-visible 文案里的 `state_machine_allowed_actions` 和
   spawn/validation 相关语义提示，区分硬机制错误与 runtime 语义干预。

## 6. 经验记录

1. 当 Agent 没有做显然应做的 patch，第一优先级仍是检查 provider-visible projection
   是否暗含语义约束。本次就是 projection 残留，而不是 Agent 智能不足或 tool-loop 不稳定。
2. Runtime 执行层去掉 gate 不够；只要 projection 继续显示旧 gate 文案，Agent 仍会被带偏。
3. `NodeKind` 可以作为 map/state 分类，但不能自动变成普通工具动作权限表。
