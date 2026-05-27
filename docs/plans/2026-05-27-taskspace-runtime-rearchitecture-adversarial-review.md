# TaskSpace Runtime 重构方案独立对抗审查

日期：2026-05-27

审查来源：`claude-ds-pro`

审查对象：

- `docs/plans/2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md`
- `docs/plans/2026-05-22-taskspace-runtime-design.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`

## Summary

审查结论：当前实施方案方向正确，但存在多处会导致重构后重新退化为单节点日志、或引入新卡死点的阻塞问题。

最关键的问题是：

1. 成本信号如果只是提示，不能阻止单宽节点继续吸收工具调用。
2. `single_node_reason` 无法被 runtime 客观验证，不能作为防线。
3. 实施方案缺少明确的每轮 task routing 协议，无法处理多个 task、切换旧 task、恢复 pending task。
4. 主 agent lease 引入后，缺少 finish/bind 之间的恢复和卡死规则。
5. 并发 spawn、reborn 与 running subagent、rollout repair、EdgeKind 都需要补齐。

## Findings

### P0: 成本信号必须是硬阻断，否则 70+ 工具调用仍可重现

方案同时写了“阻断普通工具”“不自动失败/不自动 reborn”“拒绝或强提醒”。如果最后落成软提醒，模型仍可忽略提示继续调用工具，单宽 node 退化不会消失。

### P0: `single_node_reason` 是弱防线

Runtime 无法判断 reason 是否真实。模型可以永远写 `single_node_reason: "this is straightforward"`，从而绕过 broad task 多节点要求。

### P0: 缺少 `TaskRoutingDecision`

产品设计需要主 agent 根据 task manifest 做 `ContinueTask/SwitchTask/CreateTask/AskUser`。实施方案只设计了 `init_task/bind_node`，无法处理用户切回旧任务、session 恢复后选择已有任务、或同 session 多 task 切换。

### P1: 主 agent lease 会制造新卡死点

如果主 agent 已持有 lease，却忘记 `finish_node` 直接 `bind_node`，runtime 应拒绝、隐式释放还是覆盖？方案没有定义。若完成 node 后没有及时绑定新 node，普通工具是否被拒绝也需要明确。

### P1: ready node 并发 claim 需要原子化

如果一轮中两个并行 `spawn_agent` 都指定同一个 `node_id`，必须保证只有一个能创建 lease。方案需要明确在 `SessionState` 锁内原子 claim。

### P1: `finish_node(next_node_id)` 的竞争窗口未定义

如果 finish 后尝试绑定 next node，但该 node 刚被并发 subagent 抢走，主 agent 应进入 idle、失败还是自动选择其他 node，需要定义。

### P2: `Closed` 节点是否可重新绑定存在歧义

如果 Closed 不可绑定，重新细化同一节点只能创建 follow-up node，可能节点膨胀；如果可绑定，会破坏“结果已沉淀”的语义。方案需要明确取舍。

### P1: `reborn_task` 与 running subagent 交互缺失

旧 map 变 historical 时可能仍有 running subagent。它们的结果应写回旧 map、丢弃、还是阻塞 reborn，需要明确。

### P1: Phase 3 和 Phase 5 顺序会造成中间态卡死

如果主工具 gate 在 `finish_node/block_node` 完成前上线，node 进入 Running 后无法离开。实施阶段需要合并或调整。

### P2: 缺少 repair 机制

恢复时如果 active task/map/node/lease 指向缺失对象，runtime 需要保守修复策略。只写“resume 后可恢复”不是实现机制。

### P2: 命名层存在混乱

内部仍叫 `ActionMapRuntimeState`，产品和 prompt 叫 TaskSpace。必须明确 schema、prompt、snapshot 和内部类型的映射规则，避免模型看到旧 map mode 术语。

### P2: Phase 0 必须是自动化失败回归，不只是文档化

只文档化失败样本不足以防止重构中再次出现单宽节点。必须在 Phase 0 增加自动化断言。

### P2: `EdgeKind` 被遗漏

产品设计里存在 dependency / related 两类边。当前实现只有单向依赖边。重构必须显式引入 EdgeKind，并确保 related edge 不阻塞 ready 推进。

## Required Fixes

1. 明确成本信号的硬阻断规则、解除条件和允许的恢复动作。
2. 不把 `single_node_reason` 作为安全防线；单节点 task 必须有严格预算或自动进入维护 barrier。
3. 加回 `TaskRoutingDecision`，让每轮 task 选择成为显式协议。
4. 定义 main lease 已存在、finish 后未 bind、bind 失败的状态机行为。
5. 明确所有 lease claim 必须在 session state 锁内原子完成。
6. 定义 reborn 遇到 running subagent 的策略。
7. 调整 phase 顺序，避免 gate 上线早于 finish/block。
8. 补充 rollout replay repair 策略。
9. 明确 EdgeKind。
10. Phase 0 增加自动化失败回归。

## Suggested Tests

- broad task 单节点 + `single_node_reason` 后，超过单节点预算必须阻断普通工具。
- TaskSpace active 的每个用户 turn 必须先 route task，除非已有未完成控制动作。
- 两个并行 `spawn_agent(node_id=same)` 只有一个成功创建 lease。
- main lease 存在时 `bind_node` 必须失败并给出明确恢复指令。
- `finish_node(next_node_id)` 被并发抢占时，主 agent 进入 idle，普通工具被阻断。
- reborn 时存在 running subagent，必须按定义阻断或安全归档到旧 map。
- replay 中 active lease 指向缺失 node 时，runtime 进入 repair-required 状态而不是 panic 或伪造状态。
- dependency edge 阻塞 ready；related edge 不阻塞 ready。

## Verdict

审查认为：方向正确，但在修复上述 P0/P1 问题前，不应进入工程实现。
