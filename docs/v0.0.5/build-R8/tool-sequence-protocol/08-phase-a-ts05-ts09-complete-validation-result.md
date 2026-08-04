# Phase A TS-05～TS-09 完整验证结果

- Date: 2026-08-05
- Status: Phase A validation completed
- Scope: 旧协议审计、完整容器输入、合法边界、状态正交、无损结果
- Production behavior change: 无；新增代码均为 `#[cfg(test)]` 合同和特征测试
- Whale Agent/API run: 未执行，token 与费用均为 0

## 1. 总结

TS-05～TS-09 能通过本地确定性证据回答的问题已经一次性验证完成。容器方向没有出现新的产品设计歧义：

- Agent 输入只包含响应级 `hosted_node_id`、`client_call` 和 `map_call`；
- 原生 Tool input 作为 JSON object 或 freeform string 原样保留，容器不解释参数；
- Map 只占据前置、读取、后置或终态边界，普通 Work 的依赖不由数组顺序表达；
- Provider 事实、client 结果和 Map 结果分别结算；原始 text/image/error 继续复用现有 Event Store；
- Tool outcome 不应自动决定节点 lifecycle。

剩余问题已经收敛为生产代码中可定位的 blocker，不再需要增加同类 fixture 或重复真实 Provider 探针。

## 2. TS-05：旧协议消费面

当前生产仍完整运行 R7 的 `taskspace_control.actions[] + sibling calls` 协议。以下是正式切换必须替换的生产面：

| 面 | 当前事实 | 处理顺序 |
|---|---|---|
| Agent schema/parser | `taskspace_tool.rs` 与 `taskspace_control_args*` 强制 `actions[]` | 容器 schema 接线后删除 |
| Prompt/version | `taskspace_core_protocol_v3.md` 要求 control-first 和 sibling 对位 | 与生产切换同一提交更新 hash |
| Preflight | `sequence_preflight.rs::match_actions` 按数量、位置、Tool 名二次配对 | 由容器结构预检取代 |
| Executor | `execute_prepared_taskspace_siblings` 剥离 control 后执行 siblings | 由容器 decoder + 原 Router 取代 |
| Control handler | initialize/execute/reopen 经普通 Router 会被旧旁路拒绝 | 恢复为普通 Map Tool handler |
| Hosted | Web/Image 被 `RejectedNative` 拒绝，并使整个响应零 dispatch | Hosted reconciler 接线后原子删除 |

确认不存在的旧设计：`current_node`、`bind_node` Tool、普通 Tool 的 TaskSpace 参数侵入、生产 shadow call。当前
`call_owners` 是结果归属索引，不是 Agent 的单独绑定动作，应保留并改接容器身份。

## 3. TS-06：完整输入合同

冻结的 Agent 输入只有两种 item，加一个可选响应级 Hosted 作用域：

```json
{
  "hosted_node_id": "research",
  "items": [
    {
      "kind": "map_call",
      "item_id": "map-1",
      "tool": "taskspace_control",
      "input": {"action": "execute", "expected_revision": 12}
    },
    {
      "kind": "client_call",
      "item_id": "work-1",
      "node_id": "implement",
      "tool": "apply_patch",
      "input": "*** Begin Patch"
    }
  ]
}
```

Runtime 结算时从真实 Provider 响应生成 `provider_result`，它不是 Agent 输入 item。合同测试确认：

- object 和 freeform input 逐值保留；
- `item_id` 非空且唯一；
- client 的 `node_id` 必填，map_call 禁止伪造节点；
- 容器不能递归，client 不能伪装 `taskspace_control`；
- Agent 不能提交 `provider_result` 或 provider ID；
- 一个容器最多一个 `apply_patch`；
- Hosted-only 容器允许 `items=[]`，但必须有非空 `hosted_node_id`。

## 4. TS-07：合法边界

最终合法形状共七类。前五类是普通推进组合，另有 Hosted-only 和最终关闭两个机械特例：

| 形状 | 结果 |
|---|---:|
| `[hosted_scope]` | PASS |
| `[map_read]` | PASS |
| `[map_terminal]` | PASS |
| `[actions+]` | PASS |
| `[map_prelude, actions+]` | PASS |
| `[actions+, map_epilogue]` | PASS |
| `[map_prelude, actions+, map_epilogue]` | PASS |

负例覆盖 map 中置、非终态 map-only、read 携带 actions、reopen 位于后置、finish 位于前置、空容器、重复/空 ID、递归、
双 Patch、错误 map Tool 和 Agent 伪造 Provider result，全部按预期拒绝。

## 5. TS-08：状态正交

现有真实 DAG 测试证明，Tool success 和 failure 在释放 reservation 后对节点产生相同机械效果：节点都从 `InFlight` 回到
`Ready`，不会自动 Complete 或 Block；结果只以 `is_error` 差异记账。

同时，特征测试坐实当前生产状态机仍违反完整正交合同：当 Agent 在 Tool 尚未结算时显式 Complete 同一节点，现有 invariant
返回 `TransitionInvalid`。源码还禁止 Completed/Blocked 节点保留 reservation，最终 finish 也要求全局 reservation 为空。
这不是模型行为问题，而是正式状态模型的实现 blocker。

## 6. TS-09：结果与 revision

已通过：

| 验证 | 结果 |
|---|---:|
| 原生 text/image body 不经文本投影 round-trip | PASS |
| Event Store 对 text/image/error/Hosted 全字段 round-trip | PASS |
| 结算 manifest 只保存索引，不复制业务正文 | PASS |
| `failed/not_executed/outcome_unknown` 等合同枚举互不混淆 | PASS |
| 成功、不完整、Store unavailable 的唯一 `canonical_revision` | 3/3 PASS |
| item 内夹带第二份 `canonical_revision` | 按预期拒绝 |

需要注意：`FunctionCallOutputPayload.success` 是 Runtime 内部元数据，不进入 Provider wire；权威正文是原生 body，执行状态必须由
结算 manifest 单独保存。当前生产只有 `succeeded: bool` 和 Map `is_error: bool`，尚不足以实现已冻结的状态集合。

## 7. 唯一 Blocking 清单

以下七项是 Phase A 后的唯一已知 blocker。它们都需要生产实现，不会被更多相同类型的验证自然消失：

| ID | Blocking 项 | 根因 | 主要实施单元 |
|---|---|---|---|
| PB-01 | 容器生产入口缺位 | 当前请求仍暴露普通 Tool + control siblings；测试容器未生成 ToolSpec/decoder | TS-10～TS-13、TS-18～TS-19 |
| PB-02 | Client Tool 还原只覆盖 Function/Freeform | MCP、ToolSearch、LocalShell 需要复用各自已有 typed identity | TS-10、TS-14 |
| PB-03 | 旧 actions/sibling 协议仍是主路径 | schema、parser、prompt、preflight、executor、control 旁路相互绑定 | TS-15、TS-19～TS-20 |
| PB-04 | Hosted 事实仍被拒绝 | `RejectedNative` 与 Added/Done 捕获把 Web/Image 视作非法响应 | TS-12、TS-16、TS-19～TS-20 |
| PB-05 | Reservation 与 lifecycle 仍耦合 | Completed/Blocked/finish 被未结算 Tool reservation 阻止 | TS-15～TS-16 |
| PB-06 | Tool outcome 粒度不足 | `bool/is_error` 无法区分 failed、cancelled、not_executed、outcome_unknown | TS-16 |
| PB-07 | 失败反馈复制 Map/revision | 旧 preflight 将同一失败 payload 复制给每个 pairing，再附加 developer fact | TS-16 |

## 8. 非 Blocking 的后续验证

以下事项需要在生产接线后验证，但不阻止 Phase B 开始：

- `tool_choice=auto` 没有协议级容器必达保证；当前真实 Web 探针为 2/2，生产接线后再按预算观测缺失率；
- DeepSeek Image Generation 当前没有自然请求 descriptor，只保留协议 fixture，能力开放后再做真实探针；
- 缓存、请求数、token、Agent 行动质量必须使用正式容器，而不能用测试 schema 代替。

这些项目不应被误写为当前产品 blocker，也不能在没有生产路径时提前消耗 Whale Agent 预算。

## 9. 验证命令

```bash
cargo test -p codex-core taskspace_hosted_binding_contract_tests
cargo test -p codex-core taskspace_sequence_
cargo test -p codex-core sequence_state_contract_tests
cargo test -p codex-core response_items_round_trip_without_field_loss
cargo test -p codex-core finalized_result_
python3 -m unittest scripts/taskspace-benchmark/test_r8_hosted_container_probe.py
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

Phase A 到此结束。下一步不是继续增加验证样例，而是按 PB-01～PB-07 的依赖关系进入 Phase B 未接线生产内核。
