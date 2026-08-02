# R8 TaskSpace Tool 序列协议专题

- Created: 2026-08-01
- Status: MVT-0～MVT-6 completed / MVT-7 boundary verified / production ingress decision pending
- Priority: Foundation / blocks the existing issue queue
- Scope: TaskSpace 的 Agent 动作入口、Tool 顺序、节点归属与 Runtime 硬边界

## 1. 专题目的

当前 TaskSpace 要求 Agent 在同一响应中分别生成 `taskspace_control` 和若干普通 Tool call，再由 Runtime 通过
control 中的动作清单与普通 Tool 逐项配对。最近一次真实运行证明，这种要求能够在执行前拒绝非法响应，却不能在
Agent 生成动作时直接表达“这些调用共同构成一个合法序列”。结果是 Agent 多次生成单独普通 Tool、单独 control 或
归属不一致的 sibling calls，工作已经完成仍无法闭合 Map。

直接证据：[`../I01/02-i01-w9-map-always-repeat1-result.md`](../I01/02-i01-w9-map-always-repeat1-result.md)。

本专题不把该现象继续当作孤立的提示词问题或 I03 行为问题。它重新定义 TaskSpace 的顶层动作承载方式，使合法顺序
成为 Agent 直接生成的产品对象，同时保持原生 Tool 完全不知道 Map、节点和 TaskSpace。

## 2. 当前阶段

产品逻辑和不可违反的约束已经形成基线。当前先验证“单一序列容器 + 执行归属分派”能否在不侵入原生 Tool、
不放松执行前预检、也不改变 Standard provider wire 的前提下复用现有基建。可行性验证只回答基础路线是否成立，
不提前实施完整序列协议。

MVT-0 已形成用户接受的 Standard + map-request 真实基线。MVT-1 已证明 Function/Freeform 普通 Tool 可以从未来
序列项还原为原生 `ToolCall`，并继续复用同一个 `ToolRouter -> ToolRegistry -> handler/hook` 链路；没有修改普通
Tool schema，也没有增加第二套执行器。MVT-2 已证明同一序列调度器可在 Map 操作边界后，按 canonical Map 的 ready
frontier 处理 client/provider-hosted adapter；容器不需要也不得重复表达 Work 依赖。MVT-3 已证明非法 revision、
非法 node、双 Patch 及未满足依赖均在任何 client/hosted adapter 启动前零副作用拒绝。MVT-4～MVT-6 又证明了
受控 hosted 请求可由同一分派边界机械构造、不会污染主 Agent 会话，并能区分确定失败与结果未知且不自动重试。

MVT-7 核验发现：Standard 完整请求基线保持不变，但生产 TaskSpace 请求当前仍直接暴露原生 Tool；“主请求只暴露
序列容器”需要正式接入完整容器 schema 和入口分派，不能由测试专用假容器代替。当前因此停在正式工程设计的范围
决策点，不启动真实 Agent，也不把尚未接入的 H5 误报为完成。

文档顺序：

1. [`00-product-definition.md`](00-product-definition.md)：已确认的产品模型、角色边界、合法行为和执行归属原则。
2. [`01-execution-ownership-mvp-feasibility-plan.md`](01-execution-ownership-mvp-feasibility-plan.md)：执行归属方案的最小可行性测试计划。
3. [`02-mvt1-native-router-reuse-result.md`](02-mvt1-native-router-reuse-result.md)：MVT-1 原 Router 复用实现与验证结果。
4. [`03-production-engineering-plan.md`](03-production-engineering-plan.md)：基于 MVT-0～MVT-7 证据形成的生产容器
   schema、切换边界、工作单元、验收和安全停止计划。
5. 正式实施结果：执行计划后单独形成，不以 MVT 测试 adapter 代替生产完成证据。

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

## 4. 当前阶段完成条件

- 顶层序列、原生 Tool、`taskspace_control`、节点归属和 Runtime 的产品责任清楚且互不冲突；
- Standard 与 TaskSpace 的差异被限定在正确边界；
- 连续动作、多节点动作、终态动作和非法空转的产品规则明确；
- 已确认决策与待工程阶段选择的事项明确分开；
- 本地可行性测试能证明非法序列零执行、合法本地 Tool 复用原 Router、hosted Tool 只在预检后触发；
- 单一序列顺序能够成为调用和归属的唯一事实，不再产生 sibling manifest 或 shadow call；
- Standard 请求结构保持不变，TaskSpace 主请求不直接暴露 provider-hosted Tool；
- 测试结论明确区分“架构不可行”和“某个 provider 暂不支持受控 hosted 调用”；
- 只有可行性门槛通过后才进入完整工程设计。
