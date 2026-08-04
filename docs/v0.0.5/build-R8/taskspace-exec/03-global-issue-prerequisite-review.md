# TaskSpace Exec 与 R8 全局问题前置关系审查

- Created: 2026-08-05
- Status: Reviewed planning decision
- Authority: TaskSpace Exec 实施顺序与 R8-I01～I10 的唯一映射
- Changes issue states: No

## 1. 审查问题

在开始 TaskSpace Exec 工程前，需要判断 R8 全局问题中哪些应先独立解决、哪些应成为新方案自身的验收条件、哪些只能
等新协议落地后重新观测。判断标准不是旧问题编号，而是修复对象是否会被新入口替换：

1. 若问题污染 canonical Map 或所有后续证据，必须先处理；
2. 若问题发生在新方案必须复用的底层能力，应融入最早对应工作单元；
3. 若问题只存在于即将删除的 sibling/manifest/feedback 路径，不再单独修旧实现；
4. 若问题属于 Agent 行为或成本，只能在新协议正确、观测可信后评价；
5. 融入计划不等于问题自动关闭，仍须按全局问题账本的证据要求结算。

## 2. 审查结论

| 问题 | 与 TaskSpace Exec 的关系 | 决策 | 对应工作单元 | 问题状态处理 |
|---|---|---|---|---|
| I09 Map 恢复合法性 | 所有 Runtime/Map 工作的事实前提 | 已关闭，满足建设前置，不新增动作 | 已有 I09 证据；TX-14 做邻接回归 | 保持 closed |
| I07 观测可信性 | 已确认的请求/token 双计会污染后续成本结论；本地 reject 口径会被新 trace 改写 | 只把已坐实的计数子问题前置为 TX-00；其余融入 TX-11 | TX-00、TX-11、TX-14 | 保持 queued，直到完整 I07 验收 |
| I10 能力身份 | exec 内部 catalog、实际 Router、缓存和报告必须认同同一工具能力版本 | 不做旧路径外围补丁；提升为外层合同和共享 catalog 的硬条件 | TX-02、TX-06、TX-12、TX-14 | 保持 queued，生产切换后重评 |
| I06 统一 admission | 超级工具内部调用如果直接复用当前 Code Mode dispatch，可能绕过顶层 sequence preflight | 不独立修旧 sibling；作为 typed plan、preflight 和 nested dispatch 的核心验收 | TX-03、TX-04、TX-08、TX-14 | 保持 queued，生产切换后重评 |
| I01 唯一最终 revision | W0～W8 已形成确定性基础，但旧协议 E3 会在切换后失效 | 保留当前实现和测试，不继续旧 W9/W10；新外层结果重新验证 | TX-10、TX-14、TX-15 | 保持 verifying |
| I05 拒绝反馈 | 旧 pairing output、developer message 和状态事实重复会被 outer exec result 替换 | 不修旧 carrier；在新结果中一次区分 preflight、Tool 和 Map 事实 | TX-10、TX-14 | 保持 queued |
| I02 上下文重复 | 旧独立高优先级副本会随旧反馈路径删除 | 不单独优化旧上下文；用新 final wire 和缓存门禁验收 | TX-10、TX-12、TX-14、TX-15 | 保持 queued |
| I03 动作组合 | 主要证据来自旧 control manifest + sibling 生成失败 | 不在旧协议上增强 prompt 或拒绝；新 ToolSpec 接入后真实观测 | TX-02、TX-12、TX-15 | 保持 queued |
| I04 节点选择 | 需要先排除协议、反馈和上下文失真 | 不增加 Runtime 语义约束；新协议正确后重测 | TX-14、TX-15、TX-16 | 保持 queued |
| I08 成本 | 依赖能力身份、计数、反馈和 Agent 行为全部可信 | 最后评价，不反向删减语义或硬规则 | TX-00、TX-12、TX-15、TX-16 | 保持 queued |

## 3. 唯一独立前置：I07 的已确认计数子问题

这里不新增 `I07-A` 产品问题编号，只把 I07 中已确定、与新旧协议正交的一个工程子范围记为 TX-00。

当前事实：

- `Session::update_rate_limits()` 更新 rate-limit state 后发送一条没有 `provider_request_id` 的 `TokenCount`。该事件仍
  携带当前 `last_token_usage`，但语义是状态广播，不是新的 provider 请求完成事实；
- `New-TaskspaceRolloutRequestTraceSummary` 只检查 `token_count + last_token_usage`，把上述状态广播再次累计为请求、
  input、cached input 和 output；
- 最近真实证据中 8 个 provider 请求因此被报告为 15 个，token 也近似双计；
- 费用账本使用独立 reconciled provider boundary，没有受该错误污染，但 benchmark 汇总不可用于比较。

TX-00 的最小修复边界是消费语义，不预设删除 Runtime 的 rate-limit UI 事件：

1. 只有带完整 `provider_request_id` 的 provider-completed usage event 才能建立请求边界并累计该请求 usage；
2. 无 ID 的 TokenCount 继续可以更新 UI/状态，但不得进入 request/token 聚合；
3. 相同 `provider_request_id` 重复出现且 usage 一致时按一次计算；内容冲突时 fail closed，不能静默选择；
4. fixture 必须复现“一个 completed usage + 一个无 ID rate-limit snapshot”的真实成对形态；
5. provider boundary、final wire 与 rollout 聚合对同一请求集合逐 ID 对账。

I07 的另一部分“本地 preflight reject 被当成 upstream mismatch”不在 TX-00 修旧语义。TaskSpace Exec 会改变 preflight
和内部 item trace，应在 TX-11 按新身份模型解决。

## 4. 两个必须融入主方案的硬问题

### 4.1 I10：一个能力身份

当前 `trace_taskspace_provider_tool_schema_profile()` 计算的 `tool_set_sha256` 只写日志；`provider_wire_trace` 又对最终
`tools` 独立计算 `tools_hash`。两者没有成为 Tool declaration、内部 Router、缓存和观测共用的权威身份。

TaskSpace Exec 不能在外围再加第三个 hash。TX-02/TX-06 必须定义并实现一个从 canonical effective ToolDefinitions
机械派生的 capability identity，并满足：

- 外层 exec description 中声明的内部 Tool 集合与 nested Router 实际可执行集合来自同一快照；
- Tool 名、kind、input/output schema、namespace/deferred 状态变化才改变身份；
- description 或 schema 实际变化必须可观测，不能用固定版本号掩盖；
- provider wire、cache gate、dispatch trace 和 benchmark 都引用同一身份；
- Standard 不因 TaskSpace identity 改造改变 Tool payload。

### 4.2 I06：任何内部调用都先过同一硬门

当前顶层 `validate_tool_sequence()` 只看到 provider response 的顶层 calls；Code Mode 则新建 nested Router，在执行期直接
调用 `handle_tool_call_with_source()`。如果 TaskSpace Exec 只复制 Code Mode，这些内部调用不会天然经过 TaskSpace
请求级 admission。

因此 I06 不是建设前的旧路径补丁，而是 TX-03/TX-04/TX-08 的设计成败条件：

- Agent source 必须先产生完整 typed plan，而不是执行过程中逐个发现调用；
- preflight 对计划中的 Map 边界、node binding、Tool 身份、revision 和单 Patch 做机械检查；
- preflight 通过后才把 client/map item 还原为原生 ToolCall；
- 原 Tool 参数、权限、sandbox、hook 和 handler 不感知 TaskSpace；
- 任一 Tool 类型必须修改原生参数或绕过 plan 才能工作时，停止方案而不是增加例外。

## 5. 不应提前执行的旧路径工作

- 不继续 I01-W9/W10 的旧三 policy × sibling 协议真实验证；其结果不能证明新 exec final wire。
- 不删除或重写旧 rejection carrier 后再由 TX-10 重做一遍；只保留当前测试作为迁移前反例。
- 不用 prompt 修复 I03，也不让 Runtime 自动补 control、node 或下一动作。
- 不在新 capability identity 建立前晋升 TaskSpace 缓存基线。
- 不根据当前异常 request/token 数字评价 I08。

## 6. 推荐执行顺序

```text
TX-00  修复 I07 已确认的 usage/request 聚合错误
  -> Phase A  TX-01～TX-05
       TX-02 冻结 I10 capability identity 合同
       TX-03/TX-04 冻结 I06 typed plan 与 preflight
  -> Phase B  TX-06～TX-11
       TX-06 接通 I10 单一 catalog identity
       TX-08 接通 I06 统一 admission/dispatch
       TX-10 承接 I01/I02/I05 结果合同
       TX-11 承接 I07 新协议 trace 口径
  -> Phase C  TX-12～TX-13 原子切换并删除旧路径
  -> Phase D  TX-14～TX-16 重验 I01～I08 并更新唯一问题账本
```

该顺序不自动关闭任何 open 问题。它只保证每个问题在不会产生废代码、不会污染新架构的最早阶段获得处理。
