# A2 Hosted 绑定既有证据与回撤结果

- Date: 2026-08-06
- Status: Reopened / partial evidence only
- Scope: TaskSpace Exec Phase A / TX-05
- Production behavior change: 无，模块尚未注册
- New Whale Agent/API runs: 0

## 1. 重新验证的问题

A2 原先只回答：Provider-hosted Tool 已经执行并返回结果后，TaskSpace 是否能在不猜测、不重执行、不要求 Agent 复制
传输身份的前提下，把整批真实结果绑定到一个 Agent 声明的 Map 节点。这个问题范围过窄，遗漏了同响应逐项多节点归属。

验收合同是：

1. Provider output item 自带稳定 `id/item_id`，Runtime 原样读取；
2. Agent 只声明同一响应 Hosted 事实所属的单值 `hosted_node_id`；
3. Runtime 把同响应中的每个真实 Hosted 事实机械登记到该节点；
4. 缺节点声明、缺 Provider ID 和重复 Provider ID 被标记为未绑定；
5. Provider Tool 成败不自动改变节点生命周期。

## 2. 被否定的旧假设

旧 A2 原型要求 Agent 填写 `response_id`、`provider_item_type`、`provider_item_id` 和 `node_id`，再由 Runtime 做 exact-set
匹配。该设计把 Provider 已经返回的唯一身份复制成第二份 Agent 声明，违反单一事实源，也与 2026-08-05 的真实探针
结果冲突：Agent 节点声明 2/2 正确，Provider ID 回显 0/2。

`response_id` 也不应成为 Agent 绑定字段。当前响应边界由 Runtime 已知，持久身份使用 Provider output item 自带的唯一
ID；节点归属来自 Agent。两者无需被 Agent 拼成一份复合 key。

## 3. 当时的实施变化

| 位置 | 变化 |
|---|---|
| `taskspace_exec/plan.rs` | 删除 `hosted_records[]`，改为可选单值 `hosted_node_id` |
| `taskspace_exec/decoder.rs` | 接受节点声明；明确拒绝 Agent 重新声明 Provider 传输身份 |
| `taskspace_exec/preflight.rs` | 校验 Hosted 节点非空；允许 Hosted-only 合法计划 |
| `taskspace_exec/provider_reconcile.rs` | 直接从 `ResponseItem` 读取 Web/Image ID、类型和状态，再绑定到声明节点 |
| `tools/taskspace_hosted_binding_contract_tests.rs` | 删除被新模块测试覆盖的旧独立原型，避免两份实现长期并存 |

## 4. 证据链

| Evidence | Result | Interpretation |
|---|---:|---|
| `WAR-20260805-005841-R8-HOSTED-PROBE-DCF750E2` | PASS | Provider 返回唯一 `call_...` ID；Agent 同响应正确声明节点 |
| `hosted_item_ids_survive_responses_input_replay` | PASS | Provider ID 在 Responses replay 中不丢失 |
| `response_items_round_trip_without_field_loss` | PASS | TaskSpace Event Store 保留 Web/Image 原始身份和结果 |
| `agent_declares_one_node_while_runtime_reuses_every_provider_identity` | PASS | mixed Web/Image、completed/failed 均使用真实 ID 绑定同一节点 |
| 缺节点、缺 ID、重复 ID、无事实声明测试 | PASS | 异常均机械暴露，没有内容、顺序或 URL 猜配 |
| Agent 传输身份重声明测试 | PASS | 旧 `hosted_records` 字段被严格 schema 拒绝 |

## 5. 验证命令

```bash
cargo test -p codex-core taskspace_exec --lib --quiet
cargo test -p codex-api hosted_item_ids_survive_responses_input_replay --quiet
cargo test -p codex-core response_items_round_trip_without_field_loss --lib --quiet
python3 -m unittest scripts/taskspace-benchmark/test_r8_hosted_container_probe.py
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

## 6. 回撤结论

以下结论继续成立：Provider Tool 的真实结果带有 Runtime 可直接复用的身份；Agent 不应复制或创造 Provider 传输 ID；
Tool outcome 不决定节点生命周期。

以下结论已失效：同一响应的 Hosted 事实只能归属一个节点；多个节点必须拆响应；缺失或冲突绑定可以写为 Root/unbound
后结束处理。它们分别违背多活跃节点模型和“所有工作动作必须有 Agent 声明归属”的硬约束。

A2 因此从 `passed` 回撤为 `in-progress`。Phase B 重新阻断，后续按
[`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md) 验证逐项多节点声明、唯一机械
关联和整批拒绝语义。
