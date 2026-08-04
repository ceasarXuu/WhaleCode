# Hosted Tool 与容器真实 Provider 探针结果

- Date: 2026-08-05
- Record: `WAR-20260805-005841-R8-HOSTED-PROBE-DCF750E2`
- Model: `deepseek-v4-flash`
- Wire: `POST https://api.deepseek.com/responses`
- Scope: 1 个合成能力样本，1 arm，repeat=2，禁止自动重试
- Result artifact: `benchmarks/taskspace/r8/evidence/WAR-20260805-005841-R8-HOSTED-PROBE-DCF750E2.json`

## 1. 验证目标

请求只暴露两个 Tool：

1. Provider 原生 `web_search`；
2. 最薄的 client function `taskspace_probe(node_id, provider_item_id)`。

Agent 被要求先使用实时 Web Search，再在同一个 Response 中调用 `taskspace_probe`，声明固定节点
`research-node`，并把真实 `web_search_call` output item ID 填入 `provider_item_id`。该探针不执行 client Tool，
不接入生产 TaskSpace Runtime，也不测试任务质量；它只观察 Provider wire 和模型可见性。

## 2. 结果

| 指标 | Repeat 1 | Repeat 2 | 合计 |
|---|---:|---:|---:|
| HTTP | 200 | 200 | 2/2 协议可用 |
| Hosted Web output item | 5 | 1 | 6 |
| 容器 function call | 1 | 1 | 2 |
| Hosted 与容器同响应共存 | 是 | 是 | 2/2 |
| `node_id=research-node` | 正确 | 正确 | 2/2 |
| Provider item ID 精确回显 | 失败 | 失败 | 0/2 |
| Agent 实际填入 | Pricing 页面 URL | Pricing 页面 URL | 均非 Provider ID |
| Input token | 11,119 | 3,433 | 14,552 |
| Cached input | 7,424 | 1,408 | 8,832 |
| Output token | 1,325 | 2,105 | 3,430 |
| Provider elapsed | 15.666s | 21.574s | 37.240s |

按 2026-08-05 DeepSeek V4 Flash 价格估算，本轮费用为 `¥0.01275664`。账本和 result artifact 已完整结算。

## 3. Trace 事实

Repeat 1 的 Hosted Tool 并非一次调用，而是五个独立 output item：一个初始搜索、两个打开页面、一个页内查找和一个补充
搜索，其中三个状态为 `failed`、两个为 `completed`。它们分别拥有唯一 `call_00...call_04` ID。容器位于这些 Hosted
事实之后，声明了正确节点，但把页面 URL 当成了 `provider_item_id`。

Repeat 2 只有一个 `web_search_call`，随后同样产生容器；节点仍正确，ID 字段仍被填成页面 URL。由此可以排除“第一轮
调用过多导致偶发抄错”的单一解释。

首次启动曾在联网前因本机 Python 缺少 HTTPS handler 失败。Runner 随后改为通过临时 `0600` curl config 承载认证头；
该本地失败没有发出 HTTP 请求，也没有消耗 Provider 预算。正式两轮均无重试。

## 4. 已关闭与未关闭的判断

### 4.1 已关闭

- DeepSeek Responses 路径接受原生 Hosted Web Search 与 client function 同时暴露；
- 模型能够在 Hosted Tool 结束后，于同一 Response 继续生成容器；
- Agent 能稳定声明该批 Hosted 工作所属的 TaskSpace 节点；
- Runtime 能从 Provider output item 获得每次 Hosted 调用的唯一真实 ID。

### 4.2 未成立

- 不能要求 Agent 回显 Provider output item ID；明确要求后仍 0/2 失败；
- `tool_choice=auto` 的 2/2 成功是能力证据，不是“容器必达”的 Provider 硬保证；
- 一个业务搜索动作可能展开成多个 Hosted output item，不能假设“一项节点工作对应一次 Hosted Tool 调用”。

## 5. 对 TS-06 的约束

TS-06 不得再设计 `provider_result_ref.response_item_id` 让 Agent 逐项填写。Provider ID 应仅由 Runtime 从响应事实中取得并
保存。剩余的最简候选是“响应级 Hosted 节点归属”：Agent 在容器中只声明一个 `hosted_node_id`，Runtime 将同一 Response
中观察到的全部 Hosted output item ID 机械登记到该节点。

该候选没有内容匹配、ordinal、自造调用 ID 或 Agent 回显，但会形成一个明确产品限制：**同一 Response 的 Hosted Tool
事实只能归属一个节点**。多个节点分别需要 Hosted 工作时必须拆成不同 Response。这个限制影响并行性和请求成本，必须由
用户确认后才能冻结 schema 和本地正反 fixture。

若实际响应缺少容器，Provider 原始事实必须保留为 unbound；Runtime 不能丢弃、重执行或推断节点。后续是否允许 Agent
使用 Runtime 暴露的真实 ID 补绑定，属于恢复合同，不应掩盖正常路径的容器缺失率。

## 6. 外部合同

- [OpenAI Responses streaming events](https://platform.openai.com/docs/api-reference/responses-streaming/response/refusal/delta?lang=curl)：
  Hosted output item 提供唯一 `item_id`，`required` 只保证调用一个或多个允许的 Tool。
- [DeepSeek Models & Pricing](https://api-docs.deepseek.com/zh-cn/quick_start/pricing)：本轮模型能力和费用估算依据。
- [DeepSeek Claude Code integration](https://api-docs.deepseek.com/quick_start/agent_integrations/claude_code)：DeepSeek Provider
  执行 Web Search 可能产生额外模型请求和 token 成本。
