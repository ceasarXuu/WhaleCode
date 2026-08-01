# Phase F. Route-aware Spawn/Node Profile Observability

> 2026-06-25 更新：route/profile 不再限制 spawn/node 数量；只提供起始复杂度估算和 over-profile 观测。
>
> 2026-06-26 执行：artifact/release 已显式汇总 subagent review debt；release gate 会阻断未审查 subagent result，但不会因为 over-profile hint 阻断。

## F.1 目标

Fanout 质量仍然要被治理，但不能用 profile 数字硬砍：

- `max_spawn_agent_calls`、`max_nodes`、`max_open_leaf_nodes` 是 profile hint。
- 超过 hint 时记录 `over_profile_hint`，不阻断 create/spawn。
- Spawn 是否合理由 subagent plan 的决策目标、证据边界、结果采纳路径决定。
- Thin/default/deep 只影响起始 prompt 和 map 复杂度建议，不能锁死整个 session 强度。

## F.2 仍然允许的质量门

这些不是预算硬上限，可以继续阻断：

```text
spawn 前没有 active TaskSpace route/binding
spawn 没有明确 node_id 或无法 claim 对应 ready node
record_subagent_plan 缺少 why_parallelizable / expected_artifact / acceptance_check / max_scope
subagent plan 没有 decision target
subagent result 长期未 adopt/reject/defer，导致证据债务未清
同一 running node 已被 main lease 持有，不能被子 agent 抢占
```

这些可以确保 fanout 有证据收益，而不是用预算上限替代架构判断。

当前 runtime 已实现或强化的质量门：

```text
spawn_agent 必须 claim ready node
ready node 必须先有 unused record_subagent_plan
record_subagent_plan 必须包含 bounded scope 和 decision yield reference
completed narrow inspect 后，不允许再为单条 follow-up inspect 滥用 subagent
unreviewed result 会阻断 ordinary work 或 downstream spawn，直到 mark_result_validity 明确 accept/question/reject
subagent 完成 inspect/validation 类 node 前必须有对应工具或问题状态证据
```

Phase F 已完成的产物：

```text
spawn-node-budget-summary 继续保持 advisory-only 语义
release artifact 显式汇总 unreviewed_subagent_result_count / review debt
release gate 阻断未审查 subagent result
post-ABI B-tier smoke 需要证明这些质量门没有再次制造业务失败
```

## F.3 禁止的行为

```text
thin route 因 max_spawn_agent_calls=0 直接拒绝 spawn
spawn_agent_call_count >= max_spawn_agent_calls 后拒绝 spawn
node_count >= max_nodes 后拒绝 create_node
open_leaf_node_count >= max_open_leaf_nodes 后拒绝 create_node
release decision 因 spawn/node count 超 profile 失败
```

## F.4 Runtime trace

`spawn_node_budget` trace 继续保留，但语义是 profile advisory：

```text
kind = spawn_node_budget
producer = runtime
enforcement = advisory
status = allowed
budget_kind = spawn|node
budget_gate_reason = spawn_budget_available|spawn_profile_hint_exceeded|node_budget_available|node_profile_hint_exceeded
```

如果出现 `status=blocked`，应视为硬预算回归，release gate 应失败。

## F.5 Cost artifact

`spawn-node-budget-summary.json`：

```json
{
  "schema_version": "taskspace-spawn-node-budget-summary-v1",
  "status": "pass",
  "within_budget_status": "within_profile_hint|over_profile_hint|missing_runtime",
  "over_budget_enforcement_status": "advisory_only|blocked_event_observed",
  "spawn_agent_call_count": 3,
  "max_spawn_agent_calls": 0,
  "node_count": 5,
  "max_nodes": 4,
  "over_profile_hint": true,
  "blocked_budget_event_count": 0,
  "subagent_review_debt_status": "no_unreviewed_subagent_results|unreviewed_subagent_results|not_measured",
  "subagent_result_count": 1,
  "reviewed_subagent_result_count": 1,
  "unreviewed_subagent_result_count": 0
}
```

`status=pass` 的条件：

```text
runtime event 存在
blocked_budget_event_count = 0
unreviewed_subagent_result_count = 0
```

不是：

```text
spawn_agent_call_count <= max_spawn_agent_calls
node_count <= max_nodes
```

## F.6 Release gate

Release decision 的 `spawn_node_budget_gate_pass` 表示：

```text
profile advisory trace 存在
没有 profile 产生的 blocked event
subagent_review_debt_status = no_unreviewed_subagent_results
unreviewed_subagent_result_count = 0
```

不表示：

```text
运行被限制在起始 route/profile 的 max_* 内
```

## F.7 Tests

Rust：

```text
cargo test -p codex-core budget --lib
cargo test -p codex-core taskspace --lib
```

PowerShell：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
```

关键断言：

```text
create_node over profile => allowed + node_profile_hint_exceeded
spawn over profile => allowed + spawn_profile_hint_exceeded
spawn-node summary over_profile_hint => status=pass
release decision 不要求 within_budget_status=pass
release decision 会阻断 unreviewed_subagent_result_count > 0
```

## F.8 本地收益证明

Phase F 的真实收益不是“减少次数”，而是把 fanout 治理从硬预算迁移到可验证质量债：

```text
正例 artifact:
  target/cost-instrumentation-selftest/artifacts/spawn-node-budget-summary.json
  status = pass
  within_budget_status = over_profile_hint
  over_budget_enforcement_status = advisory_only
  over_profile_hint = true
  blocked_budget_event_count = 0
  subagent_review_debt_status = no_unreviewed_subagent_results
  subagent_result_count = 1
  reviewed_subagent_result_count = 1
  unreviewed_subagent_result_count = 0

负例 artifact:
  target/release-decision-selftest/run-20260626-214802-116/unreviewed-subagent-result/release-decision.json
  decision = fail
  closeable = false
  spawn_node_budget_gate_pass = false
  subagent_review_debt_status = unreviewed_subagent_results
  unreviewed_subagent_result_count = 1
  blockers includes spawn_budget_gate_failed
```

已通过门禁：

```text
cargo test -p codex-core budget --lib
cargo test -p codex-core taskspace --lib
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
```
