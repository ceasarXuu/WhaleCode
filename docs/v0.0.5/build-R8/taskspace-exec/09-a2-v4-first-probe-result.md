# A2 V4 首次真实 Probe 结果

- Date: 2026-08-06
- Record: `WAR-20260806-021153-R8-A2-V4-HOSTED-BINDING-001`
- Status: failed as specified / result confounded and exposed single-owner contract defect / A2 remains blocked
- Requests: 1 of 2 approved; automatic retries 0

## 1. 事实结论

DeepSeek Responses 请求成功，Provider Hosted Web Search 与唯一 `taskspace_exec` outer Function Call 在同一响应共存。Runtime
从原始 SSE 读取到 7 个身份和 `output_index` 均有效的 `web_search_call`，outer call 位于全部 Hosted items 之后，Agent
也在声明中覆盖了两个目标节点。

但是 Agent 只生成了 4 个 bindings，并产生两类 schema 偏差：

| 检查项 | 真实结果 | 判定 |
|---|---:|---|
| HTTP / Responses protocol | 200 / completed | 通过 |
| Provider Hosted facts | 7 | 有效 |
| outer `taskspace_exec` | 1，位于 output index 18 | 通过 |
| Hosted bindings | 4 | 失败：数量不完整 |
| capability 字段 | `capability` | 失败：应为 `capability_id` |
| binding Tool 名 | `search` | 失败：应为 canonical `web_search` |
| 两个节点均出现 | 是 | 通过，但不足以抵消完整性失败 |

Runtime 必须按合同整批拒绝该响应；不得保留四项部分绑定、默认落 Root 或记为 unbound 后继续。因此 A2-V4 未通过，
第二次请求依据预先声明的停止条件没有执行。

## 2. 不能直接归因给模型的混杂因素

首次 probe 同时暴露两项测试/合同缺陷，导致结果不足以证明方案不可行：

1. 探针指令要求“不超过四个 Hosted Web Search item”。Provider 实际将 search、open_page 等行为拆成 7 个独立
   `web_search_call`，而 Agent 恰好声明 4 项。固定数量提示可能把 Agent 锚定在四项，不能把 7/4 偏差全部解释为模型
   无法逐项绑定。
2. 候选 Tool 描述只写了“capability identity”和“Hosted Tool name”，没有明确字段必须叫 `capability_id`，也没有明确
   `web_search_call` 的 search/open_page/find_in_page 等 action subtype 均映射为 canonical Tool 名 `web_search`。Agent
   生成 `capability` 和 `search` 与这两个暴露缺口直接一致。
3. v2 候选把每项 Hosted fact 限制为单个 `node_id`。真实 output index 1 的一次 search 同时查询 DeepSeek 和 OpenAI，
   说明一个 Provider fact 确实可能同时服务多个节点；单 owner 合同本身不满足已确认的产品语义。

所以本轮唯一严谨结论是：**v2 合同下 A2-V4 失败且合同本身不完整；尚不能判断修正后的 v3 多 owner 合同是否可被
DeepSeek 稳定执行。**

## 3. 已完成的离线修正

1. 删除任何预设 Hosted item 数量的自然语言限制，只保留 `max_output_tokens=6000` 的机械费用边界；
2. 计划升级为 `taskspace_exec_plan_v3`，每项 binding 使用非空、无重复的 `node_ids[]`，允许一个事实服务多个节点；
3. Tool 合同明确 source 的四个唯一字段：`version`、`capability_id`、`calls`、`hosted_bindings`；
4. 明确每个实际 `web_search_call` 都必须有一项 binding，包括失败状态和所有 action subtype；
5. 明确 binding 的 Tool 始终为 `web_search`，不得写 `search`、`open_page` 等 action subtype；
6. Docker probe 新增 repo owner 前置门禁，要求以宿主 UID/GID 运行，防止原子写把账本和证据变为 root `0600` 文件。
7. 修正后的系统 instructions 只描述 TaskSpace 工作目标和调用时机；字段名、完整性和 Tool 映射只由
   `taskspace_exec` Tool contract 暴露，避免多层重复提示掩盖 Tool 合同是否有效。

这些修正只改善合同忠实暴露和测试有效性，不增加 Runtime 语义判断，不改变 Agent 的节点选择权，也不放宽整批拒绝。

## 4. 成本与证据

| 指标 | 实际值 |
|---|---:|
| API requests | 1 |
| Input tokens | 20,832 |
| Cached input | 16,640 |
| Uncached input | 4,192 |
| Output tokens | 3,557 |
| Cache hit / input | 79.88% |
| Elapsed | 36.35s |
| Estimated cost | ¥0.0116388 |

结果文件：`benchmarks/taskspace/r8/evidence/WAR-20260806-021153-R8-A2-V4-HOSTED-BINDING-001.json`。
原始 request/SSE 位于同名 `target/provider-probes/` 目录，不提交敏感或大体积 raw artifact。

## 5. 复验门禁

修正后的 probe 必须重新通过离线测试、Rust 合同测试和缓存门禁，再单独申请真实预算。复验仍采用真实双子任务，不在用户
任务中填写预期 plan 或 binding 数量。只有 v3 计划字段严格正确、全部 Provider facts 数量一致、每项 canonical Tool
正确、每项 `node_ids[]` 非空且两个节点均由 Agent 声明、连续 repeat 全部通过时，A2-V4 才能完成。
