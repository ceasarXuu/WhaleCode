# R8 TaskSpace Tool 序列协议专题

- Created: 2026-08-01
- Updated: 2026-08-05
- Status: Phase A discovery active / TS-01～TS-04 completed / hosted node binding and container presence unresolved
- Priority: Foundation / blocks the existing issue queue
- Scope: TaskSpace 的 Agent 动作入口、Map 边界顺序、节点归属与 Runtime 硬边界

## 1. 专题目的

当前 TaskSpace 要求 Agent 在同一响应中分别生成 `taskspace_control` 和若干普通 Tool call，再由 Runtime 通过
control 中的动作清单与普通 Tool 逐项配对。最近一次真实运行证明，这种要求能够在执行前拒绝非法响应，却不能在
Agent 生成动作时直接表达“这些调用共同构成一个合法序列”。结果是 Agent 多次生成单独普通 Tool、单独 control 或
归属不一致的 sibling calls，工作已经完成仍无法闭合 Map。

直接证据：[`../I01/02-i01-w9-map-always-repeat1-result.md`](../I01/02-i01-w9-map-always-repeat1-result.md)。

本专题不把该现象继续当作孤立的提示词问题或 I03 行为问题。它重新定义 TaskSpace 的顶层动作承载方式，使合法行动
和 Map 边界成为 Agent 直接生成的产品对象，同时保持原生 Tool 完全不知道 Map、节点和 TaskSpace。

## 2. 当前阶段

产品逻辑和不可违反的约束已在 2026-08-04 根据 provider-hosted Tool 的实际响应路径重新对齐。容器方向继续成立，
但容器不再被误解为“所有 Tool 都等待 Runtime 预检后执行”的统一执行队列，而是 TaskSpace 唯一的行动与节点归属账本：

- client-managed Tool 和 `taskspace_control` 仍只通过容器提交；
- provider-hosted Tool 仍由 provider 原生执行，容器引用本响应内已完成的原生输出并声明节点归属；
- Tool 执行状态与节点生命周期正交，Runtime 不根据 Tool 成败自动推进、阻塞或关闭节点；
- 容器只约束 Map 操作位于合法边界，不重复表达普通 Work 之间的 DAG。

MVT-0 已形成用户接受的 Standard + map-request 真实基线。MVT-1 已证明 Function/Freeform 普通 Tool 可以从未来
序列项还原为原生 `ToolCall`，并继续复用同一个 `ToolRouter -> ToolRegistry -> handler/hook` 链路；没有修改普通
Tool schema，也没有增加第二套执行器。TS-04 已证明 `taskspace_control` 同样可以复用统一 Router 生命周期和唯一 Map
事务。这两项仍是正式方案的直接基础。

MVT-2～MVT-6 对 hosted adapter、统一 preflight 和 ready frontier 的验证结果保留为历史工程证据，但其产品解释已经
被替代：原生 hosted 输出在 Runtime 收到响应前已经发生，不能被容器预检或 Map 事务回滚；Tool outcome 也不决定节点
状态。专用 hosted executor 只证明适配器可行，不再是正式方案默认方向。

MVT-7 核验的 Standard 隔离事实仍有效。正式 TaskSpace 请求应同时暴露序列容器和 provider 支持的原生 hosted
capability；顶层 client-managed Tool 不再单独暴露。正式工程计划正在按该请求/响应结构更新，不能由测试专用假容器
或 hosted proxy adapter 代替。

文档顺序：

1. [`00-product-definition.md`](00-product-definition.md)：已确认的产品模型、角色边界、合法行为和执行归属原则。
2. [`01-execution-ownership-mvp-feasibility-plan.md`](01-execution-ownership-mvp-feasibility-plan.md)：历史最小可行性测试与
   证据；其 hosted adapter 产品解释已被 2026-08-04 决策替代。
3. [`02-mvt1-native-router-reuse-result.md`](02-mvt1-native-router-reuse-result.md)：MVT-1 原 Router 复用实现与验证结果。
4. [`03-production-engineering-plan.md`](03-production-engineering-plan.md)：按原生 hosted 输出核对、Tool/节点状态正交
   和 Map 边界约束重写后的生产容器计划。
5. [`04-ts04-control-router-seam-result.md`](04-ts04-control-router-seam-result.md)：停点 1 的统一 Router 生命周期、
   单一 Map transaction 和唯一 binding 来源验证结果。
6. [`05-phase-a-ts01-ts03-validation-result.md`](05-phase-a-ts01-ts03-validation-result.md)：client Tool 能力矩阵、hosted
   identity/request wire 本地验证，以及节点归属与容器必达性的剩余合同缺口。
7. 后续正式实施结果：按工作单元单独形成，不以 MVT 或 seam 测试 adapter 代替生产完成证据。

## 3. 与 R8 其他问题的关系

从本专题建立起，`01-r8-known-issues.md` 中尚未关闭问题暂停按原顺序实施。原因不是这些问题已经消失，而是新的顶层
动作协议可能：

- 直接取代 I03 当前的 control manifest + sibling calls 问题模型；
- 改写 I06 的统一入口与不可绕过边界；
- 改变 I01、I02、I05 的提交结果和反馈承载方式；
- 改变 I08 的请求、Tool schema 和上下文成本；
- 使 I04 的一部分错误自然消失，也可能暴露新的节点顺序问题；
- 要求 I07、I10 的观测和能力身份按新协议重评。

专题完成后必须重新盘点问题全集，不能直接恢复旧队列，也不能自动把受影响问题判为关闭。

## 4. 正式实施入口条件

- 顶层序列、原生 Tool、`taskspace_control`、节点归属和 Runtime 的产品责任清楚且互不冲突；
- Standard 与 TaskSpace 的差异被限定在正确边界；
- 连续动作、多节点动作、终态动作和非法空转的产品规则明确；
- 已确认决策与待工程阶段选择的事项明确分开；
- 本地可行性测试能证明非法 client/map 序列零执行、合法本地 Tool 复用原 Router、hosted 原生输出可被唯一核对；
- 单一容器能够成为行动和归属的唯一事实，Map 边界顺序不再由 sibling manifest 或 shadow call 二次表达；
- Standard 请求结构保持不变，TaskSpace 主请求只额外保留 provider 原生 hosted capability；
- Tool 执行事实与节点生命周期分别结算，不相互推导；
- 测试结论明确区分“client/map 容器不可行”和“某个 provider hosted 输出缺少稳定引用身份”；
- TS-02 已证明 hosted 输出存在稳定的响应内身份；TS-06 必须进一步冻结 Agent 声明节点归属与 Runtime 使用该身份登记事实的
  单一合同，TS-07～TS-09 合同冻结后，才进入生产代码实施。
