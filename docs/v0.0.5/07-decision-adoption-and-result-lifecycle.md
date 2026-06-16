# 07. Decision Adoption and Result Lifecycle

## 1. 背景

v0.0.4 让 result adoption 变得可观测，但没有让它成为收敛机制。结果是：

```text
high_unreviewed_result_ratio: 15/15
subagent_no_decision_yield: 7/15
```

v0.0.5 要把 adoption 从“额外记账”改成“map 自我管理和上下文收敛”的核心机制。

## 2. Result Lifecycle

每个 result 必须进入以下生命周期之一：

```text
New
AcceptedAdopted
AcceptedRetained
Rejected
Deferred
Archived
AuditOnly
```

含义：

| 状态 | 含义 | 是否进入 active projection |
|---|---|---|
| New | 新结果，尚未处理 | 只短期进入，且数量受限 |
| AcceptedAdopted | 已采纳，支撑 fact/decision/criterion | 可进入 |
| AcceptedRetained | 可信但暂不支撑当前 decision | 按 salience 进入 |
| Rejected | 明确不使用 | 不进入 |
| Deferred | 暂缓，需条件触发 | 只保留摘要 |
| Archived | 已压缩或过期 | 不进入 |
| AuditOnly | 仅审计证据 | 不进入 |

## 3. 批量 Review

result review 不应一个 result 一个工具调用。

在 `state_commit` 中批量处理：

```json
{
  "result_updates": [
    {"result_id": "r1", "validity": "accepted", "adoption": "fact", "fact_id": "f1"},
    {"result_id": "r2", "validity": "rejected", "reason": "stale duplicate"},
    {"result_id": "r3", "validity": "deferred", "condition": "only revisit if validator fails"}
  ]
}
```

## 4. Decision Dependency Chain

每个 patch / validation / synthesis decision 必须有 why-chain：

```text
decision -> facts/results -> criteria/questions -> validation evidence
```

最小要求：

```text
patch decision:
  depends_on >= 1 accepted fact or result
  supports >= 1 success criterion
  created_by current node or phase summary

final synthesis:
  cites accepted decisions
  cites validation state or waiver
  unresolved blockers are explicit
```

## 5. Unreviewed Debt Policy

unreviewed result 不要求全部 review，但不能无限 active。

规则：

```text
New result active age <= N model requests
超过 N 后必须：accept / reject / defer / archive
```

建议初始：

```text
N = 3 state_commits 或 6 model requests
```

## 6. Subagent result policy

subagent result 必须明确处理：

```text
accepted -> 支撑 fact/decision
rejected -> 原因记录
explicitly deferred -> 触发条件记录
```

如果连续 K 个 subagent result 没有 decision yield：

```text
同类 subagent spawn 禁止，直到主 agent 解释新的 decision target。
```

建议：

```text
K = 2
```

## 7. Adoption 和 context projection 绑定

只有这些 result 可以进入 active projection：

```text
AcceptedAdopted
AcceptedRetained with high salience
Deferred only if its condition is active
```

New result 默认只进入短窗口；Rejected/Archived/AuditOnly 不进入。

## 8. 验收指标

| 指标 | 目标 |
|---|---:|
| high_unreviewed_result_ratio | 不再 15/15，下降 >= 60% |
| accepted_adopted_result_count | > 0 in 100% solved TaskSpace runs |
| subagent_decision_yield | 可计算，且 subagent-heavy runs > 0 |
| unreviewed active result age p95 | <= policy N |
| final synthesis decision chain | 100% |
| patch decision without support | 0 |

## 9. 注意事项

adoption 不能变成更多模型轮次。因此：

```text
- 必须走 state_commit 批量提交
- routine classification 可由 runtime 建议
- 模型只处理语义不确定项
```
