# TS-06 Hosted 响应级归属合同验证

> **已封存证据（2026-08-05）**：响应级单节点归属不是现行方案；fixture 只保留为反例。
>
> **结论回撤（2026-08-06）**：单 `hosted_node_id` 自洽不等于产品合同成立，Root/unbound 降级也不被接受。当前逐项
> 多节点计划见 [`../taskspace-exec/07-a2-multi-node-binding-validation-plan.md`](../taskspace-exec/07-a2-multi-node-binding-validation-plan.md)。

- Date: 2026-08-05
- Status: Hosted 归属片段 verified / production 未接线
- Runtime behavior change: 无
- Whale Agent/API run: 未执行，token 与费用均为 0
- Test seam: `core/src/tools/taskspace_hosted_binding_contract_tests.rs`

## 1. 验证对象

真实 Provider 探针已经证明：DeepSeek Responses 能在同一响应中产生 Hosted Web Search 事实和 TaskSpace 容器，但 Agent
不能可靠回填 Provider 分配的 output item ID。此次 MVT 因而只验证以下最小合同：

```json
{"hosted_node_id":"research"}
```

`hosted_node_id` 是 Agent 对本响应全部 Hosted 事实的唯一节点归属声明。Runtime 从 `web_search_call`、
`image_generation_call` 等原始响应项读取真实 ID、类型和 Provider 状态，并逐项登记到该节点。Agent 不生成、复制或猜测
Provider ID。

该片段最终会并入 `taskspace_tools` 顶层容器，不是独立 Tool，也不是第二套 sibling manifest。

## 2. 机械规则

1. 一个响应最多声明一个非空 `hosted_node_id`。
2. 同一响应中的全部 Hosted output item 使用该声明；成功、失败或其他 Provider 状态不改变归属。
3. Tool 状态只作为 Provider 事实保存，不触发节点 complete、block、reopen 或 finish。
4. 缺少容器或作用域时，每项原始事实保留为 `unbound`，Runtime 不猜测、不丢弃、不重执行。
5. 同一响应中的重复 Provider ID 标记为 `duplicate_in_response`，不重复写入。
6. 同节点 replay 标记为 `already_bound` 并保持幂等；同一 Provider ID 不能改绑到另一节点。
7. 缺少 Provider ID 时标记为 `identity_missing`，不得创建替代 ID。

## 3. 测试结果

执行：

```bash
cargo test -p codex-core taskspace_hosted_binding_contract_tests
```

| Case | 结果 | 证明内容 |
|---|---:|---|
| 作用域 schema | PASS | 只接受一个字符串节点；拒绝节点数组和 Agent 提供的 `provider_item_id` |
| 多 Hosted 事实与混合状态 | PASS | Web/Image、completed/failed 均绑定同一声明节点 |
| 缺失作用域 | PASS | 全部事实保留为 unbound，ledger 不写入 |
| 重复、缺 ID 与 replay | PASS | 分别记录机械状态；重放幂等 |
| 跨节点改绑 | PASS | 冲突被发现，既有绑定不被覆盖 |

总计 `5 passed; 0 failed`。同时运行 Hosted 探针解析单测 `2 passed; 0 failed`，历史探针脚本通过 Python 编译检查。

## 4. 已关闭的风险

- 节点归属不再依赖 Agent 读取 Provider 传输层 ID。
- Runtime 不需要按工具名、数组位置、参数或结果内容猜配。
- 一次业务搜索展开为多个 Hosted output item 时，所有事实仍能逐项保留真实身份和状态。
- 失败 Hosted Tool 不会被误解为节点失败，也不会阻止 Agent 独立声明节点生命周期。
- 缺少容器不会造成事实丢失或暗中重试。

## 5. 明确限制与剩余工作

这是 TS-06 的 Hosted 归属片段，不是完整容器 schema，也没有改变生产行为：

- 同一响应只支持一个 Hosted 节点；多个节点需要 Hosted 工作时必须使用不同响应。
- `client_call`、`map_call` 及完整顶层 schema 仍需在 TS-06 后续 fixture 中冻结。
- 当前生产 `provider_tool_declaration.rs` 中的 `RejectedNative` 仍未删除；应在 TS-12/TS-19/TS-20 按原子切换计划处理，
  不能在本 MVT 中提前形成半接线状态。
- `tool_choice=auto` 仍不提供容器必达的 Provider 硬保证；缺失率需要后续获批真实产品样本观测。
- MCP、ToolSearch、LocalShell 等 client Tool 的容器兼容性属于 Phase B，不由本测试代替。

因此，Hosted 节点归属不再是 Phase A 的设计阻塞。TS-06 其余 schema 和 TS-07～TS-09 已在后续一次性验证中完成，详见
[`08-phase-a-ts05-ts09-complete-validation-result.md`](08-phase-a-ts05-ts09-complete-validation-result.md)。
