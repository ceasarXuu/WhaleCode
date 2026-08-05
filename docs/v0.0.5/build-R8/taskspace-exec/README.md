# R8 TaskSpace Exec 主方案

- Created: 2026-08-05
- Status: Phase A reopened at A2 / Phase B blocked / 生产代码尚未接入
- Priority: Foundation / blocks the existing R8 issue queue
- Scope: TaskSpace 的唯一 client Tool 入口、合法动作序列、节点归属与 Hosted 结果核对

## 1. 路线决策

R8 从本专题起以 `taskspace_exec` 作为 TaskSpace 顶层动作协议的唯一主方案：

1. `taskspace_exec` 是一个 Function Call 形态的超级工具，参考 Codex `exec/code-mode` 的内部 Tool 暴露、嵌套
   调用和原 Router 复用方式，但不照搬其 Freeform Tool wire。
2. TaskSpace 请求不再向 Agent 顶层暴露普通 client Tool。普通 client Tool 和 `taskspace_control` 的能力说明由
   `taskspace_exec` 从原生 `ToolSpec` 派生并在内部暴露。
3. Agent 在 `taskspace_exec` 内声明待执行的 client Tool、每次调用的 `node_id`、TaskSpace Map 操作，以及本响应
   每项已完成 provider-hosted Tool 的节点归属。
4. Runtime 对 client 部分执行机械预检、解析和原生 Tool dispatch；对 provider 部分不重执行。Runtime 直接复用
   provider 原始结果中的 `id/item_id`，但节点归属必须由 Agent 逐项声明并完整核对。
5. `taskspace_exec` 只增加两个 TaskSpace 职责：合法序列和节点绑定。它不规划任务、不选择节点、不解释 Tool 结果，
   也不根据 Tool 成败推进节点状态。

旧的普通 Tool schema 入侵、顶层结构化序列容器和 `taskspace_control.actions[] + sibling calls` 三条路线降级为候选，
统一封存在 [`../tool-sequence-protocol/`](../tool-sequence-protocol/README.md)。它们不再拥有生产实施状态；只有主方案
被证据否定且用户重新决策后，才可重新评估。

## 2. 当前事实

- 最新 Codex 主线仍使用一个 `exec` 入口，将 Function、Freeform 和 Namespace Tool 从原 `ToolSpec` 派生为内部
  ToolDefinition，并把嵌套调用送回统一 Tool runtime。
- Codex 主线 `exec` 是 Freeform JavaScript Tool；Whale 已通过本地改造和一次 DeepSeek V4 Flash 真实编码闭环证明
  `{source: string}` Function Call 形态能够进入相同嵌套 Tool 路径。
- 现有证据证明“Function 超级工具 + 原 Router”可行，但尚未证明 TaskSpace 完整合法序列的副作用前批次预检，以及
  provider-hosted 逐项双写身份在所有 provider 输出上的稳定性。
- 当前生产代码仍运行旧 sibling 协议。文档主方案切换不等于生产切换；正式切换必须是一次可验证的原子迁移，随后
  删除旧路径，不保留兼容双轨。
- Phase A 已证明严格声明式 source、完整 typed plan、零副作用 preflight，以及 Runtime 可直接读取 Provider
  `id/item_id`。A2 先前只验证了整响应单节点原型，未覆盖同响应多 Hosted 动作分别绑定多个节点，也错误接受了
  Root/unbound 降级；A2 已回撤为未完成，Phase B 暂停。

## 3. 文档

1. [`00-product-contract.md`](00-product-contract.md)：已确认的产品语义、Agent/Runtime/Provider 边界和非目标。
2. [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md)：最新 Codex 主线事实、本地 Function
   exec 证据和可复用边界。
3. [`02-engineering-plan.md`](02-engineering-plan.md)：根据 Phase A 结果重排后的 response envelope、Map/dispatch、Hosted
   持久化、反馈、切换和验证计划。
4. [`03-global-issue-prerequisite-review.md`](03-global-issue-prerequisite-review.md)：I01～I10 哪些前置、融入或后置的
   唯一映射，以及 I07 计数子问题的 TX-00 边界。
5. [`04-phase-a-discovery.md`](04-phase-a-discovery.md)：当前生产、Codex 上游 seam 和旧协议删除清单。
6. [`05-phase-a-result.md`](05-phase-a-result.md)：TX-01～TX-05 实施结果和 A2 纠偏结论。
7. [`06-a2-revalidation-result.md`](06-a2-revalidation-result.md)：A2 既有证据、失效结论和重新打开原因。
8. [`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md)：A2 逐项多节点绑定的分步验证与完成门禁。
9. [`08-a2-v1-v3-result.md`](08-a2-v1-v3-result.md)：V1～V3 的 wire、候选合同和原子拒绝离线证据。
10. [`09-a2-v4-first-probe-result.md`](09-a2-v4-first-probe-result.md)：首次真实 V4 probe 的失败事实、测试混杂因素和复验前置修正。

## 4. 推进规则

- R8 已知问题队列继续暂停，直到该主方案完成生产接入并重新盘点 I01～I10。
- 唯一允许在 Phase A 前实施的全局问题子范围是 TX-00：修复 I07 已坐实的 usage/request 聚合错误；它不改变 Tool、
  Map、prompt 或 provider 行为。
- 不再向旧三类方案追加生产实现；必要的历史 fixture 只能作为新方案的反例或回归证据。
- 每个阶段只验证一个主要不变量；涉及 provider/Agent 行为的真实运行必须重新申请预算。
- Tool declaration、prompt 或 provider payload 发生变化时，先运行缓存敏感面门禁，再说明变化并申请真实缓存回归。
- 生产代码变更完成后按项目规则另行申请对抗性审查；本次只建立路线合同和工程计划。
