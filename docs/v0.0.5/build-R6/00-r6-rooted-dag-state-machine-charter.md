# build-R6 根式 DAG 状态机重构宪章

> R6 将 TaskSpace 收敛为一个由 Agent 使用、由 Runtime 机械维护的有向依赖图状态机。
> Map 不是状态机旁边的数据结构，Map 本身就是状态机：节点承载状态，边承载前置关系，
> 唯一 Task Root 是起点，唯一 Finish 是终点，所有工作都必须位于 Root 到 Finish 的路径上。

## 0.1 元数据

```text
Created: 2026-07-15
Updated: 2026-07-15
Version: v0.0.5 build-R6
Status: Phase A Frozen
Owner / Responsible: WhaleCode core runtime / TaskSpace
Related Systems: action_map, taskspace_control, event store, snapshot/replay,
  context projection, Docker benchmark, Web Viewer
Related Links:
  docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md
  docs/v0.0.5/build-R5/01-r5-phased-simplification-plan.md
  docs/v0.0.5/build-R5/31-r5-map-native-context-compression-charter.md
  docs/v0.0.5/build-R5/52-r5-k3-s4-2-result.md
Risk Level: Critical
Plan Type: Full architecture replacement
Change Type: Breaking model cutover / no compatibility
```

## 0.2 为什么进入 R6

R5 已经完成了大量必要铺垫：清理 Runtime 语义越界、建立 node-local Event Store、提高工具
反馈忠实度、收敛 projection、建立 Docker benchmark 和成本观测。但 R5 最后的自然运行暴露出
一个更底层的模型差异：Map 可以有多个零入度节点、多个任意终点和零条边；Task 生命周期、
Map 生命周期、当前节点和终结动作又在图外各自维护。

因此当前实现更接近“带可选依赖边的节点列表 + 外置状态机”，而不是预期的“Map 本身就是
状态机”。继续在 R5 中修边、补 root 推断或调整折叠条件，会把错误抽象固化为兼容债。

R6 是一次明确的模型切换：

```text
R5: TaskState + ActionMap + optional edges + external terminal convention
R6: one rooted single-entry/single-exit DAG whose topology and node states are the state machine
```

## 0.3 产品定义

TaskSpace 是 Standard 自然上下文的图化、状态机化再组织：

1. Agent 把任务拆成节点、声明依赖、选择工作节点、推进节点状态并显式结束任务。
2. Runtime 保存图、校验硬不变量、执行合法的原子状态变更、归档原始事件并忠实反馈。
3. Projection 从同一份 canonical Map 机械构造全局视图，不维护第二套任务事实。
4. Runtime 不规划节点、不推断依赖、不替 Agent 选择路径、不评价语义充分性。
5. 状态机只负责底线，不负责 Agent 能力上限。

目标图示：

```text
                         +--> [Work A] --+
[Task Root: OPEN] -------+                +--> [Work C] --> [Finish: PENDING]
                         +--> [Work B] --+             \
                                                           Agent explicit close

Root 在整个任务期间保持 OPEN。
普通节点按依赖完成条件进入 READY。
Finish 可由 Runtime 机械派生为 READY，但只能由 Agent 显式闭合并提交最终总结。
```

## 0.4 Canonical Map 模型

### 0.4.1 唯一权威对象

```text
TaskSpaceMap
  id
  root_node_id
  finish_node_id
  nodes[]
  edges[]
  node_events[]
  revision
```

每个节点只保留状态机和忠实上下文所需字段：

```text
MapNode
  id
  role: task_root | work | finish
  goal
  status
  owner / lease
  source_refs[]
  result_refs[]
  event_refs[]
```

每条边只有一个含义：

```text
MapEdge(from -> to) = to 的执行依赖 from
```

不再同时维护 `parent_id`、层级关系和依赖边。父子、前置条件、状态推进关系都由同一组有向边
表达。一个节点可以有多个入边和多个出边，以表达 join、fork 和共享前置条件。

### 0.4.2 不允许的平行权威

以下状态不得再独立决定任务生命周期：

- 图外可变 `TaskStatus`；
- 图外可变 `MapStatus`；
- 通过“最后一个 finish_node_id”临时指定终点；
- 通过创建顺序、当前节点或零入度扫描推断 Root；
- 通过无后继节点扫描推断 Finish；
- 与 Map 并行维护的计划、事实、决策或 completion ledger。

存储层可以保留 session/task/map 标识和索引，但任务是否完成必须由 canonical Root/Finish 状态
唯一派生，不能存在第二个可独立写入的完成位。

## 0.5 图硬不变量

任意可提交 Map revision 必须同时满足：

| Code | 不变量 |
|---|---|
| `single_root` | 恰好一个 `task_root`，且 ID 等于 `root_node_id` |
| `single_finish` | 恰好一个 `finish`，且 ID 等于 `finish_node_id` |
| `root_is_only_source` | Root 入度为 0；除此以外每个节点入度至少为 1 |
| `finish_is_only_sink` | Finish 出度为 0；除此以外每个节点出度至少为 1 |
| `acyclic` | 图中无环，自环也视为环 |
| `root_reaches_all` | Root 能到达每个节点 |
| `all_reach_finish` | 每个节点都能到达 Finish |
| `valid_references` | 所有边端点存在，边唯一，节点 ID 唯一 |
| `role_status_coherent` | 节点 role 与 status 组合合法 |
| `terminal_is_manual` | Finish/Root 的闭合只能来自 Agent 显式终结事务 |

这些规则排除孤立节点、额外起点、额外终点和无法完成的悬挂分支。Runtime 只报告违反了哪一条
机械规则，不解释 Agent 为什么画错图，也不自动补边修正。

## 0.6 生命周期契约

### 0.6.1 角色与状态

| Role | 合法状态 | 说明 |
|---|---|---|
| `task_root` | `open -> closed` | 初始化时打开，只能随最终终结事务闭合 |
| `work` | `pending -> ready -> running -> completed` | `blocked` 是可恢复运行状态，不等于完成 |
| `finish` | `pending -> ready -> closed` | READY 可机械派生，CLOSED 只能由 Agent 提交 |

R6-A 必须把完整状态转换表冻结为机器可读合同。未列出的转换一律拒绝；拒绝只返回状态、期望
前置条件和 `state_commit=false`，不得附带下一步策略。

### 0.6.2 Ready 规则

普通节点或 Finish 的全部前置条件满足时，Runtime 可以确定性地将其置为 READY：

```text
前驱是 task_root -> root=open 即满足启动条件
前驱是 work      -> predecessor=completed 才满足
前驱是 finish    -> 非法拓扑
```

Root 保持 OPEN 不代表阻塞整个图；它是任务仍存续的事实，也是直接后继的启动条件。Runtime
只计算依赖是否满足，不判断节点目标是否已经在语义上完成。

### 0.6.3 显式终结

终结必须是 Agent 发出的单个原子事务，目标只能是 Map 固有的 `finish_node_id`：

```text
preconditions:
  finish.status == ready
  graph invariants pass
  no running or blocked required work node
  final_summary is non-empty and Agent-authored

commit atomically:
  finish.ready -> finish.closed
  root.open -> root.closed
  map/task completion becomes derived true
  exact final_summary is stored as terminal event/result
```

Runtime 不自动触发 Finish，不自动生成或改写总结，不因“看起来完成”而结束任务。失败时整个事务
零提交，Agent 仍可修改 Map 或继续工作。

## 0.7 图变更契约

### 0.7.1 初始化

空 TaskSpace 只能通过一次 Agent 声明的初始化事务建立：

- 明确 Root、Finish、初始 work nodes 和全部初始边；
- Runtime 可以机械生成 map/node/event ID，但不能替 Agent生成目标或依赖；
- 初始化候选图先完整校验，再一次提交；
- 空图、缺 Root、缺 Finish、额外 source/sink 或断开图均零提交失败。

系统提示词和 tool schema 必须清楚说明初始化合同，但不得给 Agent 注入任务拆分建议。

### 0.7.2 动态变更

新增工作常常需要把已有边改接到新节点。R6 使用 Agent 声明的原子图事务，而不是单独
`create_node` 后让 Map 暂时失效：

```text
mutate_graph
  add_nodes[]
  add_edges[]
  remove_edges[]
```

Runtime 在 clone/candidate graph 上应用全部变更并验证；全部通过才增加 revision。Runtime 不得：

- 猜测新节点应插在哪条边上；
- 自动把新节点接到当前节点或 Finish；
- 为通过单入口/单出口校验而补边；
- 部分提交合法子集；
- 把校验错误改写成行动建议。

Agent 可以一次表达 fork、join 或多前置依赖。图事务只解决拓扑原子性，不限制 Agent 的语义规划。

## 0.8 Agent 与 Runtime 边界

| 事项 | Agent | Runtime |
|---|---:|---:|
| 定义节点 goal | 决定 | 原样保存 |
| 声明节点与依赖边 | 决定 | 校验并提交 |
| 选择 current/owner | 决定 | 校验 lease 和状态 |
| 判断工作是否语义完成 | 决定 | 不判断 |
| 推进显式节点状态 | 发起 | 校验状态转换 |
| 派生依赖 readiness | 不必手工计算 | 机械计算 |
| 选择工具与纠正错误 | 决定 | 只执行硬权限/安全规则 |
| 生成最终总结 | 决定 | 原样归档 |
| 自动补图或自动结束 | 禁止委托 | 禁止执行 |
| 保存事件、回放和日志 | 使用 | 负责准确性 |

发现 Agent 低级错误时，排查优先级继续遵守 R4/R5 已冻结原则：先检查工具反馈和上下文是否丢失、
残缺、扭曲或重复，再评估 Agent 能力；不得默认增加 Runtime 语义约束。

## 0.9 Event Store、Replay 与 Projection

### 0.9.1 Event Store

所有状态变化必须形成足以重建 Map 的 canonical event，例如：

```text
MapInitialized
GraphMutationCommitted
NodeBound
NodeBlocked
NodeCompleted
ReadinessChanged
TerminalCommitted
NodeDetailExpanded
```

事件保存 Agent 原始输入、机械校验结果、revision 和 source refs。相同事件历史必须重建出相同
nodes、edges、statuses、Root/Finish 和 terminal summary；snapshot 是加速结构，不是第二事实源。

### 0.9.2 Projection

Projection 是 canonical Map 的纯构造器：

- Root 和 Finish 始终显式可见；
- 全部节点与边的骨架始终可见；
- current/frontier 和近端节点展示更多忠实详情；
- 远端详情可按已验证策略透明折叠，并提供精确 ref/hash；
- tool feedback、失败状态和裁剪事实不可被再解释；
- 不输出 next-action、策略建议、事实推断或纠错提示；
- 不与基础上下文平行重复维护同一任务事实。

R5 S4.2 在自然 Map 中未观察到依赖边，收益处于 HOLD。R6 先建立正确图模型，再重新建立压缩
基线；不得为了激活折叠而诱导 Agent 造深图。

## 0.10 无兼容切换

本项目处于实验阶段，没有需要保留的 TaskSpace 用户数据。R6 明确禁止：

- 旧 TaskState/MapStatus 与新 Root/Finish 双写；
- 读取旧 snapshot 后猜测 Root/Finish；
- 为零边 Map 自动生成边；
- 把旧最后节点迁移为 Finish；
- 长期 feature flag、legacy adapter 或 silent fallback。

旧 schema/session/snapshot 遇到 R6 Runtime 时应明确失败并要求新建会话。风险控制依靠小主题提交、
测试、日志和 Git 回退，不依靠产品兼容分支。

## 0.11 R6 总验收

R6 完成必须同时满足：

1. 任意已提交 Map 都是唯一 Root、唯一 Finish、全节点位于 Root 到 Finish 路径上的 DAG。
2. 除 Root 外没有零入度节点，除 Finish 外没有零出度节点；多入边、多出边正常工作。
3. Root 在任务期间始终 OPEN；只有 Agent 显式终结事务能同时闭合 Finish 和 Root。
4. Task/Map completion 由图内状态唯一派生，不存在可独立漂移的外置完成状态。
5. 初始化和图变更全量预检、原子提交，失败时 revision、nodes、edges、statuses 零变化。
6. Event history、snapshot、resume、fork 和 replay 对同一 Map 产生逐字段一致结果。
7. Projection 只构造同一 Map 的忠实视图，不维护或注入平行语义。
8. Standard/R5/R6 横向样本中 correctness 无明确回退，成本差异可按 request/module/cache 解释。
9. 自然复杂样本能产生有意义的 fork/join/依赖边，而不是 Runtime 自动规划或测试提示诱导。
10. 旧模型生产路径和兼容代码删除，工作区、测试、文档与日志合同一致。

## 0.12 明确非目标

- R6 不实现通用业务工作流引擎、条件分支语言、循环图或自动调度策略。
- R6 不解析 reasoning，也不从自然语言推断节点、边或完成状态。
- R6 不用 Runtime 限制 Agent 的普通工具选择或代码决策。
- R6 不在根式 DAG 尚未稳定前追求新的压缩策略收益。
- R6 不承诺仅靠折叠解决“最小骨架本身超过上下文窗口”的长期问题。
- R6 不借重构扩大 `third_party/codex-cli` 之外的产品功能范围。

## 0.13 外部设计依据

这些来源用于验证机械状态机原则，不用于引入更强的 Runtime 业务编排：

1. van der Aalst 等对 workflow net 的定义要求唯一开始/结束位置，且每个节点都处于开始到结束的
   路径上；R6 采用其单入口、单出口和全路径覆盖思想，但保持 Agent 自主声明图。
   [TU/e: Aggregating causal runs into workflow nets](https://research.tue.nl/en/publications/aggregating-causal-runs-into-workflow-nets-2)
2. Amazon States Language 将 `StartAt` 和 terminal state 作为状态机结构的一等部分；R6 同样让
   Root/Finish 成为显式 schema，不通过运行时扫描猜测。
   [AWS Step Functions state machine structure](https://docs.aws.amazon.com/step-functions/latest/dg/statemachine-structure.html)
3. Temporal 的架构文档明确事件序列应足以恢复工作流相关状态；R6 据此要求 canonical events
   可独立重建 Map，snapshot 只做加速。
   [Temporal history service architecture](https://github.com/temporalio/temporal/blob/main/docs/architecture/history-service.md)
4. `petgraph::algo::toposort` 对有向图进行拓扑排序并在存在环时失败，可作为 R6 无环校验的成熟
   Rust 基础能力；可达性仍需独立正向/反向遍历验证。
   [petgraph toposort](https://docs.rs/petgraph/latest/petgraph/algo/fn.toposort.html)
