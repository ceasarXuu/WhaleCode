# TaskSpace Runtime 重构方案二次对抗复核

日期：2026-05-27

审查来源：`claude-ds-pro`

审查对象：

- `docs/plans/2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md`
- `docs/plans/2026-05-27-taskspace-runtime-rearchitecture-adversarial-review.md`

## Summary

二次复核结论：实施计划已完整吸收第一轮对抗审查的 P0/P1 阻塞意见。每条 Required Fix 都能在更新后的实施方案中找到对应设计。

## Required Fixes Coverage

| # | 第一轮审查要求 | 二次复核结论 |
| --- | --- | --- |
| 1 | 成本信号硬阻断规则 | 已覆盖：`MaintenanceBarrier` 与 gate 检查 `maintenance_barrier == None` |
| 2 | `single_node_reason` 不作为防线 | 已覆盖：审计字段 + 单节点严格预算 |
| 3 | `TaskRoutingDecision` | 已覆盖：`route_task` 四分支决策 |
| 4 | main lease 状态机 | 已覆盖：`bind_node` 拒绝已有 lease，`finish_node` 处理抢占和 idle |
| 5 | lease claim 原子化 | 已覆盖：要求在 `SessionState` 锁内原子执行 |
| 6 | reborn + running subagent | 已覆盖：running lease 存在时拒绝 reborn |
| 7 | phase 顺序 | 已覆盖：Phase 2 先实现 finish/block，Phase 3 再上线 gate |
| 8 | rollout replay repair | 已覆盖：损坏引用进入 `repair_required` |
| 9 | EdgeKind | 已覆盖：定义 `Dependency` / `Related` 及推进规则 |
| 10 | Phase 0 自动化回归 | 已覆盖：要求自动化 fixture 和旧实现失败/新实现阻断断言 |

## Remaining Risks

### P2: Phase 3 到 Phase 5 的过渡期仍需临时限制 spawn

如果主工具 gate 已上线，但 `spawn_agent(node_id)` 和并发 claim 原子化尚未完成，过渡期仍可能出现并发 spawn 竞争同一节点。建议在 Phase 3 验收中明确：Phase 4/5 完成前，TaskSpace 模式下 spawn 暂时只允许唯一 ready node 场景。

### P2: `MaintenanceBarrier` 当前是全局单例

`TaskSpaceRuntimeState` 中只有一个 `Option<MaintenanceBarrier>`。当前一次处理一个超预算节点是可接受的，但实现时要确保一个屏障解除后，其他已经超预算的节点能重新触发屏障。

### P2: 单节点严格预算可能过紧

单节点 task 的预算为普通工具结果数 3、`apply_patch` 0。该阈值能防止宽任务绕过，但对极窄修改可能过紧。上线后需要用真实任务校准。

### P2: free-form result 与 event replay 的边界

`result_summary` 允许 free-form，这是合理的。但事件仍必须携带结构化 `NodeResultId`、task/map/node/lease 坐标，确保 replay 和 viewer 可追溯。

## Verdict

没有阻塞问题。可以进入工程实现。残余风险均为 P2，不阻塞开工，但应在实现计划或验收测试中跟踪。
