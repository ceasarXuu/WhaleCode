# R5 Phase C 薄投影与多动作承载收敛记录

> Phase C 目标：把 active projection 收敛为 map/node/event/ref 的薄视图，同时允许
> Agent 在一次 provider response 中明确给出多个动作，减少 TaskSpace 相比 standard 的
> 一小步一请求放大。runtime 仍只执行硬状态机校验、工具调用和事件归档，不替 Agent
> 合并、重排或选择策略。

## 1. 状态

```text
Phase: R5-C
Status: implemented, focused tests passed, live sample validation passed
Updated: 2026-07-09
Primary code:
  third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
  third_party/codex-cli/codex-rs/core/src/action_map/context_compiler.rs
  third_party/codex-cli/codex-rs/core/src/session/turn.rs
  third_party/codex-cli/codex-rs/core/src/session/mod.rs
  scripts/taskspace-benchmark/lib/routing-decision.ps1
COE:
  coe/2026-07-09-05-20-r5-budget-feedback-grace.md
```

## 2. 边界结论

Phase C 的核心边界如下：

```text
允许:
  - Agent 显式输出 action sequence。
  - runtime 逐个执行已给出的 action，并逐个做硬状态机校验。
  - 每个工具结果、状态机拒绝、parser 拒绝都忠实归档为 node-local event 或 runtime feedback。
  - projection 暴露 map skeleton、current node、recent events、result refs、omission audit。

禁止:
  - runtime 根据工具结果替 Agent 选择下一步动作。
  - projection 注入 next action、coverage 推理、validator 策略、path correction 等思考层提示。
  - benchmark routing 决策通过 prompt 进入模型上下文。
  - 用 runtime 语义约束修补 Agent 的低质量操作。
```

## 3. 实现内容

| Area | Change | Boundary |
|---|---|---|
| action sequence | 支持 `taskspace-action-sequence-v1`，一次响应最多 8 个 Agent 明确动作 | 不合并、不重排、不生成动作；遇到拒绝、edit/test 失败、final/blocked 即停止 |
| parser recovery | JSON 起点扫描只接受首字段为 `schema_version` 的 object | 防止 prose 或 f-string `{...}` 抢占解析，不解释语义 |
| patch normalization | unified hunk header 归一成 native apply_patch 可接受的 `@@`，并最小化脆弱上下文 | 只做工具输入语法归一化，不改变目标内容 |
| failure fail-stop | 同一 sequence 中 `apply_patch` 失败或 `run_test` 非零退出后停止后续动作 | 避免失败后继续执行依赖动作；失败反馈仍交给 Agent |
| runtime feedback | action-contract parse/reject 反馈写入当前 node 的 `runtime_feedback` event | 保留失败语义，避免只在 trace 中可见 |
| thin projection | active projection 只渲染 map/node/events/refs/omission audit/budget | 移出 facts/decisions/success criteria/coverage/next-valid-actions 等 active 语义账本 |
| context replacement | active replacement 用结构标记识别 `ContextProjectionV1 active replacement:` | 不再依赖旧 compact marker |
| routing prompt | `TaskShapeRouterV1` 保留 artifact/report，`New-TaskspaceRoutingPrompt` 返回空串 | routing 不再 model-visible 注入策略 |
| bootstrap prompt | bootstrap 改为 thin bootstrap，只声明 start/route 硬入口 | 移除 cognitive preflight/result validity/compact profile 文案 |

## 4. 验收样本

Phase C 选择 `count-call-stack`，因为它同时覆盖：

```text
read evidence -> illegal inspect edit rejection -> legal node transition -> patch -> test feedback -> final external validation
```

最新验收 run：

```text
RunDir: target/r5cphase6/count-call-stack/20260709-183144-389
PairReport: target/r5cphase6/count-call-stack/20260709-183144-389/pair-001/pair-report.md
```

横向结果：

| Baseline | Outcome | Wall Time | Tool Calls | Notes |
|---|---:|---:|---:|---|
| R4-D historical | solved | 154525ms | 11 | `target/r4-d-count-call-stack-dependency-read-20260630/...`，R4 证明该样本可被 TaskSpace 解决 |
| standard current | solved | 15135ms | 10 | `public_validation_exit_code=0`，`hidden_oracle_exit_code=0` |
| R5-C current | solved | 45228ms | 10 | `public_validation_exit_code=0`，`hidden_oracle_exit_code=0` |

R5-C 关键观测：

```text
utility_direction: both_success
failure_taxonomy: none
taskspace_tool_call_ratio: 1
taskspace_wall_time_ratio: 2.99
right rollout_trace.model_request_count: 8
right rollout_trace.input_tokens: 961499
right rollout_trace.cached_input_tokens: 949120
taskspace_projection_count: 8
taskspace_projection_tokens: 10418
taskspace_control_count: 2
agent_messages: 8
agent_actions: 15
multi_action_messages: 4
old routing/compact prompt hits: 0
```

旧策略提示扫描未命中：

```text
TaskShapeRouterV1 active profile constraints: 0
Use the smallest evidence path: 0
read the validator/test contract: 0
active compact profile: 0
cognitive_preflight_requirement: 0
result_validity_requirement: 0
implementation_result_contract: 0
```

## 5. 已通过验证

```text
cargo fmt --all
cargo test -p codex-core taskspace_action_contract -- --nocapture
cargo test -p codex-core active_projection -- --nocapture
cargo test -p codex-core developer_context_is_experiment_only_and_exposes_basemap_without_active_map -- --nocapture
cargo check -p codex-core
cargo build -p codex-cli --bin whale
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-routing-verification-first.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 -Scenario count-call-stack -Repeats 1 -RunSide both -RunRoot target/r5cphase6 ...
```

`cargo fmt --all` 仍输出 stable Rust 对 `imports_granularity = Item` 的既有警告，退出码为 0。

## 6. 残余问题

Phase C 已关闭薄投影和多动作承载的当前 blocker，但还没有解决所有 cadence 成本：

```text
1. R5-C 仍有 8 次 rollout provider request，距离 standard-like native tool loop 还有差距。
2. Agent 在 patch 成功后仍多次尝试环境不可用的测试路径，例如缺少 pytest 或缺少 PYTHONPATH。
3. 最终出现 provider budget hard stop，但发生在 patch 已正确落地且 public/hidden validation 均通过之后。
```

Phase C 后续发现 active projection 仍残留 `hard action-class constraints` /
`allowed action classes`，该残留在 R5-C1 已关闭，见
`docs/v0.0.5/build-R5/06-r5-phase-c1-native-tool-loop-boundary.md`。

这些残余不能通过 runtime 增加语义约束解决。下一阶段应继续优先检查：

```text
context/event/ref 是否高效、忠实、不过度重复；
测试失败反馈是否被清晰保留；
是否需要更接近 native tool loop 的 action carrier，而不是更多 runtime 策略。
```

## 7. 操作记录

本阶段记录两条后续复用经验：

```text
1. cargo test 并行跑多个 filter 时会等待 package/artifact lock；需要可读日志时优先顺序跑。
2. cargo test 只能安全传一个 filter；多个 focused filter 应拆成多条命令或使用共同前缀。
3. benchmark 的 durable 证据优先看 pair-report、right/artifacts/request-summary.json 的 rollout_trace、
   taskspace-control-usage.json、context-projection-summary.json、whale-exec.jsonl。
```
