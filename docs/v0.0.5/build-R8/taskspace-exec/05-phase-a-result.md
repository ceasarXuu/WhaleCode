# TaskSpace Exec Phase A 结果

- Created: 2026-08-05
- Corrected: 2026-08-06 / A2 reopened
- Status: Incomplete / A1 passed / A2 in progress
- Runtime activation: None
- Paid Whale Agent runs in revalidation: 0

## 1. 结果总览

| Unit | 结果 | 证据 | 结论 |
|---|---|---|---|
| TX-01 | 通过 | `9ecbbf257`、[`04-phase-a-discovery.md`](04-phase-a-discovery.md) | 当前生产、上游 seam 和旧协议删除范围已固定 |
| TX-02 | 通过 | `2cd2a26b2`、5 个 catalog tests | Function 外壳和 capability identity 可由同一 ToolSpec 快照派生 |
| TX-03 | A1 通过 | `5c32b092f`、decoder tests | 严格声明式 source 在副作用前形成唯一 typed plan |
| TX-04 | 通过 | `f734b68a1`、preflight tests | 结构、Tool/input、node、Map 边界、递归和单 Patch 可机械判断 |
| TX-05 | A2 未完成 | 历史 Provider 探针、协议/Store 回归、单节点 reconciliation tests | 只证明 Runtime 可读取 Provider `id/item_id`；未证明逐项多节点绑定和整批拒绝 |

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

## 3. A2：保留证据与失效结论

首次实现错误地要求 Agent 在 `hosted_records[]` 中重复填写：

```text
(response_id, provider_item_type, provider_item_id, node_id)
```

这与既有证据冲突。Provider response 已为每个 Hosted output item 返回唯一 `id/item_id`，Runtime 能直接读取、持久化和
replay；真实探针也证明 Agent 能稳定声明所属节点，但不能可靠复制 Provider 的传输层 ID。重复填写不是更强约束，而是
制造第二份可能冲突的身份事实。

第二版原型将 Agent 合同缩成：

```json
{"hosted_node_id":"research"}
```

Runtime 从同一响应中的 `web_search_call`、`image_generation_call` 等原始事实直接读取 `id/item_id` 和状态，把全部事实
机械登记到一个节点。该原型证明 Agent 不必复制 Provider ID，但只支持响应级单节点归属。

该限制不是 TaskSpace 的产品规则。一个响应可以推进多个活跃节点，其中不同 Hosted 动作也可能归属不同节点；强制拆分
响应会错误地把 Provider 传输边界变成 Map 边界。原型还把缺失或冲突绑定降级为 Root/unbound，等于接受不完整归属。
这两项结论均已撤销。

## 4. A2 证据结论

| 证据 | 结果 | 证明内容 |
|---|---:|---|
| DeepSeek Hosted 探针 repeat=2 | PASS | 同响应 Hosted + Function 共存 2/2；节点声明正确 2/2 |
| Provider 原始 output | PASS | 每项 Web Search 均有唯一 `call_...` ID |
| Agent Provider ID 回显 | 0/2，已移出合同 | 证明 Agent 不应承担传输身份复写 |
| Responses replay identity | PASS | Web/Image ID 能进入下一请求 |
| TaskSpace Event Store round trip | PASS | Web/Image 原始字段和 Provider ID 可恢复 |
| 单节点 TaskSpace Exec tests | PASS，但覆盖不足 | 只证明单值 `hosted_node_id` 原型有确定结果，不证明产品合同成立 |

因此，Provider 身份可由 Runtime 读取这一事实仍成立；“整响应单节点足够”和“异常可作为 unbound/Root 结算”不成立。
A2 已回撤，必须补证逐项声明、同响应多节点、唯一机械关联和整批拒绝。

## 5. 后续计划

1. 先执行 [`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md) 的 A2-V1～V4。
2. 只有逐项关联可唯一核对、失败整批拒绝且目标模型可生成时，TX-05 才恢复为 `verified-isolated`。
3. Phase A 完成前不进入 TX-06，不把单节点原型带入 response lifecycle、Map Store 或生产投影。

## 6. 验证

本次状态回撤没有发起新的 Provider 请求。既有测试和真实探针仍作为部分证据保留，但不能证明多节点合同。历史详情见
[`06-a2-revalidation-result.md`](06-a2-revalidation-result.md)，新验证计划见
[`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md)。
