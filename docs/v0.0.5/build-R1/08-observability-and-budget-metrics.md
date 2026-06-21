# 08. Observability and Budget Metrics

## 1. 背景

v0.0.4 的成本根因之所以能确认，是因为本机分析补齐了 rollout/token-count/request proxy 的拆解。但这些指标还不是一等公民。

v0.0.5 必须把成本指标纳入 pair/sample/suite artifact，否则无法判断 2x 目标是否达成。

## 2. 新 artifact

### `token-summary.json`

每个 side 输出：

```json
{
  "mode": "taskspace",
  "input_tokens": 50780045,
  "cached_input_tokens": 50119296,
  "uncached_input_tokens": 660749,
  "output_tokens": 293242,
  "reasoning_output_tokens": 82420,
  "model_request_count": 1229,
  "avg_input_per_request": 41318,
  "max_input_per_request": 104989,
  "avg_output_per_request": 238,
  "taskspace_control_count": 850,
  "state_commit_count": 0,
  "largest_tool_output_bytes": 169047,
  "large_output_replay_count": 1
}
```

### `context-projection-summary.json`

```json
{
  "projection_count": 44,
  "avg_projection_tokens": 6200,
  "p95_projection_tokens": 8800,
  "active_item_count_avg": 14,
  "archived_item_count": 53,
  "audit_only_item_count": 20,
  "history_shadow_elidable_tokens": 120000,
  "context_replacement_potential": 2.4
}
```

### `state-management-summary.json`

```json
{
  "state_commit_count": 8,
  "legacy_action_count": 12,
  "auto_bookkeeping_events": 31,
  "gc_events": 9,
  "compaction_events": 7,
  "result_lifecycle": {
    "new": 3,
    "accepted_adopted": 8,
    "accepted_retained": 4,
    "rejected": 12,
    "deferred": 2,
    "archived": 20,
    "audit_only": 6
  }
}
```

## 3. Pair-level ratio report

每个 pair 输出：

```text
TaskSpace / Standard agent walltime
TaskSpace / Standard direct input+output tokens
TaskSpace / Standard uncached input
TaskSpace / Standard output tokens
TaskSpace / Standard model request count
TaskSpace / Standard avg input/request
TaskSpace / Standard tool calls
```

## 4. 2x 目标口径

v0.0.5 的主口径：

```text
TaskSpace direct input+output tokens <= 2x Standard
TaskSpace agent walltime <= 2x Standard
```

辅助口径：

```text
model_request_count_ratio <= 2.5x
avg_input_per_request_ratio <= 1.25x
uncached_input_ratio <= 2x
output_token_ratio <= 2x
```

允许 caveat：

```text
如果 public validator 或 Docker 是 suite bottleneck，不影响 TaskSpace/Standard agent-side ratio 判定。
```

## 5. Budget guardrail 作为保险丝

v0.0.5 可以保留 budget guardrail，但它不是主修复。

触发条件：

```text
- model_request_count > profile_limit 且 no new decision/adoption
- taskspace_control_count > profile_limit 且 no patch/validation progress
- projection_tokens > profile_budget for 2 consecutive turns
- large output >50KB would enter prompt
- no-yield subagent result count >= 2
```

动作：

```text
warn -> compact -> state_commit checkpoint -> thin downgrade -> hard stop
```

不要直接 hard stop，除非已经尝试 compaction/downgrade。

## 6. Cost-to-value metrics

v0.0.5 报告必须包含：

```text
extra_solved_pairs
extra_agent_minutes
extra_direct_tokens
extra_uncached_tokens
extra_tokens_per_additional_solved_pair
model_requests_per_decision
state_commits_per_decision
tokens_per_accepted_adopted_result
tokens_per_satisfied_criterion
```

## 7. 验收看板

建议 E3 aggregate 输出：

```text
[PASS/FAIL] cost target <=2x
[PASS/FAIL] model_request_ratio <=2.5x
[PASS/FAIL] avg_input_per_request_ratio <=1.25x
[PASS/FAIL] state_commit adoption >=80% of state updates
[PASS/FAIL] large output replay = 0
[PASS/FAIL] high_unreviewed reduced
[PASS/FAIL] solved not regressed beyond tolerance
```

## 8. 报告分层

| 层级 | artifact |
|---|---|
| side | `token-summary.json`, `context-projection-summary.json`, `state-management-summary.json` |
| pair | `pair-cost-report.md`, `pair-value-report.md` |
| sample | `sample-cost-summary.json`, `sample-routing-summary.json` |
| suite | `suite-cost-gate.json`, `suite-value-gate.md` |

## 9. 不再只看 total tokens

报告必须区分：

```text
accounting tokens
cached input tokens
uncached input tokens
output tokens
request count
context size
projection size
```

否则仍会混淆“缓存命中但 workload 大”和“实际不可承受成本”。
