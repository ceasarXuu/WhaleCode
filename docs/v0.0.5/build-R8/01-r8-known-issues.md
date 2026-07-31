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

## 2. 当前全集

| 顺序 | ID | 问题现象 | 当前已知事实 | 仍需重新证明 | 状态 | Source |
|---:|---|---|---|---|---|---|
| 1 | R8-I01 | 同一 response 中存在 prepare revision 与最终 canonical revision 两种成功值 | ordinary result attribution 会再次推进 Map；历史 stale 与此相关 | 完整事务时序、唯一权威应在何时产生、如何满足 response 级约束 | investigating | GI-001 |
| 2 | R8-I02 | 动态 Tool 事实被额外写成 developer/system message，破坏缓存前缀 | 全 carrier 消融使 request 2+ cache 从 33.60% 恢复至 84.85% | 哪些消息是纯重复，哪些承载唯一事实；生产删除边界 | queued | GI-002 |
| 3 | R8-I03 | Agent 对 TaskSpace response 动作序列的遵循不稳定 | 出现 ordinary-only、solo init、数量或顺序不匹配 | 是反馈、Tool 合同、协议说明还是模型偶发问题 | queued | GI-003 |
| 4 | R8-I04 | Agent 会向 Waiting/Completed 等不合法节点声明动作 | 状态机能拒绝，Agent 通常能在后续纠正 | Agent 调用前实际获得了哪些节点事实，是否存在传递问题 | queued | GI-004 |
| 5 | R8-I05 | 状态拒绝反馈存在重复、嵌套和 canonical/candidate 状态歧义 | 多条 carrier 曾表达同一 violation | 当前所有输出路径是否仍重复或扭曲，去重后的完整事实集合 | queued | GI-005 |
| 6 | R8-I06 | 单 response 单 Patch 规则可能被 nested dispatcher 绕过，Agent 也曾尝试多 Patch | 顶层 preflight 可零执行拒绝；nested inventory 仍需核对 | 能力入口全集、是否真实绕过、最简单统一边界 | queued | GI-006 |
| 7 | R8-I07 | Observer 曾误分类、漏载体、错误对账或接受陈旧证据 | 多轮修复后仍发生过 fresh review blocker | 当前最小可信观测面；哪些通用 observer 机制应删除或保留 | queued | GI-007 |
| 8 | R8-I08 | TaskSpace 固定与动态上下文成本仍高于 Standard | 唯一 control schema 有固定成本；动态 carrier 已确认破坏缓存 | 修复已知异常后的不可约成本和各 projection 的真实成本 | queued | GI-008 |
| 9 | R8-I09 | Store hydrate 可能绕过 canonical schema 与 rooted-DAG 校验 | 写入路径有校验，历史读取路径存在直接安装风险 | 当前生产 hydrate 调用链、共享 validator 与原子失败行为 | queued | GI-009 |
| 10 | R8-I10 | Tool discovery 缺少稳定 capability epoch | 可见 Tool 集可能按请求变化，旧 epoch 身份未覆盖该变化 | 是否影响正确性、缓存或仅影响观测；最小身份模型 | queued | GI-010 |

当前问题数：**10**。当前问题：**R8-I01**。

## 3. 已知但不作为独立问题迁移

| 事项 | R8 处理方式 |
|---|---|
| R7.1 的 20 个 Phase | 不迁移；其中包含调查、实现、评测和发布动作，不是问题全集 |
| 五层架构 | 不作为预设答案；相关职责边界按 R8 全局约束重新验证 |
| 三种 projection 的固有差异 | 保留为产品模式，不把已声明差异当成缺陷 |
| 旧 candidate 的完成度与晋升门 | 不迁移；R8 重新建立自己的实现与验收证据 |
| 历史兼容与旧 Map 数据 | 无保留价值，不建立兼容工作 |
| 未证明的“Agent 智能不足” | 不登记；优先检查上下文和 Tool 反馈 |

## 4. 关闭要求

每个问题关闭时必须在本表更新状态，并链接一份问题结果文档。结果文档必须包含：

- 实际根因和被否定的假设；
- 修改与删除的代码路径；
- 确定性测试、日志和回归结果；
- 对 Standard、连续动作、普通 Tool、Map Store、缓存和成本的影响；
- 若使用真实 Agent，关联全局运行账本；
- 对全局约束逐项检查的结论。
