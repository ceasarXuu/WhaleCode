# R5 Phase C0 执行节奏与预算账本收敛记录

> Phase C0 目标：先修复 TaskSpace 相比 standard 明显的 request cadence 断点，保证
> 状态机/map 生命周期不会在可执行节点首轮或工具失败反馈回传前被预算账本截断。

## 1. 状态

```text
Phase: R5-C0
Status: implemented, live sample validation passed
Updated: 2026-07-09
Primary code:
  third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
  third_party/codex-cli/codex-rs/core/src/session/turn.rs
COE:
  coe/2026-07-09-05-20-r5-budget-feedback-grace.md
```

Phase C0 不改变 Agent 的动作选择，不自动修正 malformed patch，也不向 projection
加入新的策略提示。修复范围限定在 provider request budget accounting：runtime 只保证
状态机/map 工具链的硬底线和反馈交付窗口。

## 2. 根因

`count-call-stack` 暴露了两个连续 budget lifecycle cliff：

1. Agent 已完成 inspect -> implement 状态转移，但全局 request budget 已到 profile hint，
   implement 节点 `node_request_count=0/2` 时被 pre-dispatch hard stop。
2. 放开 implement 首轮后，该首轮请求仍被标记为 `budget_recovery`，并消耗唯一的
   post-budget feedback grace；随后 `apply_patch verification failed` 的工具失败反馈没有
   下一轮模型请求窗口。

这两个问题都属于预算账本边界，不属于 projection 语义加工问题：

```text
错误方向:
  让 projection 重写 patch 语法、提示 Agent 必须怎么修、或让 runtime 代替 Agent 选择动作。

正确方向:
  忠实保留工具失败原文和 node-local event；
  只修复预算账本，避免状态机生命周期和反馈交付窗口互相吞掉。
```

## 3. 实现内容

| Area | Change | Boundary |
|---|---|---|
| provider pre-dispatch gate | 新增 fresh executable node first-request grace | 只允许 implement/smoke/regression/final 新节点首轮请求通过 hard stop |
| post-budget grace accounting | fresh executable node 首轮请求不计入 post-budget feedback grace | 后续同节点 feedback request 仍受唯一 grace 硬限制 |
| trace reconstruction | 从 trace 重建 budget counters 时使用同一 fresh-node 规则 | 避免 replay 后重新把首轮请求算错 |
| observability | provider budget trace 增加 `fresh_node_first_request_grace:true` 和 `post_budget_grace_counted:false` | 仅做账本解释，不影响 Agent 决策 |
| action-contract patch normalization | 剥离缺失 Begin 但带尾部 `*** End Patch` 的 unified diff 外壳残片 | 纯语法归一化，不替 Agent 选择 patch 内容 |

## 4. 验收标准

必须满足：

```text
1. implement_solution / smoke_test / regression_test / final_synthesis 新节点首轮请求不再被全局 rollout profile hint 截断。
2. 上述首轮请求不消耗工具失败反馈的 post-budget grace。
3. 同一节点后续 budget_recovery 仍会计入 post-budget grace，资源底线不被无限放开。
4. 不新增 projection 语义重写、策略提示或 patch 自动纠错。
```

已通过的 focused tests：

```text
cargo fmt --all
cargo test -p codex-core taskspace_active_budget_allows_fresh_executable_node_first_request -- --nocapture
cargo test -p codex-core post_budget_grace_counter_ignores_fresh_executable_node_first_request -- --nocapture
cargo test -p codex-core taskspace_active_budget_allows_one_budget_recovery_grace_request -- --nocapture
cargo test -p codex-core taskspace_action_contract_normalizes_unified_diff_with_trailing_end_only -- --nocapture
cargo test -p codex-core taskspace_action_contract_apply_patch_normalizes_unified_diff -- --nocapture
cargo check -p codex-core
cargo build -p codex-cli --bin whale
```

`cargo fmt --all` 仍输出既有 stable Rust 警告：
`can't set imports_granularity = Item`。该警告不影响格式化退出码。

## 5. 样本计划

按 R5 规则，Phase C0 使用 `count-call-stack` 单样本做横向复验：

```text
Scenario: count-call-stack
Repeats: 1
Sides: standard + current R5
Model: deepseek-v4-flash
Expected C0 signal:
  R5 不再停在 implement 首轮或 apply_patch 失败反馈后的同一 hard stop。
  若仍失败，失败点必须推进到 Agent 基于工具反馈后的下一步决策或后续状态机硬规则。
```

复验结果：

| Run | Standard | R5 Current | 结论 |
|---|---:|---:|---|
| `target/r5c0runs2/count-call-stack/20260709-070112-493` | solved | wrong | C0 预算窗口推进成功；失败点变为 action-contract patch trailing-only End 归一化缺陷 |
| `target/r5c0runs3/count-call-stack/20260709-070533-898` | solved | solved | C0 live gate 通过；R5 right `business_success=True`、public/hidden oracle 均 0 |

关键指标：

```text
target/r5c0runs3/count-call-stack/20260709-070533-898/pair-001/pair-report.md
standard:
  wall_time_ms: 29287
  tool_call_count: 11
  changed_paths: src/call_stack_counter.py
taskspace:
  wall_time_ms: 77214
  tool_call_count: 6
  changed_paths: src/call_stack_counter.py
  maps: 1
  nodes: 2
  edges: 1
utility_direction: both_success
included_in_utility_aggregate: False (Repeats=1 diagnostic)
```

残余观察：

```text
R5 right 仍在 request_count=7/6、node_request_count=2/2 后出现 provider budget hard stop。
该 hard stop 发生在 patch 和测试动作之后，benchmark public/hidden validation 已通过。
它不是本 case 的失败点，但说明 Phase C 仍需继续处理 action-contract 一步一请求的结构性成本。
```

操作注意：

```text
RunRoot 不能包含 benchmark harness 判定为非中性的词，例如 standard、taskspace、action-map、map、node、subagent。
本阶段使用类似 target/r5c0runs2 的中性目录，避免 `fresh-node` 这类名字触发 cwd neutrality failure。
```

## 6. 工程收益

Phase C0 的收益不是提升某个样本分数，而是明确 budget 层边界：

```text
TaskSpace 的状态机生命周期可以跨 node 正常推进。
工具失败反馈获得一次忠实回传窗口。
runtime 不承担语义纠错、不替 Agent 决定下一步。
后续 R5-C thin projection 可以专注于上下文透传和结构瘦身，不需要继续背 budget cliff。
```
