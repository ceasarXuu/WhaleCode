# R8 TaskSpace Tool 序列协议专题

- Created: 2026-08-01
- Status: Product definition
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

当前只定义产品逻辑和不可违反的约束，不编写工程设计，不选择 provider schema、解析结构、反馈 wire 或迁移步骤。

文档顺序：

1. [`00-product-definition.md`](00-product-definition.md)：已确认的产品模型、角色边界、合法行为和非目标。
2. 工程设计：尚未开始；必须在产品定义确认后另建文档。
3. 实施计划：尚未开始；必须从已确认的工程设计派生。

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

## 4. 本阶段完成条件

- 顶层序列、原生 Tool、`taskspace_control`、节点归属和 Runtime 的产品责任清楚且互不冲突；
- Standard 与 TaskSpace 的差异被限定在正确边界；
- 连续动作、多节点动作、终态动作和非法空转的产品规则明确；
- 已确认决策与待工程阶段选择的事项明确分开；
- 用户确认该产品定义后，才进入工程设计。
