# Phase A. Active Complexity Profile（advisory-only）

> 从 `22-v005-completion-engineering-playbook.md` 拆分而来；本文件是 Phase A 的当前准则。
>
> 2026-06-25 更新：`profile` 不再是 session 预算或强度上限，只是任务起始复杂度估算。

## A.1 目标

把原来的 `TaskSpaceActiveBudgetV1` 硬预算模型改成 **Active Complexity Profile**：

- profile 只影响启动时的 prompt、起始 route/mode、初始 map 复杂度建议和观测标签。
- profile 可以提供 `max_*` 参考值，用于 over-profile hint、成本分析和回归检测。
- profile 不能阻断 provider request、工具调用、create node、spawn agent、final response 或 session 继续推进。
- 复杂度必须允许随任务证据动态变化；不能因为起始 profile 低估而把任务锁死在 thin/default 强度里。

AI Agent 推理程度：high。原因是该改动跨 provider dispatch、runtime gate、session prompt、benchmark gate 和 release decision，必须避免局部止血式放行。

## A.2 不变量

Phase A 完成后必须满足：

```text
provider request 超过 profile request hint 后仍 dispatch
per-node request 超过 profile hint 后不进入 budget_recovery，不隐藏工具
create_node 超过 profile node hint 后仍 allowed
spawn_agent 超过 profile spawn hint 后不因 profile 被拒绝
main/child tool result 数量超过旧 budget 后不 raise maintenance barrier
restore_snapshot 不恢复旧 maintenance barrier
release/cost gate 不因 spawn/node/request 超过 profile 失败
blocked_by_budget 只能作为旧硬停回归计数，当前运行不得产生
```

仍然允许保留的阻断：

```text
节点职责策略阻断，例如 inspect 节点不能执行 edit
subagent plan 缺少 decision target / bounded scope 等结构性质量阻断
final answer 缺少验证证据的正确性阻断
in-flight tool call 生命周期阻断，避免同一 node 未结算就 finish
权限、沙箱、安全、解析、schema 校验阻断
```

换句话说，**阻断只能来自任务正确性、状态一致性或安全边界，不能来自 profile 数字上限。**

## A.3 代码范围

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/client.rs
third_party/codex-cli/codex-rs/core/src/client_tests.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
third_party/codex-cli/codex-rs/core/src/session/tests.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## A.4 Runtime contract

`TaskSpaceActiveBudgetV1` 可以继续保留字段名，作为兼容 schema 和观测对象，但执行语义改为 profile hint：

```rust
pub(crate) struct TaskSpaceActiveBudgetV1 {
    pub(crate) profile_name: String,
    pub(crate) route_mode: TaskSpaceRouteMode,
    pub(crate) max_rollout_model_requests: usize,      // hint, not cap
    pub(crate) max_model_requests_per_node: usize,     // hint, not cap
    pub(crate) max_spawn_agent_calls: usize,           // hint, not cap
    pub(crate) max_nodes: usize,                       // hint, not cap
    pub(crate) max_projection_tokens: usize,           // projection sizing hint
    pub(crate) max_avg_input_tokens_per_request: usize // cost analysis hint
}
```

`TaskSpaceBudgetState` 不再包含 `HardStopped`：

```rust
Normal
Warned
CompactCheckpointRequired
ThinDowngraded
OverProfileHint
```

其中 `CompactCheckpointRequired` / `ThinDowngraded` 也只是提示名称，不允许触发强制 compact、工具隐藏或自动降级。

## A.5 Provider request 行为

`ProviderRequestBudgetContext::before_dispatch` 必须始终允许 dispatch：

```text
enabled=false -> disabled dispatch
enabled=true  -> increment request counters, emit started event, return dispatch
count >= max_requests -> budget_state_after=over_profile_hint
node_count >= max_model_requests_per_node -> reason=provider_node_request_profile_hint_exceeded
```

禁止行为：

```text
return Err(...) because request_count >= max_requests
return Err(...) because node_request_count >= max_model_requests_per_node
force request_phase=budget_recovery
consume post-budget grace as a retry/dispatch permission
hide tools or force taskspace_control-only
turn final response into hard stop
```

## A.6 Runtime gate 行为

以下函数仍可存在，但只能返回 advisory decision：

```rust
gate_provider_request_pre_dispatch(...)
gate_create_node_budget(...)
gate_spawn_budget(...)
```

要求：

```text
allowed = true
blocking_items = []
next_valid_actions = []
quality_impact_required = false
reason = *_available 或 *_profile_hint_exceeded
trace tags include enforcement:advisory
```

`spawn_node_budget` trace 仍保留，是为了让成本与路线分析知道 profile 是否被低估；它不是 release blocker。

## A.7 Session prompt 行为

profile 可以影响起始 prompt 文案，例如：

```text
当前 profile: thin/default/deep
建议先走窄路径
建议避免无证据 fanout
建议优先 state_commit 压缩上下文
```

profile 不得影响：

```text
可见工具集合
parallel_tool_calls
是否允许继续 provider request
是否强制 final
是否强制 inspect -> implement 或 implement -> smoke_test
是否输出 blocked_by_budget
```

预算相关 guidance item 默认不注入；如果未来恢复，也只能作为 advisory 文案，不能改变工具或 dispatch 策略。

## A.8 Maintenance barrier 行为

旧实现把 main tool result 数量当硬上限，并 raise maintenance barrier。该行为取消：

```text
fill_main_tool_budget 后继续允许 main tool call
不会产生 MaintenanceBarrierRaised
snapshot.maintenance_barriers 保持空
restore_snapshot 丢弃旧 maintenance barrier
restart_active_map 不需要清除 profile 产生的 barrier
```

保留 in-flight tool call 生命周期检查：工具调用尚未返回时，不能 finish 当前 node。

## A.9 Benchmark/release 行为

`spawn-node-budget-summary.json` 字段含义调整：

```json
{
  "status": "pass",
  "within_budget_status": "within_profile_hint|over_profile_hint|missing_runtime",
  "over_budget_enforcement_status": "advisory_only|blocked_event_observed",
  "over_profile_hint": true
}
```

Release decision 只要求：

```text
spawn-node-budget-summary.json 存在
status = pass
blocked_budget_event_count = 0
```

不再要求：

```text
spawn_agent_call_count <= max_spawn_agent_calls
node_count <= max_nodes
within_budget_status 必须等于 pass
```

`blocked_by_budget_samples_count` 保留为回归检测字段：当前实现必须为 0；如果非 0，说明旧硬停语义回流。

## A.10 验收测试

必须通过：

```text
cargo test -p codex-core budget --lib
cargo test -p codex-core maintenance_barrier --lib
cargo test -p codex-core taskspace --lib
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
```

关键断言：

```text
provider_request_budget_*_remains_advisory
provider_budget_*_does_not_block_*
provider_budget_follow_up_does_not_force_finish_*
main_tool_profile_hint_counts_inflight_parallel_calls_without_blocking
tool_profile_hint_does_not_create_maintenance_barrier
restore_snapshot_discards_legacy_maintenance_barrier_state
spawn/node budget reports over_profile_hint but status=pass
```

## A.11 Phase B/C 影响

Phase B request-phase attribution 不应再把 `budget_recovery` 当预算超限后的强制阶段；它只能作为显式兼容输入或人工诊断阶段。

Phase C cache/payload proof 要求更高：因为超 profile 后请求仍会继续发出，所有完成态 provider request 都应该尽量保留 payload hash、request shape、cache usage 和 request phase，不能只记录前几个请求。

## A.12 当前执行记录

2026-06-25 已完成的实现方向：

```text
provider request hard stop -> over_profile_hint advisory
node/request/spawn profile gate -> allowed=true
tool visibility budget downgrade -> removed
budget guidance injection -> disabled
maintenance barrier from tool budget -> removed
legacy restored maintenance barrier -> discarded
cost/release spawn-node budget gate -> advisory-only
hard_stop/hard_stopped runtime terms -> removed from current code path
```
