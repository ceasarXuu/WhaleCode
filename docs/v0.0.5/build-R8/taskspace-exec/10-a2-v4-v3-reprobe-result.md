# A2 V4 v3 复验结果

- Date: 2026-08-06
- Record: `WAR-20260806-025727-R8-A2-V4-HOSTED-BINDING-002`
- Status: failed as specified / contract carrier blocked / A2 remains blocked
- Requests: 1 of 2 approved; automatic retries 0

## 1. 结果

修正后的 DeepSeek Responses 请求成功。Provider 返回 6 个身份和 `output_index` 都有效的
`web_search_call`，Agent 在所有 Hosted 动作之后调用了唯一 `taskspace_exec`。

| 检查项 | 实际值 | 判定 |
|---|---:|---|
| HTTP / Responses protocol | 200 / completed | 通过 |
| Provider Hosted facts | 6 | 有效 |
| outer `taskspace_exec` | 1，位于 output index 16 | 通过 |
| `version` / `capability_id` | v3 / 正确能力 ID | 通过 |
| binding Tool / `node_ids[]` | canonical `web_search` / 非空且覆盖双节点 | 通过 |
| Hosted bindings | 2 | 失败：比 6 个真实 facts 少 4 项 |
| `calls` | 2 个 Provider 搜索日志对象 | 失败：不是 client Tool call |

按既定原子门禁，Runtime 必须拒绝整个计划，不能接受两项部分绑定。探针依停止条件没有发送第二次请求。

## 2. 可见性证据

这一轮可以排除“Agent 没看到 Provider 内部动作”：

1. output index 2 和 9 是两次 completed `search`；
2. output index 5、6、12、13 是四次 failed `open_page`；
3. 每组动作之间的 Agent message 明确说明正在打开页面、页面访问失败，以及因失败而继续搜索。

因此 Agent 实际知道 6 项 Hosted 动作及其成败。它只声明两次 completed search，是合同生成问题，不是反馈丢失。

## 3. 阻塞原因与归因边界

Provider 真正机械校验的 Function 参数只有：

```json
{
  "source": "string"
}
```

`version`、`capability_id`、`calls` 和 `hosted_bindings` 都被编码在 `source` 字符串内。Provider JSON Schema 无法对它们的字段、类型、完整性或
数量施加结构约束，所有硬合同实际上仍只由 Tool description 表达。

本轮有两层归因：

1. 直接近因：专用 probe 的 Tool description 没有明说 `calls` 只用于 client Tool，且本例必须为空。因此 `calls`
   误用本身不能归因为模型无法遵循完整协议，它暴露了探针与真实 catalog 合同不完全同构。
2. 系统性阻塞：Tool description 已明说 bindings 必须覆盖每个实际 `web_search_call`，包括 failed 和所有
   action subtype；Agent 也确实看到四次 failed `open_page`，但仍未声明。这证明 source 内层完整性只靠文本说明无法作为
   可验收的硬合同，但不证明换成结构化 Function Schema 后模型必然通过。

这个阻塞不能靠 Runtime 事后补绑、语义猜配或默认 Root 处理，也不应通过继续增加提示词来伪装成硬合同。

## 4. 影响与停点

1. v3 typed plan 和 Runtime 原子拒绝逻辑的离线正确性仍成立；
2. 当前 outer Function 合同承载方式不能稳定让 Agent 生成该 plan，因此 A2 和 Phase A 保持阻塞；
3. 不再启动第三次提示词式 probe；下一步必须先设计让完整 TaskSpace plan 进入 Provider Function Schema 可机械约束层的方案；
4. 新方案不得修改普通 Tool 原生 schema，不得让 Runtime 替 Agent 选节点，不得建立第二套 Tool 合同或语义匹配层。

## 5. 成本与证据

| 指标 | 实际值 |
|---|---:|
| API requests | 1 |
| Input tokens | 16,838 |
| Cached input | 11,648 |
| Uncached input | 5,190 |
| Output tokens | 2,556 |
| Cache hit / input | 69.18% |
| Elapsed | 20.962s |
| Estimated cost | ¥0.01053496 |

结果文件：`benchmarks/taskspace/r8/evidence/WAR-20260806-025727-R8-A2-V4-HOSTED-BINDING-002.json`。原始 request/SSE 位于同名
`target/provider-probes/` 目录，不提交敏感或大体积 raw artifact。
