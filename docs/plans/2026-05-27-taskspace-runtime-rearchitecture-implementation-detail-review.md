我已完整阅读两份文档。以下是只读对抗审查结果。

---

## 对抗审查报告：TaskSpace Runtime 工程文档

**审查范围**：
- A: `2026-05-22-taskspace-runtime-design.md`（设计文档）
- B: `2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md`（实施方案）

**审查方法**：交叉比对两文档，检查内部一致性、边界条件遗漏、状态机完备性、协议歧义。

---

### P0 · 阻塞性问题

#### 1. `single_node_reason` 自我矛盾

B 第 374 行声明 `single_node_reason` "只作为审计字段，不作为安全防线"，但紧接着第 375 行对单节点任务强制执行严格预算（3 次工具结果上限、0 次 apply_patch），超过即设 `MaintenanceBarrier`。**这就是在用预算机制做安全防线**。要么承认它是防线并去掉"仅审计"声明，要么放宽单节点约束。当前写法会让实现者困惑该字段的真正权重。

#### 2. 控制动作失败升级策略在 B 中消失

A 第 975-981 行定义了三级失败升级：1 次重试 → 2 次强制 AskUser → ≥3 次 bootstrap 回滚。B 完全没有继承这个机制。B 的 `TaskSpaceRuntimeState` 中没有 `consecutive_control_failures`、`last_control_error`、`bootstrap_failed_reset_at_ms` 字段。如果模型连续输出格式错误的 `taskspace_control`，B 没有定义退路——agent 卡在无限重试循环中。

#### 3. `repair_required` 无清除路径

B 的 guard 判定表中，`repair_required` 状态下只允许 `route/reborn/ask` 或 repair 相关动作。但如果 `route_task` 因 repair 状态而被拒绝怎么办？（比如 `active_task_id` 指向不存在 task，repair 要求 agent route，但 route 又需要 active task）这形成了一个死锁。B 没有定义 `repair_applied` 事件或状态转换来清除 `repair_required`。

#### 4. `reborn_pending` 无取消机制

两份文档都没有定义如何清除一个已设置但用户改变主意的 `reborn_pending`。如果用户执行 `/task-reborn` 后悔了，agent 只能通过 `AskUser` 来清掉它——语义很奇怪。A 说 `reborn_pending` 在 agent 返回 `AskUser` 后清除（第 908 行），但 `AskUser` 的含义是"我缺上下文"，不是"用户取消 reborn"。这是语义重载。

---

### P1 · 严重问题

#### 5. `bootstrap_required` 和 `routing_required` 的关系未定义

A 使用 `bootstrap_required` 作为"需要初始化"的标记。B 同时引入了 `bootstrap_required` 和 `routing_required`，后者在每个 user turn 设置为 true。两者的交互未明确：当 `bootstrap_required=true` 且 `routing_required=true` 时，应该先满足哪个？agent 是否可以先 route 再 bootstrap？route 之后 bootstrap 是否自动清除？

#### 6. Node 状态 `Closed` 引入但下游行为未完整定义

B 引入 `Closed` 替代 A 的 `completed`。但 `Closed` 节点上的 `active_lease` 行为、result 追加行为、以及已 `Closed` 节点是否可以被后续 edge 引用（如 `Related` 边），都没有在 B 中明确。A 中 completed node 的 result 仍可被 compactor 提取用于 `RebornContext`——`Closed` 是否保留相同语义？

#### 7. `finish_node` → idle 过渡的 developer context 缺失

B 第 447 行：`finish_node(next_node_id)` 绑定失败时主 agent 进入 idle。但 B 没有定义 developer context 如何向模型反映这个 idle 状态。模型需要知道：(a) finish 本身成功，(b) next_node 绑定被抢占，(c) 当前无 lease，(d) 必须重新 bind。如果 prompt 只显示"no main lease"，模型可能误以为 finish 失败了。

#### 8. TaskNote / open_questions / blockers 在 B 中被移除

A 的 `TaskState` 包含 `open_questions`、`blockers`、`notes: Vec<TaskNote>`，并定义了 `TaskNoteKind` 枚举。B 的 `TaskState` 完全移除了这些字段。这意味着 B 失去了结构化的阻塞、疑问、用户态度跟踪能力。如果这些信息只存在于自然语言 context_summary 中，压缩后会不可逆丢失。

---

### P2 · 值得关注

#### 9. 成本阈值缺乏依据

B 第 577-581 行的成本信号阈值（单 node 12 次工具结果、3 次 apply_patch、20 个 map nodes、30% blocked ratio、60 min runtime）没有任何推导依据。一个 node 做 12 次小 read 完全合理；另一个 node 做 5 次大 shell 操作可能已有问题。阈值与操作类型的"重量"无关。

#### 10. `last_main_node_id` 的二义性

A 第 365-366 行说 `last_main_node_id` "只是恢复和 viewer 展示用的最近主节点提示"，但第 1630 行在 repair 中把它用作重建 `current_binding` 的权威来源。B 第 141-142 行的 `TaskState` 仍保留此字段但不说明其权威性。如果它只是"提示"，就不该在 repair 中当恢复依据。

#### 11. 并发 task switch + subagent 完成的竞态

两份文档都说 session state 锁保护状态变更，但没有明确描述以下交错：主线程在锁内完成 task switch（task A → pending），释放锁；同时 subagent 完成回调在锁内写 result 到 task A 的 node。这按设计是正确的（写回 pending task），但 subagent completion 触发的 `advance_downstream` 在 pending task 上产生的 ready 状态变化，下一轮 manifest 注入时能否被 agent 看到？B 未说明 pending task 内部的状态推进是否会触发 manifest 摘要更新。

#### 12. `create_nodes` 在 maintenance barrier 期间的歧义

B 的 guard 表第 741 行说 maintenance barrier 时允许 `create_nodes+bind`，但单独 `create_nodes` 是否允许？表中文案是"只允许 finish/block/create_nodes+bind/reborn/ask"，暗示 `create_nodes` 必须与 `bind` 配对。但如果 agent 只是先创建节点、下一轮再 bind，这个过渡轮次中 `create_nodes` 单独调用会被 barrier 拒绝。

#### 13. reborn 遇到 running lease 的拒绝信息不完整

B 第 469 行：reborn 遇到 running lease 时拒绝并返回 running lease 列表。但没有定义返回格式——是作为 `TaskSpaceControlFailed` 事件还是结构化错误？agent 收到这个列表后，如何知道每个 lease 对应哪个 node、哪个 subagent、是否可以 close/wait？

#### 14. A 中 `ever_bootstrapped` 保护不完整

A 的 bootstrap 自动回滚只在 `ever_bootstrapped = false` 时生效。但考虑场景：成功 bootstrap 后，由于 rollout 损坏导致所有 task 丢失，`ever_bootstrapped = true` 阻止自动回滚，而 repair 只能把 session 卡在 `repair_required`。用户此时无法退出 TaskSpace（设计如此），也无法恢复任务。

#### 15. Viewer URL 端口冲突未处理

两份文档都说打印 `http://127.0.0.1:<port>/` 作为 viewer URL，但都没有讨论端口复用失败、端口被占用、或远程/SSH 环境下 localhost 不可达的场景。

---

### 两份文档之间的一致性评价

| 关注点 | 设计文档 A | 实施方案 B | 一致性 |
|--------|-----------|-----------|--------|
| Node 状态集合 | 4 状态 (ready/running/completed/blocked) | 5 状态 (增加 Pending，重命名 Closed) | B 已声明覆盖 A |
| Lease 模型 | `AssignmentLease` + 4 状态枚举 | `ExecutionLease` + `LeaseHolder` 枚举 | 结构性偏离，B 简化了 |
| 主 agent 绑定 | `current_binding: TaskActionBinding` | `current_main_lease_id: ExecutionLeaseId` | 概念演进，从"绑定"到"租约" |
| 控制动作 | 5 个 action | 7 个 action | B 扩展，增加 finish/block |
| 失败处理 | 三级升级 | 未定义 | **严重缺失** |
| TaskNote | 完整定义 | 移除 | **功能回归** |
| 压缩快照 | 详细结构定义 | 仅描述原则 | B 细节不足 |

---

### 总结

两份文档在核心架构方向上是自洽的——Task/Map/Node/Lease/Result 五层模型清晰，runtime 不做语义判断的原则一致。但 B 作为"以真实失败样本驱动"的实施方案，在以下方面存在实质性缺口：

1. **失败升级策略缺失**（P0-2）——A 有的三级重试/回滚在 B 中完全消失
2. **repair 死锁风险**（P0-3）——`repair_required` 与 `route_task` 互斥且无清除路径
3. **TaskNote/blokers 移除**（P1-8）——结构化跟踪能力回归
4. **成本阈值无依据**（P2-9）——硬屏障依赖魔法数字

建议在 B 中至少补回控制动作失败升级策略和 `repair_required` 的清除状态机，否则进入工程实现后会在这两个点上反复踩坑。
