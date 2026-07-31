# R8 已知问题唯一账本

- Created: 2026-07-31
- Updated: 2026-07-31
- Authority: R8 当前问题状态的唯一事实源
- Historical evidence: `docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md`

## 1. 使用规则

本账本迁移 R7.1 已观测到的问题，不迁移旧根因和旧修复方案。`Source` 只用于追溯历史证据。

新增问题必须满足以下之一：

- 当前源码或确定性测试证明独立缺陷；
- 当前有效 trace 证明新的用户可见或 Agent 可见异常；
- 当前问题深挖后发现无法归入既有问题的独立根因。

不得把一次失败中的多个日志表现重复登记为多个问题，也不得把计划、验收或发布步骤登记为产品问题。

## 2. 影响分层

| 层级 | 责任面 | 该层失败的影响 | 对应问题 |
|---:|---|---|---|
| F0 | canonical Map Store | Runtime 读取到非法或错误事实，所有上层判断失去基础 | I09 |
| F1 | Runtime 事务与 revision | 同一动作出现竞争状态或提交身份，导致 stale、重复提交和错误恢复 | I01 |
| F2 | Tool admission 与 dispatch | TaskSpace 硬约束可被入口绕过，出现未绑定或多 Patch 真实执行 | I06 |
| F3 | Tool feedback 与 context | Agent 收到丢失、重复、歧义事实，缓存前缀也可能被破坏 | I05、I02 |
| F4 | capability 与观测身份 | 可见能力和证据边界不稳定，行为与成本结论无法准确归因 | I10、I07 |
| F5 | Agent 协议行为 | Agent 生成低效或非法动作，但底层仍应守住正确性 | I03、I04 |
| F6 | 成本与晋升 | 衡量修复后的不可约产品成本，不能反向决定底层语义 | I08 |

## 3. 当前全集与优先级

`P0` 表示 canonical 正确性或不可绕过边界；`P1` 表示语义、能力身份或证据可信性；`P2` 表示 Agent 行为；
`P3` 表示修复后的成本和发布验收。执行序优先处理更底层责任面。

| 执行序 | ID | 层级 | 严重度 | 问题现象 | 直接影响面 | 主要下游影响 | 状态 | Source |
|---:|---|---:|---:|---|---|---|---|---|
| 1 | R8-I09 | F0 | P0 | Store hydrate 可能绕过 canonical schema 与 rooted-DAG 校验 | 所有 TaskSpace policy 的 resume、fork、child、进程重启 | 非法 Map 会污染 revision、状态机、反馈和行为分析 | [closed](I09/01-i09-store-hydrate-repair-result.md) | GI-009 |
| 2 | R8-I01 | F1 | P0 | prepare revision 与最终 canonical revision 同时成为成功事实 | 所有包含 ordinary sibling actions 的 TaskSpace response | stale、额外恢复请求、Final Receipt 和缓存问题 | queued | GI-001 |
| 3 | R8-I06 | F2 | P0 | nested dispatcher 可能绕过 response preflight 和单 Patch 边界 | 顶层/nested dispatch、Patch、Standard 能力隔离 | 未绑定动作或多个 Patch 可能真实执行 | queued | GI-006 |
| 4 | R8-I05 | F3 | P1 | 状态拒绝反馈存在重复、嵌套及 canonical/candidate 歧义 | 所有 TaskSpace 状态拒绝路径及 ToolSearch sibling | 同形重试、额外 read_map、错误状态理解 | queued | GI-005 |
| 5 | R8-I02 | F3 | P1 | 动态 Tool 事实被额外写成 developer/system message | 三种 projection、所有产生 receipt/failure carrier 的请求 | 缓存崩落；粗暴删除又可能丢唯一事实 | queued | GI-002 |
| 6 | R8-I10 | F4 | P1 | Tool discovery 缺少稳定 capability epoch | Standard/TaskSpace 的可见 Tool 集、缓存和 provenance | Tool 变化无法与 Map/context 变化准确区分 | queued | GI-010 |
| 7 | R8-I07 | F4 | P1 | Observer 曾误分类、漏载体、错误对账或接受陈旧证据 | benchmark、问题诊断、成本与晋升报告 | 错误根因、错误关闭和无效性能结论 | queued | GI-007 |
| 8 | R8-I03 | F5 | P2 | Agent 对 response 动作序列的遵循不稳定 | TaskSpace 初始化、ordinary actions、连续动作 | 额外拒绝与请求；需在 F0～F4 修复后重评 | queued | GI-003 |
| 9 | R8-I04 | F5 | P2 | Agent 向 Waiting/Completed 等不合法节点声明动作 | 有依赖关系的节点执行与 lifecycle mutation | 状态机拒绝和额外恢复；可能是反馈问题派生 | queued | GI-004 |
| 10 | R8-I08 | F6 | P3 | TaskSpace 固定与动态上下文成本高于 Standard | 请求、token、缓存、耗时和商业可行性 | 决定最终模式价值，但不能先于正确性收敛 | queued | GI-008 |

问题总数：**10**；Open：**9**；Closed：**1**。下一问题：**R8-I01**。

已关闭问题：
[`I09/00-i09-store-hydrate-repair-plan.md`](I09/00-i09-store-hydrate-repair-plan.md)。
[`I09/01-i09-store-hydrate-repair-result.md`](I09/01-i09-store-hydrate-repair-result.md)。

## 4. 依赖与重评关系

| 上游问题 | 关闭后必须重评 | 原因 |
|---|---|---|
| I09 Store hydrate | I01、I04 | 先确保进入 Runtime 的 Map 本身合法，才评价 revision 和节点行为 |
| I01 revision | I02、I03、I08 | 唯一最终状态是删除 receipt、评价动作请求和成本的前提 |
| I06 dispatch boundary | I03、I08 | 先保证所有入口共享硬门，行为和成本统计才完整 |
| I05 rejection feedback | I02、I03、I04 | 先确定失败事实的唯一表达，再删除动态副本并评价 Agent 行为 |
| I02 dynamic carrier | I03、I04、I08 | 先恢复自然上下文与缓存，再评价行为和不可约成本 |
| I10 capability epoch | I07、I08 | Tool 集变化必须能被观测和成本报告区分 |
| I03 response behavior | I04 | lifecycle 错误属于更具体的 response 行为，避免重复归因 |
| I01～I07、I09～I10 | I08 | 成本是最终验收，不作为底层设计的先验优化目标 |

I07 不作为所有问题的整体前置。每个底层问题先建设自身所需的最小、可重算证据；I07 随后只负责收敛跨问题
共用的观测身份和报告口径，避免再次形成长期 Observer 专项。

## 5. 已知但不作为独立问题迁移

| 事项 | R8 处理方式 |
|---|---|
| R7.1 的 20 个 Phase | 不迁移；其中包含调查、实现、评测和发布动作，不是问题全集 |
| 五层架构 | 不作为预设答案；相关职责边界按 R8 全局约束重新验证 |
| 三种 projection 的固有差异 | 保留为产品模式，不把已声明差异当成缺陷 |
| 旧 candidate 的完成度与晋升门 | 不迁移；R8 重新建立自己的实现与验收证据 |
| 历史兼容与旧 Map 数据 | 无保留价值，不建立兼容工作 |
| 未证明的“Agent 智能不足” | 不登记；优先检查上下文和 Tool 反馈 |

## 6. 关闭要求

每个问题关闭时必须在本表更新状态，并链接一份问题结果文档。结果文档必须包含：

- 实际根因和被否定的假设；
- 修改与删除的代码路径；
- 确定性测试、日志和回归结果；
- 对 Standard、连续动作、普通 Tool、Map Store、缓存和成本的影响；
- 若使用真实 Agent，关联全局运行账本；
- 对全局约束逐项检查的结论。
