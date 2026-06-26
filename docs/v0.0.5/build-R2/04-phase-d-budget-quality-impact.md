# Phase D. Profile Advisory Quality Impact

> 2026-06-25 更新：预算/profile 不再产生硬停；本阶段只保留质量影响与回归检测。
>
> 2026-06-26 复核：Phase A 后续修复没有恢复 hard stop。Phase C 已补齐 producer-owned scan 证据；Phase D 已补齐 quality impact 字段解析和 forbidden budget action release gate；release-like closeout 仍依赖 Phase E/G。

## D.1 目标

成本下降不能通过跳过正确性工作获得。由于 Phase A 已取消 profile 硬上限，Phase D 的职责改为：

- 记录 profile hint 是否被超过。
- 记录旧 blocked 输入是否出现，作为兼容/回归信号。
- 阻断 release-like 声明中的 validation skip、score-ineligible solved、manual override。
- 不再要求或鼓励 provider request blocked、node budget block、spawn budget block、final hard stop。

## D.1.1 当前状态

Status: implemented for Phase D scope; not standalone release-complete.

当前实现已经把 profile overrun 作为 `over_profile_hint` / `observe`
处理，并把 `blocked_by_budget_samples_count` 保留为旧硬停回归计数。后续
Phase D 不应重新引入预算硬停。

Phase D 当前已经完成：

```text
BudgetQualityImpactV1 runtime trace is produced for provider request budget events.
Cost instrumentation preserves active_budget_source, route_mode, budget_state_before,
  budget_state_after, budget_transition_reason, logical_request_id, attempt_seq.
Release decision fails forbidden current budget actions:
  hard_stop, node_budget_block, spawn_budget_block,
  provider_request_budget_exhausted, blocked_by_budget.
```

仍需由后续阶段补齐的依赖：

```text
Phase C: exact payload scan producer-owned proof
Phase E: legacy state action displacement denominator
Phase G: v005-non-agent-gates.json 聚合器和本地证据 hash
```

## D.2 当前语义

允许的 `budget_action`：

```text
observe
legacy_profile_hint_blocked_input
legacy_compact_checkpoint_blocked_input
validation_skip
manual_override
```

禁止作为当前行为产生：

```text
hard_stop
node_budget_block
spawn_budget_block
provider_request_budget_exhausted
blocked_by_budget
```

`blocked_by_budget_samples_count` 保留在汇总中，但含义是“旧硬停回归计数”。Release-like run 必须为 0。

## D.3 Runtime producer

`record_provider_request_budget_events` 对完成态请求记录：

```text
budget_action=observe
provider_request_status=response_completed|response_failed|cancelled
score_eligible=true unless validation evidence says otherwise
final_classification=score_eligible
```

如果读取到历史/兼容 `status=blocked` 输入：

```text
provider_request trace tag: legacy_blocked_input_observed:true
budget_action=legacy_profile_hint_blocked_input
final_classification=legacy_blocked_input_observed
score_eligible=false
```

该兼容路径不得设置 `budget_response_action_taken:true`，否则离线统计会把它误判为当前预算响应动作。

## D.4 Cost instrumentation

`budget_induced_quality_impact_summary.json` 必须输出：

```json
{
  "budget_quality_impact_logged_for_every_budget_action": true,
  "budget_quality_impact_missing_count": 0,
  "budget_induced_validation_skip_count": 0,
  "budget_induced_score_ineligible_solved_count": 0,
  "blocked_by_budget_samples_count": 0,
  "manual_override_used_count": 0
}
```

注意：`budget_action_count` 只统计真正的预算响应动作。当前 advisory-only profile 超出不应增加该计数。

## D.5 Release gate

Release decision 仍然必须失败于：

```text
budget_quality_impact_missing_count > 0
budget_induced_validation_skip_count > 0
budget_induced_score_ineligible_solved_count > 0
blocked_by_budget_samples_count > 0
manual_override_used_count > 0
summary derived counts 与 event 不一致
derived_forbidden_budget_action_count > 0
```

Release decision 不得因为以下情况失败：

```text
request_count > max_rollout_model_requests
node_request_count > max_model_requests_per_node
node_count > max_nodes
spawn_agent_call_count > max_spawn_agent_calls
```

这些只进入 `over_profile_hint` 观测。

## D.6 Tests

Rust:

```text
cargo test -p codex-core budget --lib
cargo test -p codex-core taskspace --lib
```

PowerShell:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
```

关键断言：

```text
profile hint overrun => budget_action=observe
legacy blocked input => legacy_* classification, not hard_stop
blocked_by_budget_samples_count = 0
manual_override mismatch still fails release decision
forbidden budget_action such as hard_stop still fails release decision
```

## D.7 2026-06-26 本地验证

真实 smoke 证据来自：

```text
target\phase-c-real-benefit-proof\single-file-fast-fix-20260626-202548\
single-file-fast-fix\20260626-202549-940\pair-001\right\artifacts
```

该样本不能证明 TaskSpace 业务收益，但能证明 Phase D 没有通过硬停或
跳过验证来伪造成果：

```text
budget_event_count = 40
budget_quality_impact_event_count = 40
budget_action_count = 0
budget_quality_impact_logged_for_every_budget_action = true
budget_quality_impact_missing_count = 0
budget_induced_validation_skip_count = 0
budget_induced_score_ineligible_solved_count = 0
blocked_by_budget_samples_count = 0
manual_override_used_count = 0
spawn-node over_budget_enforcement_status = advisory_only
spawn-node blocked_budget_event_count = 0
```

本阶段门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
  passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
  passed

cargo test -p codex-core budget --lib
  60 passed

cargo test -p codex-core taskspace --lib
  91 passed
```
