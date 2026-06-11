# ADR-0001: 0.0.4 定位为 Problem-State Runtime，而不是 Planner

## Status

Accepted for 0.0.4 draft.

## Context

0.0.3 已证明 TaskSpace 能接入真实 Whale 执行路径，但未证明 utility 正收益。E3 中 graph activity 增加没有稳定转化为更高成功率。

## Decision

0.0.4 不做 full automatic planner，不新增复杂规划算法。0.0.4 聚焦：

```text
ProblemStateLedger
ResultAdoption
GraphHealth
CleanAudit
```

## Consequences

- runtime 继续不判断语义真假；
- main agent 继续负责语义路由和决策；
- runtime 维护显式状态和引用；
- 版本成功以可审计性和行为 contract 为核心，而非立刻 utility win。
