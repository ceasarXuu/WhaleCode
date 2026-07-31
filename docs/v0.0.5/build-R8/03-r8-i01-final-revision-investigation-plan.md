# R8-I01 最终 Revision 单一权威调查计划

- Created: 2026-07-31
- Issue: R8-I01
- Status: planned
- Change policy: 调查阶段不修改产品行为，不运行真实 Whale Agent

## 1. 调查问题

当前一次 TaskSpace response 可能经历：

```text
control prepare
  -> sibling ordinary Tool dispatch
  -> ordinary result attribution
  -> canonical Map commit
  -> Agent-visible feedback
```

历史实现同时向 Agent 暴露 prepare revision 和归档后的 revision。R8-I01 不预设“延迟 control result”或
“保留 Final Receipt”是正确方案，而是先回答：

1. 一次 response 内实际有几次 Store transaction 和 revision 推进？
2. 每次推进分别对应什么不可合并的事实？
3. `taskspace_control` Tool output 在何时被构造并写入自然历史？
4. sibling ordinary Tool output 在何时执行、归档和写入历史？
5. Runtime 是否能够在不改变普通 Tool 的情况下，让 control result 携带最终权威状态？
6. 如果不能，限制来自 Codex 调度、DeepSeek wire、call ID 顺序还是当前自有实现？
7. preflight 拒绝、ordinary success、ordinary failure、skip、并行 sibling 和 `finish_map` 是否需要不同处理？
8. 哪些动态 carrier 是纯重复，哪些仍承载唯一事实？

## 2. 工作单元

| ID | 目标 | 位置 | 动作 | 产出 | 验证 | 安全停止 |
|---|---|---|---|---|---|---|
| I01-A | 还原生产调用链 | TaskSpace sequence、parallel、Store、provider history 构造路径 | 从 response parse 到下一 provider request 逐函数追踪 revision、call ID、消息角色和持久化时点 | 带源码锚点的时序图 | 每条边可由当前代码定位 | 仅文档，无行为变更 |
| I01-B | 建立 revision 事实表 | control result、reservation、attribution、receipt、projection | 列出 producer、含义、可见角色、消费者和是否唯一 | revision authority matrix | 当前 fixture/replay 能逐项对账 | 不新增 observer 框架 |
| I01-C | 覆盖关键分支 | 现有 TaskSpace integration/replay tests | 增加或整理 success/failure/skip/multi-sibling/finish 的确定性 trace fixture | 分支事实证据 | 测试断言 revision 与消息次数 | 不改变产品行为 |
| I01-D | 对抗根因假设 | 调用链与 fixture | 检查双权威是否来自消息时点、两阶段事务、projection 或解析错误 | 根因与反例报告 | 至少一个竞争假设被证伪 | 证据不足则继续调查 |
| I01-E | 方案比较 | 只基于 A-D 事实 | 比较删除重复、调整事务边界、调整原生 control result 时点等方向 | 代价/收益/约束表 | 用户确认重大路线后才实施 | 不在本单元改代码 |

## 3. 必须覆盖的场景

| 场景 | 必须核对 |
|---|---|
| preflight reject | 零 dispatch、零 Map commit、唯一失败结果 |
| 单 ordinary success | prepare、attribution、最终 revision 和 Tool outputs |
| ordinary failure | 原生失败保留、Map 如何记录、节点状态是否变化 |
| 多 sibling | 每个 action 的归属、提交顺序、最终 revision |
| skipped/cancelled | reservation 如何闭合，是否形成伪成功 revision |
| complete + next actions | mutation 与后继动作是否同一事务闭环 |
| finish_map | 最后 Work、Finish、Root 和 summary 的 revision |
| map-request 下一轮 | Agent 实际可见的唯一可提交 revision |

## 4. 调查验收

R8-I01 进入设计审查前必须满足：

- 完整时序图没有“Runtime 随后处理”等模糊节点；
- 每个 revision 的 producer、commit 时点、持久化状态和 Agent-visible carrier 明确；
- 所有关键分支至少有 E1 证据，主成功路径和一个失败路径有 E2 证据；
- 明确说明 TaskSpace response 级动作约束如何保留；
- 明确说明普通 Tool 为什么保持原生；
- 不用真实模型行为弥补无法解释的 Runtime 时序；
- 根因结论能够指出应删除或修改的具体责任边界。

调查完成后先向用户汇报并讨论方案，不自动进入实现。
