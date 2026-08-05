# TaskSpace Exec Phase A 结果

- Created: 2026-08-05
- Corrected: 2026-08-06
- Status: Completed / A1+A2 passed
- Runtime activation: None
- Paid Whale Agent runs in revalidation: 0

## 1. 结果总览

| Unit | 结果 | 证据 | 结论 |
|---|---|---|---|
| TX-01 | 通过 | `9ecbbf257`、[`04-phase-a-discovery.md`](04-phase-a-discovery.md) | 当前生产、上游 seam 和旧协议删除范围已固定 |
| TX-02 | 通过 | `2cd2a26b2`、5 个 catalog tests | Function 外壳和 capability identity 可由同一 ToolSpec 快照派生 |
| TX-03 | A1 通过 | `5c32b092f`、decoder tests | 严格声明式 source 在副作用前形成唯一 typed plan |
| TX-04 | 通过 | `f734b68a1`、preflight tests | 结构、Tool/input、node、Map 边界、递归和单 Patch 可机械判断 |
| TX-05 | A2 通过 | 历史 Provider 探针、协议/Store 回归、修正后的 reconciliation tests | Agent 只声明节点，Runtime 直接复用 Provider `id/item_id` |

新增 TaskSpace Exec 代码仍位于未注册的 `core/src/tools/taskspace_exec/`，没有 handler、Router 注册、请求投影或 provider
payload 改动。Standard 和当前 TaskSpace 生产行为均未改变。

## 2. A1：source 与预检

Phase A 使用单一声明式 wrapper：

```text
taskspace.plan(<strict JSON>);
```

JSON 一次表达版本、capability identity、client/map call、原生输入和 Agent 声明的节点归属。变量、条件、循环、`await`、
Markdown fence、未知字段和尾随语句均被拒绝。Runtime 不解析 reasoning，也不会边执行边发现后续计划非法。

当前 preflight 只判断纯输入硬合同。节点存在性、revision、DAG 和状态转换在 Phase B 调用 canonical Map validator，不能在
TaskSpace Exec 中复制第二套状态机。

## 3. A2：纠偏后的 Hosted 合同

首次实现错误地要求 Agent 在 `hosted_records[]` 中重复填写：

```text
(response_id, provider_item_type, provider_item_id, node_id)
```

这与既有证据冲突。Provider response 已为每个 Hosted output item 返回唯一 `id/item_id`，Runtime 能直接读取、持久化和
replay；真实探针也证明 Agent 能稳定声明所属节点，但不能可靠复制 Provider 的传输层 ID。重复填写不是更强约束，而是
制造第二份可能冲突的身份事实。

修正后的唯一 Agent 合同是：

```json
{"hosted_node_id":"research"}
```

Runtime 从同一响应中的 `web_search_call`、`image_generation_call` 等原始事实直接读取 `id/item_id` 和状态，把每项事实
机械登记到该节点。Agent 不读取、复制或创造 Provider ID；Tool 状态不改变节点状态。

该合同当前明确限制同一响应内的 Hosted 事实归属一个节点。它是已经验证的最简能力边界，不通过顺序、URL、内容或
语义相似度扩展。需要多个节点分别执行 Hosted 工作时拆分响应。

## 4. A2 证据结论

| 证据 | 结果 | 证明内容 |
|---|---:|---|
| DeepSeek Hosted 探针 repeat=2 | PASS | 同响应 Hosted + Function 共存 2/2；节点声明正确 2/2 |
| Provider 原始 output | PASS | 每项 Web Search 均有唯一 `call_...` ID |
| Agent Provider ID 回显 | 0/2，已移出合同 | 证明 Agent 不应承担传输身份复写 |
| Responses replay identity | PASS | Web/Image ID 能进入下一请求 |
| TaskSpace Event Store round trip | PASS | Web/Image 原始字段和 Provider ID 可恢复 |
| 修正后 TaskSpace Exec tests | PASS | mixed 状态、缺节点、缺 ID、重复 ID、无事实声明均有确定结果 |

因此 A2 的问题不是 Provider 结果不可绑定，而是首次 TaskSpace Exec 原型把已经存在的 Provider 身份错误地要求 Agent
再次声明。该错误合同已删除，A2 通过。

## 5. 后续计划

1. TX-06 将 Phase A catalog 接到唯一 effective ToolSpec snapshot，不在旧 `spec.rs` 建长期平行 catalog。
2. TX-07/08 建立未接线 handler，并让 client/map 调用继续经过原 Router 和 canonical Map validator。
3. TX-09 把响应级 Hosted collector 接入真实 response lifecycle，使用 `hosted_node_id + Provider item ID` 结算。
4. TX-10/11 建立唯一结果和可复算日志；不新增 developer factual carrier。
5. TX-12/13 才执行生产投影原子切换和旧协议删除。

## 6. 验证

本次 A2 复验没有发起新的 Provider 请求。此前真实探针已经同时包含 Provider ID 和 Agent 节点声明，重复付费运行不会增加
机制证据；本次只修正合同并重放确定性测试。完整命令与结果见
[`06-a2-revalidation-result.md`](06-a2-revalidation-result.md)。
