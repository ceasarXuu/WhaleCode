# TaskSpace E2E 设计修正：拟真用户、系统并行、图关系校验

## 背景

上一版 E2E 能证明 Whale 在真实 `whale exec --taskspace` 路径下可以创建多个 node、调用 `spawn_agent`、写回 node result、运行真实测试。但它有两个关键证据缺口：

1. 用户提示里出现过“可以并行调查 / parallelize”之类暗示，污染了拟真用户路径。真实用户只会描述项目问题，不会指导 agent 是否并行。
2. 报告强调 node 数量和类型，缺少对 edge、依赖顺序、节点关系的硬校验。多个 node 不等于健康的 task map。

这次修正的目标是把并行调查从“用户鼓励”改为“Whale 系统性鼓励”，并把 E2E 从“数节点”升级为“验证图结构是否合理生长”。

## 修正原则

- 用户输入只表达真实项目目标、业务规则、验收标准和交付期待。
- 用户输入不得出现 `taskspace`、`map`、`node`、`subagent`、`spawn_agent`、`parallel`、`parallelize`、`concurrent` 等内部概念或协作策略词。
- 并行调查必须由 TaskSpace developer context 和 runtime 约束共同推动：当任务自然拆成多个独立调查轨道时，主 agent 应主动创建多个 `inspect_code_context` node 并委派 subagent。
- E2E 不再只数 node，还必须验证图结构：
  - 至少两个 subagent-owned inspect 轨道。
  - 并行 inspect 轨道之间不能互相依赖。
  - implementation node 必须直接依赖这些调查轨道。
  - test / validation node 必须直接依赖 implementation node。
  - 每条 dependency edge 的下游工作时间不能早于上游完成时间。
  - 最终回答必须写回并完成当前 `final_synthesis` node。

## 当前落地

- `scripts/run-action-map-natural-multi-agent-e2e.ps1`
  - 用户 prompt 不再提示并行。
  - prompt guard 禁止 `taskspace/map/node/subagent/spawn_agent/parallel/parallelize/concurrent/simultaneously/delegate/fan out/multiple agents` 等内部概念或协作策略词。
  - 以真实 Whale CLI、真实 sandbox repo、真实工具调用、真实 subagent、真实 `pytest` 和隐藏 oracle 验证。
  - 验证至少两个子 agent 结果写入 node，且图边和执行顺序健康。

- `scripts/run-action-map-growth-health-e2e.ps1`
  - 同样使用自然用户输入，不暴露内部概念。
  - 强校验 graph health：edge 数量、edge 顺序、并行 inspect 轨道、直接依赖、测试依赖、最终节点收束。
  - 不再要求特定标题如 parser/pricing 必须承载 subagent；测试关注更本质的图约束，避免把合理的 implementation/test 分轨误判为失败。

- `scripts/export-action-map-observability.ps1`
  - 从 snapshot 导出 `edges`。
  - Markdown/HTML 报告展示 edge 表，方便人工检查图关系。

- `scripts/action-map-graph-health-lib.ps1`
  - 提供 E2E 复用的图健康检查。
  - implementation 依赖校验锚定到能走到 validation/final 链路的 implementation node，避免无关 dead-end implementation 误报通过。
  - `blocked` leaf 视为已记录结果的终态，不再误判为 open leaf；open leaf 只统计 `pending/ready/running`。

- `scripts/action-map-observability-lib.ps1`
  - snapshot 后补 result body 时保留最早的 node_result_recorded 时间戳，避免后续 snapshot 覆盖真实发生时间。

- `scripts/action-map-real-user-e2e-lib.ps1`
  - 区分 `failed_collab_tool_calls` 与 `unexpected_failed_collab_tool_calls`。例如 runtime 拒绝把 subagent 绑定到已完成 node，且 agent 随后恢复到 open node，是受控 gate 结果，不作为整体 E2E 失败。

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - developer context 明确要求系统主动拆分独立调查轨道，不等待用户提出并行。
  - 要求显式维护依赖边：独立调查之间不互相依赖，implementation 依赖调查，validation/final 依赖上游结果。
  - `finish_node` 创建 next node 时，如未给出依赖，默认补上“当前完成节点 -> next node”。
  - `create_node` 未给出依赖时，默认使用当前已完成 frontier 作为依赖；当新节点会被主 agent 立即绑定时，会合并显式依赖和 frontier 依赖。
  - `implement_solution` 节点会直接合并已完成的 subagent-owned inspect 节点依赖，避免只通过间接路径表达关键证据来源。
  - `inspect_code_context` 主工具预算收紧；宽泛 inspect 节点耗尽预算且没有 subagent 工作时，runtime 阻止后续普通主 agent 工具调用，要求先拆出 ready inspect 节点并委派。
  - 最终 assistant message 会在 session 完成时写回当前 `final_synthesis` node，并释放 lease、完成节点。

## 关键证据口径

通过条件不再是“创建了很多 node”，而是：

```text
真实用户提示没有内部概念或并行暗示
真实 Whale agent 运行，使用本地安装的 C:\Users\77585\.whale\bin\whale.exe
真实 sandbox repo
真实工具调用、真实 subagent、真实 pytest
map 至少包含多个有意义 node
edge 存在且方向合理
关键依赖链：调查 -> 实施 -> 验证 -> 最终总结
至少两个 subagent-owned inspect 轨道
并列调查轨道之间没有依赖路径
implementation 直接依赖这些调查轨道
validation/test 直接依赖 implementation
下游执行不能早于上游完成
最终 final_synthesis 节点完成且无 open leaf
业务测试和隐藏 oracle 同时通过
```

## 最新验证

2026-05-30，使用本地安装后的 `whale.exe` 验证：

```text
scripts/run-action-map-natural-multi-agent-e2e.ps1
overall: PASS
thread_id: 019e77d3-0a47-75c2-ac30-7ab3e791d091
whale_sha256: CCEF6DFED3550F06C3A0CAAAA7277BC0FD16168FAE1C5F3CC7098E8F189268FF
prompt_leaks_internal_concepts: False
maps: 1
nodes: 7
edges: 9
ordered_edges: 9
edge_order_violations: 0
anchored_implementation_nodes: 1
parallel_inspect_tracks: 2
parallel_inspect_tracks_independent: True
direct_implementation_depends_on_parallel_inspect_tracks: True
direct_test_depends_on_implementation: True
open_leaf_nodes: 0
open_final_synthesis_nodes: 0
agents: 2
spawn_agent_calls: 2
subagent_results: 7
test_node_has_passing_pytest: True
ordinary_before_binding: False
unexpected_failed_collab_tool_calls: 0
```

```text
scripts/run-action-map-growth-health-e2e.ps1
overall: PASS
thread_id: 019e77d4-5797-7aa1-b136-71bf46a56e8c
whale_sha256: CCEF6DFED3550F06C3A0CAAAA7277BC0FD16168FAE1C5F3CC7098E8F189268FF
prompt_leaks_internal_concepts: False
maps: 1
nodes: 10
agents: 4
edges: 17
ordered_edges: 17
edge_order_violations: 0
anchored_implementation_nodes: 1
parallel_inspect_tracks: 4
parallel_inspect_tracks_independent: True
implementation_depends_on_parallel_inspect_tracks: True
direct_implementation_depends_on_parallel_inspect_tracks: True
test_depends_on_implementation: True
direct_test_depends_on_implementation: True
open_leaf_nodes: 0
open_final_synthesis_nodes: 0
subagent_results: 21
validation_node_has_pytest_result: True
unexpected_failed_collab_tool_calls: 0
hidden_oracle_exit_code: 0
real_command_execution: 26
```

补充自测：

```text
scripts/test-action-map-graph-health.ps1
overall: PASS
healthy-direct: PASS
direct-vs-transitive: PASS
anchored-implementation: PASS
order-violation: PASS
open-terminal: PASS
blocked-terminal: PASS

scripts/test-action-map-observability-lib.ps1
overall: PASS
preserve-existing-result-time: PASS
fill-empty-result-time: PASS

scripts/test-action-map-real-user-e2e-lib.ps1
overall: PASS
unexpected-failed-collab-filter: PASS
```

这个口径仍不能证明所有复杂任务中的 TaskSpace 都有效，但它证明了第一版关键底线：用户不教 agent 并行时，Whale 仍能把复杂开发任务组织成有边、有顺序、有归属、能收束的 task map。
