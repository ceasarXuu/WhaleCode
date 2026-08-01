# R5-J6.7.3 Map/control 语义去重结果

- 日期：2026-07-12
- 状态：完成
- 下一阶段：J6.7.4 Projection 与 Compaction 收敛

## 1. 生产路径变更

1. root task 删除 `task_goal/task_objective` 正文副本，只保存 canonical
   `source_event_ids`；机械空 Map 使用空引用，Agent 初始化时引用最新 user event 与本次 control call event。
2. `finish_nodes`、`finish_then_end` 和 `block_node` 删除 `result_summary/blocker_summary`。
   节点生命周期结果只保存 Agent control call 的 canonical event 引用，Runtime 不生成结论正文。
3. nested ordinary call/result 继续由 canonical Event Store 保存；control success 只返回 committed
   node/result IDs 和状态，不复述工具输出。
4. parser、control gate 和 ordinary-tool hard gate 都只返回一份 typed JSON。错误固定区分
   `protocol`、`state_machine`、`resource`，不再并列输出自然语言恢复提示。
5. 不保留旧字段兼容分支；旧字段在 parser 边界直接进入 typed protocol failure。

## 2. Provider schema 验证

`probe-schema-first-taskspace-control.ps1` 已与真实 schema 对齐，删除旧字段和虚构的
`payload_type`。DeepSeek stable endpoint 下：

- `initialize_then_actions`：HTTP 200，单 control call，含 1 个 nested action。
- `finish_nodes`：HTTP 200，单 control call，finish shape 合法。
- `finish_then_end`：HTTP 200，单 control call，terminal candidate 完整。

beta strict endpoint 接受 schema，但三种返回 shape 均不合法；当前产品 tool 为 `strict:false`，因此不把
strict 结果伪装为生产能力。

## 3. 工程验证

Rust：

- `codex-tools taskspace`：3 passed。
- `taskspace_control_args`：8 passed。
- `taskspace_control`：19 passed。
- Event Store/codec：7 passed。
- focused Runtime：13 passed。
- rollout reconstruction：22 passed。
- SessionState：7 passed。
- protocol map runtime：3 passed。
- locked `whale` build：passed。

PowerShell：cost instrumentation、metrics extractor、performance observation 均 passed。

## 4. Docker 横向结果

正式 run root：`target/r5-j6-7-3-live`。两个 paired run 都是单次诊断样本，因此 billing runner 的
非零退出来自 `repeats_lt_3/aggregate_not_enabled` 证据门禁，不代表 Agent 或 validator 失败。

| Sample | Mode | Result | Agent | Requests | Runtime tools | Input | Cached | Wall | Map |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| count-call-stack | Standard | solved | complete | 9 | 17 | 67,791 | 64,640 | 14.88s | none |
| count-call-stack | R5 | solved | complete | 9 | 10 | 70,224 | 57,728 | 19.09s | 3 nodes, open=0 |
| subscription-billing-repair | Standard | solved | complete | 16 | 28 | 206,397 | 196,608 | 65.63s | none |
| subscription-billing-repair | R5 | solved | complete | 10 | 17 | 98,154 | 88,832 | 49.30s | 3 nodes, 2 edges, open=0 |

两组 R5 的 canonical payload/call/output record 精确重复均为 0，orphan 为 0，protected miss 为 0，
retention/salience 均为 100%。旧 `task_goal/task_objective/result_summary/blocker_summary` 在两份真实
rollout 中均为 0。

## 5. 失败语义核验

真实运行出现两类可恢复错误：

1. count 样本的 Agent 在 nonterminal finish 中没有提供 next binding，收到
   `TaskSpaceControlResultV1/protocol_failed`。
2. billing 样本的 Agent 先尝试结束非当前节点，收到
   `TaskSpaceControlResultV1/state_machine_failed/lifecycle_target_not_current`。

两者都只出现一份 typed result，原始错误信息未丢失，Agent 下一请求自行修正。billing 首请求还曾在
机械空 Map 上直接调用 ordinary tool，Runtime 以 `TaskSpaceGateResultV1/state_machine_failed` 拒绝；
这说明 provider/Agent 没有遵循 bootstrap schema，不构成给 Runtime 增加语义纠正职责的理由。

## 6. 后续观察

- 两组 TaskSpace 都报告 `root_task_active_after_nodes_closed`；这是 root lifecycle 的机械状态问题，纳入
  J6.7.5 旧路径清理前的 call graph 审计，不影响本阶段 control 去重 gate。
- 单次 count 的 warm-cache 比 Standard 低 7.89 个百分点；J6.7.4 将按同 shape 与 prefix 证据验证，
  本阶段不对单样本方差作因果解释。

J6.7.3 的 known duplicate fields=0、raw feedback recovery=100%、错误分类准确，允许进入 J6.7.4。
