# build-R8 TaskSpace 逐题收敛章程

- Version: v0.0.5 build-R8
- Created: 2026-07-31
- Status: Active
- Previous milestone: `docs/v0.0.5/build-R7/`

## 1. 立项目的

R7/R7.1 在持续修复中积累了过多中间方案、编号、依赖门和局部补偿。部分工作已经形成有效能力和证据，但整体
推进出现三个问题：

1. 方案先于事实，尚未深挖完整调用链就开始设计；
2. 为防止回归不断增加并行合同、carrier、observer 和门禁，局部正确性换来了整体复杂度；
3. 多个问题同时推进，解决 A 又引入 B，难以判断单项修复的真实收益。

R8 不继续执行 R7.1 的 Phase 计划。R8 只继承：

- 已经观测到的十类问题及其原始证据；
- 用户确认的 TaskSpace 产品边界与不可回退约束；
- 当前代码和已提交测试这一客观工程现场。

R8 不继承 R7.1 的问题根因表述、修复方案、Phase 依赖图、关闭状态或五层架构作为预设答案。历史文档只读，
可以提供线索，不能替代 R8 的重新证明。

## 2. R8 的工作模型

R8 始终只有一个当前问题。每个问题按以下顺序推进：

```text
现场盘点
  -> 完整调用链与数据流
  -> 不变量和实际偏差
  -> 根因假设与反例
  -> 最小可证伪实验
  -> 方案比较
  -> 单轴实现
  -> 确定性测试与日志
  -> 必要时小预算真实运行
  -> 全局约束回归
  -> 关闭或回滚
```

前一个问题没有形成明确结论前，不实施下一个问题。允许并行的工作仅限于互不写入的资料收集、源码阅读和测试
执行；不得并行实施两个产品行为变更。

## 3. 每题必须回答的问题

每个问题至少回答：

1. 用户或 Agent 实际看到了什么？
2. Provider 实际收到了什么？
3. Agent 生成了哪些 Tool calls？
4. Runtime 在执行前、执行中和执行后分别做了什么？
5. Map Store 中的 canonical 事实如何变化？
6. 哪些事实进入自然上下文，角色、位置和次数分别是什么？
7. 哪一步发生了丢失、扭曲、重复、歧义或越界决策？
8. Standard 为什么发生或不发生同类问题？
9. 如果不改 Runtime 语义，只修复事实传递，问题是否消失？
10. 修复是否破坏连续动作、普通 Tool 保真、Map 持久化、缓存或成本？

没有回答这些问题，不得进入实现。

## 4. 证据等级

| 等级 | 证据 | 可以支持的结论 |
|---|---|---|
| E0 | 文档、旧分析、代码命名 | 只用于发现线索 |
| E1 | 当前源码调用链、静态结构、确定性 fixture | 证明实现事实和局部不变量 |
| E2 | 当前二进制的 replay、集成测试、provider payload | 证明真实链路行为 |
| E3 | 单变量真实 Whale Agent run | 证明模型行为、缓存和成本影响 |

根因至少需要 E1 和 E2；涉及 Agent 行为、缓存或产品收益时还需要 E3。一次实验只改变一个变量。真实运行继续
遵守全局预算门禁和 `benchmarks/whale-agent-run-ledger.json`。

## 5. 方案准入规则

方案进入代码前必须满足：

- 明确对应哪一个已证明根因；
- 解释为什么不能只删除错误逻辑；
- 说明是否增加新概念、状态、消息、schema 字段或分支；
- 至少比较“删除/收敛现有机制”和“新增机制”两个方向；
- 涉及产品体验或重大技术路线时先与用户讨论；
- 一个实现批次只改变一个主要行为轴；
- 失败时可以整体回退，不保留双路径和静默兼容。

默认偏好删除错误抽象、合并权威来源和复用原生 Tool 机制。新增 carrier、提示词、状态、observer 分类或
Runtime 拒绝规则必须证明现有原语无法表达。

## 6. 状态与文档

R8 只维护以下状态：

- `queued`：已知问题，尚未开始；
- `investigating`：正在收集事实，未确认根因；
- `design-review`：根因已确认，方案等待审查或用户决策；
- `implementing`：单轴方案正在实现；
- `verifying`：代码已实现，正在执行确定性与真实运行验证；
- `closed`：验收通过；
- `rejected`：旧问题表述不成立或不属于 TaskSpace 缺陷。

唯一问题状态记录在 `01-r8-known-issues.md`。每个问题的详细调查、设计和结果使用独立文档，不再建立另一套
总状态。

## 7. 外部依据

R8 使用外部规范校验协议边界，但不照搬外部架构：

- [OpenAI API：Tool choice 与 function Tool schema](https://platform.openai.com/docs/api-reference/realtime-calls)
  说明模型可在一次响应中选择一个或多个 Tool，Tool 能力通过独立 schema 暴露；
- [Model Context Protocol：Tools](https://modelcontextprotocol.io/specification/2025-03-26/server/tools)
  将 Tool 定义和 Tool result 作为明确协议对象，Tool 由模型调用，结果应通过对应调用返回；
- [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache)
  明确缓存依赖完整前缀单元，解释了中途插入动态 system message 的成本风险；
- [SQLite Atomic Commit](https://sqlite.org/talks/howitworks-20240624.pdf)
  用于核对 Map Store 原子提交和失败回滚边界。

这些资料只支持原生 Tool 结果、前缀缓存和事务边界等通用事实，不预先决定 TaskSpace 的具体实现。
