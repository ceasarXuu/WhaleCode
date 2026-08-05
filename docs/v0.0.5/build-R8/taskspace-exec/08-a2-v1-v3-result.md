# A2 V1～V3 离线验证结果（v3）

- Date: 2026-08-06
- Status: verified-isolated
- Production activation: 无
- Real Whale Agent/API runs: 0

## 1. 结论

A2 的逐项关联合同收敛为：Agent 在 `hosted_bindings[]` 中按 Provider 原始 `output_index` 顺序，为每个 Hosted output
item 声明 `{tool,node_ids[]}`；节点集合必须非空且无重复。Runtime 从原始 response item 读取真实 `id/item_id`，按
`output_index` 排序后逐项核对。

这不是语义匹配：Runtime 只核对索引、数量、Hosted Tool 类型、Provider 身份和节点集合结构。一个事实服务哪些节点，
完全以 Agent 的声明为准；Runtime 不判断业务语义是否正确，也不为每个节点复制 Provider fact。

## 2. V1：wire 证据

| 事实 | 证据 | 结论 |
|---|---|---|
| Provider Hosted item 有真实 `id/item_id` | 历史 DeepSeek probe、Responses decoder fixtures | Runtime 读取，不要求 Agent 回显 |
| Streaming event 有 `output_index` | OpenAI Responses streaming contract | 它是响应 output 的结构顺序身份 |
| 当前 Whale decoder 丢弃 `output_index` | `codex-api/src/sse/responses.rs::ResponsesStreamEvent` 与 `process_responses_event` | TX-07 必须补传，不能使用 done 到达顺序 |
| done 事件可能与 output 顺序不同 | Provider 并行 Tool 的流式完成模型 | reconciler 输入显式要求 `(output_index, ResponseItem)` 并先排序 |

官方依据：[OpenAI Responses streaming events](https://platform.openai.com/docs/api-reference/responses-streaming/response/refusal/delta?lang=curl)、
[DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls)、
[DeepSeek Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode)。

## 3. V2：Agent 合同

首次 v2 把每项事实限制为单个 `node_id`，与“一个事实可服务多个节点”的产品语义冲突。候选计划现从 v2 直接升级为
`taskspace_exec_plan_v3`，不保留兼容：

```json
{
  "hosted_bindings": [
    {"tool": "web_search", "node_ids": ["research-a", "compare"]},
    {"tool": "web_search", "node_ids": ["research-b"]},
    {"tool": "image_generation", "node_ids": ["design"]}
  ]
}
```

响应级 `hosted_node_id`、逐项单 `node_id` 和 Agent 回显 Provider ID 的 `hosted_records[]` 都由严格 decoder 拒绝。
普通 Provider Tool 和 client Tool schema 均未修改。

## 4. V3：原子拒绝

`provider_reconcile` 只有两种结果：

1. 全部事实一一对应：返回完整 bindings；
2. 任一数量、类型、Provider ID 或 output index 异常：`exact=false` 且 `bindings=[]`。

因此候选模块不再产生 partial binding、`unbound` settlement 或默认 Root owner。当前模块尚未接生产 Router、Map 或
Event Store；完整“零 dispatch、零 commit、零 Store 写入”必须在 TX-09/TX-11/TX-12/TX-17 接线时再次证明，不能用
离线纯函数测试冒充生产事务完成。

## 5. 验证

```text
cargo test -p codex-core taskspace_exec --lib --quiet
32 passed; 0 failed
```

覆盖包括 strict decode、旧单 owner 字段拒绝、空/重复节点、多 owner 单事实、同类多项、乱序 done 后按 output index
恢复、少声明、多声明、Tool 顺序错配、缺失/重复 Provider ID、重复 output index，以及任一错误返回空 bindings。

## 6. 剩余门禁

A2-V4 已执行两轮独立真实 probe，均按首败停止。首轮见 `09-a2-v4-first-probe-result.md`；修正后复验见
`10-a2-v4-v3-reprobe-result.md`。第二轮已排除 Hosted 动作不可见，但证明 `{source:string}` Function Schema 无法机械
承载 source 内层硬合同。在合同承载层重新设计并验证前，不再通过追加提示词或付费重试掩盖；A2 与 Phase A
都保持未完成。

专用探针 `scripts/taskspace-benchmark/r8_taskspace_exec_a2_probe.py` 已完成，并在现有 benchmark Docker 镜像中通过 7/7
离线自检。它从原始 SSE 读取 `output_index` 和 Provider ID，检查唯一 outer call、完整 v3 plan、逐项数量和形状、两个
节点均被声明，以及 outer call 位于 Hosted 工作之后。探针不会按搜索内容替 Runtime 判断节点语义，也不会自动重试。
首次运行前探针已通过本节所述离线自检；实际运行记录、费用和后续修正以 `09-a2-v4-first-probe-result.md` 为准。
