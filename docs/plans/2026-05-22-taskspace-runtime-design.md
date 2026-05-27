# TaskSpace Runtime 设计

日期：2026-05-22

> 工程落地请以
> [2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md](./2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md)
> 为准。本文保留产品语义和概念设计；2026-05-27 文档描述基于真实失败样本后的 runtime 重构实施方案。

## 结论

Whale 的长期目标不应是让用户手动选择是否使用 Action Map，而是提供一个默认运行的任务空间：

```text
Session = 用户对话空间
TaskSpace = session 内的任务空间 runtime
Task = 一个持续跟进的主题任务
Task Map = task 内部的行动图
Node = task map 内的可执行工作单元
```

用户不再直接理解或操作 `map`。用户看到的是“任务空间”。`map` 是 TaskSpace 内部用于约束 agent 行动的执行结构。

新的产品约束：

- 使用 `/taskspace` 进入任务空间模式。
- 单个 session 只能进入 TaskSpace，不能退出。
- 用户要回到完全普通模式，应新开 session。
- 进入 TaskSpace 后，runtime 必须约束 agent 运行在某个 task 的 map/node 上。
- 一个 session 可以有多个 task，但同一时刻只有一个 active task 驱动当前主 agent 行动。
- subagent 必须绑定 active task map 中的 node lease。

## 为什么需要 TaskSpace

当前 session 更像用户决定的强制上下文空间。用户会在一个 session 里连续谈多个主题、插话、追问、切换方向。session 本身不等于 task。

如果把 session 当成唯一 task，会出现几个问题：

- 一个长 session 中多个主题互相污染。
- 临时插话会破坏当前任务 map。
- 用户返回旧主题时缺少结构化恢复点。
- 上下文压缩会把任务结构压成普通摘要，丢失 node/result/依赖关系。
- `map mode` 变成用户要理解和切换的功能开关，不适合作为未来默认工作模式。

TaskSpace 的目标是增加一个轻量、有状态、可约束的 task 层：

```text
用户自然对话
  -> runtime 暴露当前 TaskSpace manifest
  -> 主 agent 判断属于哪个 task
  -> runtime 校验 agent 给出的 task binding
  -> agent 在该 task 的 map/node 上行动
  -> 结果写回 task/node
  -> 压缩时保留 task 结构
```

## 用户模型

用户只需要理解：

- `/taskspace`：进入任务空间模式。
- `/taskspace` 成功后会在对话流中直接打印 TaskSpace viewer URL，用户可以立刻在浏览器中打开。
- `/task-show`：再次打印或打开任务空间 viewer。用户可见命令面只保留 TaskSpace 概念。
- `/task-reborn`：当前任务换一条思路重生执行路径。用户可见命令面只保留 TaskSpace 概念。

用户不需要理解：

- map。
- node lease。
- task routing。
- 什么时候应该开关 map mode。
- 什么时候应该创建新 map。

## 命令语义

### `/taskspace`

`/taskspace` 是单向入口。

如果当前 session 未进入 TaskSpace：

1. 设置 `taskspace_enabled = true`。
2. 产生一次 transition notice。
3. 启动或复用本 session 的 TaskSpace viewer。
4. 在对话流中打印 viewer URL，例如 `TaskSpace: http://127.0.0.1:<port>/`。
5. 设置 `bootstrap_required = true`、`ever_bootstrapped = false`。
6. 下一次 agent 行动前必须执行 task bootstrap/sync。
7. bootstrap/sync 完成前禁止普通工具行动、代码修改和 subagent spawn。

如果当前 session 已经进入 TaskSpace：

1. 返回机械状态：TaskSpace already enabled。
2. 重新打印当前 TaskSpace viewer URL。
3. 不重复创建 task。
4. 不改变 active task。

不提供 `/taskspace off`。如果用户需要完全回到普通 session，应新开 session。

### `/task-show`

打开本地 browser viewer，展示 task list、active task、task map、node、lease、result。

第一版可以复用现有 viewer 技术路径，但用户文案必须是 TaskSpace。

`/task-show` 的职责是可观测性，不改变 runtime 状态：

- 如果 viewer 已启动，直接打印或打开已有 URL。
- 如果 viewer 未启动，启动 viewer 后打印 URL。
- 如果 TaskSpace 已启用但还没有 active task，viewer 仍应可打开，显示 `bootstrap_required` 或 `no active task yet`。
- 如果 TaskSpace 未启用，返回机械提示：先使用 `/taskspace`。

### `/task-reborn`

重生当前 active task 的执行路径，而不是重启整个 session，也不是新建 task。

这个命令的价值不是“清空任务”，而是在同一个 task identity 下换一条执行路径。
它主要用于处理 action path 劣化：旧 map 由于错误方向、过度生长、噪声上下文、反复失败或用户明确要求换思路，
已经不适合继续作为当前执行结构。

保留它的理由：

- task 仍然是同一个用户目标，不能新建 unrelated task 来假装重来。
- 旧 map 里可能有可复用事实、约束、文件引用和失败经验，不能直接丢弃。
- 新 map 需要从 task objective、用户约束、durable facts、open questions 重新初始化，避免继承旧路径噪声。
- viewer 需要能对比历史路径和当前路径，帮助用户理解为什么换路。

限制：

- 只能作用于当前 active task。
- 必须由用户显式命令触发，runtime 不自动 reborn。
- 命令 handler 必须设置 `reborn_pending = Some(RebornRequest { task_id, old_map_id })`；
  没有该标记时，runtime 必须拒绝 agent 自行提交的 `RebornMap`。
- 不改变 task id，不改变 task status。
- 不删除旧 map，不删除旧 result。
- 旧 map 默认只作为历史路径保留，不再驱动当前行动。
- 新 map 初始化时只继承经过压缩筛选的 durable context，不把旧 node 全量复制过来。

语义：

- 当前 active task 保留 task id 和目标。
- 当前 active task 的旧 map 只作为历史路径保留，不再参与当前执行。
- 创建新的 task map。
- agent 需要基于当前 task context 重新生成初始 nodes。
- active_map_id 指向新 map。
- 对话流中打印 TaskSpace viewer URL，用户可以直接查看新旧路径。

#### reborn map 如何产生

新 map 不是 runtime 按模板机械生成，也不是复制旧 map。

生成流程：

```text
用户执行 /task-reborn
  -> runtime 读取 active task
  -> runtime 设置 reborn_pending
  -> runtime 构造 RebornContext
  -> 主 agent 基于 RebornContext 生成 TaskMapDraft
  -> runtime 校验 TaskMapDraft
  -> runtime 创建新 map
  -> task.active_map_id 指向新 map
  -> runtime.current_binding 指向新 map 的起始 node
  -> runtime 清空 reborn_pending
  -> 旧 map 保留为 historical path
  -> 打印 viewer URL
```

`RebornContext` 只包含可迁移的 durable context：

```rust
RebornContext {
    task_id: TaskId,
    title: String,
    objective: String,
    durable_facts: Vec<String>,
    user_constraints: Vec<String>,
    source_refs: Vec<String>,
    open_questions: Vec<String>,
    blockers: Vec<String>,
    failure_lessons: Vec<String>,
    previous_map_digest: String,
    candidate_nodes: Vec<BaseMapCandidateNode>,
}
```

`RebornContext` 的字段来源要固定，不能让实现时随意拼 prompt：

| 字段 | 来源 |
| --- | --- |
| `task_id/title/objective` | 当前 active task |
| `durable_facts` | 当前 map 中 completed/blocked node 的 result summary，由 compactor 提取，不取 full body |
| `user_constraints` | task context summary、用户显式约束 note、developer 指令摘要 |
| `source_refs` | task 和当前 map 中仍被引用的文件/符号/URL |
| `open_questions` | task.open_questions 与 blocked node summary |
| `blockers` | blocked node summary 和最近 tool error summary |
| `failure_lessons` | blocked nodes、用户否定意见、reborn_reason 历史摘要 |
| `previous_map_digest` | 当前 active map 的一页摘要，包括节点数量、关键结果、失败原因 |
| `candidate_nodes` | BaseMap metadata 的候选节点，一次性暴露给 agent |

runtime 可以用已有 summary 字段和 compactor 做抽取，但不做任务语义判断。
如果某个字段无法可靠提取，就留空或写入 `unknown`，不要编造。

不进入 `RebornContext` 的内容：

- 旧 map 的完整 node 列表。
- 旧 result full body。
- 旧 agent 的长篇推理过程。
- 已被用户否定的方案细节。
- 与新路径无关的临时 shell 输出、报错堆栈和噪声日志。

主 agent 输出 `TaskMapDraft`：

```rust
TaskMapDraft {
    title: String,
    purpose: TaskMapDraftPurpose,
    nodes: Vec<NodeDraft>,
    edges: Vec<EdgeDraft>,
    current_main_local_id: String,
    inherited_context_summary: String,
}

enum TaskMapDraftPurpose {
    Initial,
    Reborn { reason: String },
}
```

`TaskMapDraft` 里的 node id 不是最终 `NodeId`。主 agent 只能生成 draft 内部临时 id：

```rust
NodeDraft {
    local_id: String,
    title: String,
    context_summary: String,
    source_refs: Vec<String>,
}

EdgeDraft {
    from_local_id: String,
    to_local_id: String,
    kind: EdgeKind,
}
```

`local_id` 只在本次 draft 内有效，例如 `boundary`、`architecture-review`、`smoke-test`。
runtime 创建 map 时负责分配正式 `NodeId`，建立 `local_id -> NodeId` 映射，并把 edge、
`current_main_local_id` 改写成正式 id。agent 不需要知道正式 id 命名规则，也不需要规避旧 map 的 id 空间。

runtime 只做结构校验，不做语义评判：

- 至少有一个 node。
- `local_id` 必须非空且在 draft 内唯一。
- `current_main_local_id` 必须引用本 draft 中存在的 node。
- edge 只能引用本 draft 中存在的 `local_id`。
- 依赖边不能形成环。
- node title/context_summary 不能为空。
- 新 map 创建成功前，旧 active map 仍然是当前执行路径。

新 map 创建成功后：

- 旧 map 的 `historical = true`。
- 新 map 的 `parent_map_id = Some(old_active_map_id)`。
- 新 map 的 `reborn_reason` 来自 `TaskMapDraftPurpose::Reborn.reason`。
- task 的 `active_map_id` 指向新 map。
- `current_binding.node_id` 指向 `current_main_local_id` 映射出的正式 node，task 的 `last_main_node_id` 同步为该 node。
- 旧 map 的 result body、summary、lease 历史继续保留在旧 map 内，不复制到新 map。

多次 reborn 时形成版本链：

```text
map-1 <- map-2 <- map-3
                  ^
                  active_map_id
```

prompt 注入时只带当前 map 的完整 active task pack，以及父 map 的短摘要。
不递归展开祖父 map，避免上下文膨胀。viewer 可以通过版本链按需查看历史。

agent 负责语义选择：

- 哪些事实应该继承。
- 哪些失败经验应该保留。
- 新路径应该从哪个节点开始。
- 是否需要先询问用户澄清。

如果 agent 认为缺少足够上下文生成新 map，应返回 `AskUser`，runtime 不切换 active_map_id。

### `/task-abandon`

不设计 `/task-abandon`。

原因是 task 不是工单系统里的可关闭对象。用户可能在任意未来轮次重新打开、追问、引用或纠偏一个看似已经结束的任务；
runtime 也无法客观判断任务已经完成或废弃。因此 TaskSpace 不提供 task abandoned 状态。

如果用户明确说“不做这个了”，主 agent 可以把这句话写入 task context summary，
然后在切换到其他 task 时让该 task 进入 `pending`。它仍保留在 TaskSpace manifest 中，供未来恢复或引用。

## 核心数据模型

### SessionState

```rust
SessionState {
    taskspace: TaskSpaceRuntimeState,
    // existing fields...
}
```

### TaskSpaceRuntimeState

```rust
TaskSpaceRuntimeState {
    enabled: bool,
    active_task_id: Option<TaskId>,
    tasks: BTreeMap<TaskId, TaskState>,
    pending_transition_notice: Option<String>,
    bootstrap_required: bool,
    ever_bootstrapped: bool,
    current_binding: Option<TaskActionBinding>,
    last_control_error: Option<TaskSpaceControlError>,
    consecutive_control_failures: u32,
    bootstrap_failed_reset_at_ms: Option<i64>,
    reborn_pending: Option<RebornRequest>,
    latest_compression_snapshot: Option<TaskSpaceCompressionSnapshot>,
    next_task_seq: u64,
}
```

```rust
RebornRequest {
    task_id: TaskId,
    old_map_id: TaskMapId,
    requested_at_ms: i64,
}
```

### TaskState

```rust
TaskState {
    id: TaskId,
    title: String,
    objective: String,
    status: TaskStatus,
    owner_session_id: ThreadId,

    context_summary: String,
    source_refs: Vec<String>,
    open_questions: Vec<String>,
    blockers: Vec<String>,

    active_map_id: Option<TaskMapId>,
    maps: BTreeMap<TaskMapId, TaskMapState>,

    last_main_node_id: Option<NodeId>,
    notes: Vec<TaskNote>,
    created_at_ms: i64,
    updated_at_ms: i64,
    last_active_at_ms: i64,
}
```

`TaskState.last_main_node_id` 只是恢复和 viewer 展示用的最近主节点提示，不是当前行动权威。
主 agent 当前行动的唯一权威来源是 `TaskSpaceRuntimeState.current_binding`。

### TaskStatus

第一版只保留两个状态：

```text
active      当前正在驱动主 agent 行动
pending     当前不驱动主 agent 行动，但仍可被未来输入恢复或引用
```

不要引入 completed、abandoned、paused、stale、quality-risk、reviewing 等更多状态。

原因：

- runtime 无法客观判断一个开放任务是否真的完成。
- 用户随时可能重新打开、追问或修正一个看似已经完成的任务。
- 用户“不做了”也不等于这个 task 永久废弃；未来仍可能作为上下文被引用。

因此 `TaskStatus` 只表达当前调度焦点，不表达完成度或价值判断。复杂信息可以写入
`context_summary`、`blockers`、node result summary 或 task notes。

如果用户说“这个任务完成了”或“不做了”，第一版也不扩展 `TaskStatus`：

```rust
TaskNote {
    kind: TaskNoteKind,
    summary: String,
    created_at_ms: i64,
}

enum TaskNoteKind {
    UserSaysDone,
    UserSaysStop,
    AgentBelievesDone,
    ContextUpdate,
    RepairNote,
}
```

这些 note 可以影响 manifest 摘要和 viewer 展示，但不能改变 runtime 调度状态。
这样可以保留信息，又不让 runtime 假装能客观判断开放任务是否完成或废弃。

### TaskMapState

用户不直接看到 `TaskMapState` 这个概念。代码层面第一版可以继续复用现有 `ActionMapInstance`，但在协议和 UI 上称为 task path 或 task map。

```rust
TaskMapState {
    id: TaskMapId,
    title: String,
    status: TaskMapStatus,
    parent_map_id: Option<TaskMapId>,
    reborn_reason: Option<String>,
    historical: bool,
    base_map_version: String,
    nodes: BTreeMap<NodeId, NodeState>,
    edges: Vec<TaskMapEdge>,
    leases: BTreeMap<LeaseId, AssignmentLease>,
    results: BTreeMap<ResultId, NodeResult>,
}
```

### NodeState

```rust
NodeState {
    id: NodeId,
    title: String,
    status: NodeStatus,
    context_summary: String,
    source_refs: Vec<String>,
    active_lease: Option<LeaseId>,
    result_ids: Vec<ResultId>,
    origin_node_id: Option<NodeId>,
}
```

Node 状态继续保持最小集合：

```text
ready
running
completed
blocked
```

### TaskMapEdge 与推进规则

第一版只保留两类边：

```rust
TaskMapEdge {
    from_node_id: NodeId,
    to_node_id: NodeId,
    kind: EdgeKind,
}

enum EdgeKind {
    Dependency,
    Related,
}
```

- `Dependency` 是有向依赖边：`from -> to` 表示 `to` 依赖 `from`。
- `Related` 是无阻塞关系边：用于 viewer 和上下文相邻提示，方向不参与 ready 推进。

ready 推进规则固定为 AND 依赖：

```text
advance_downstream(completed_node):
  for each outgoing Dependency edge completed_node -> target:
    if target.status != blocked:
      if all incoming Dependency predecessors of target are completed:
        if target.status is not running/completed:
          target.status = ready
          emit task_node_status_changed
```

如果上游 node 是 `blocked`，下游不会自动 ready。主 agent 可以选择创建替代 node、调整依赖边、
询问用户，或让用户触发 `/task-reborn`。runtime 不用质量判断替 agent 决定 blocked 依赖是否可忽略。
没有任何 incoming `Dependency` 边的 node，在 map 创建时初始就是 `ready`。

### AssignmentLease

lease 是 subagent 持有 node 的唯一硬约束。node 是否能再次被接管，不看 subagent 的自然语言结果，
只看 lease 状态和 `node.active_lease`。

```rust
AssignmentLease {
    id: LeaseId,
    task_id: TaskId,
    map_id: TaskMapId,
    node_id: NodeId,
    state: AssignmentLeaseState,
    attached_thread_id: Option<ThreadId>,
    claimed_at_ms: i64,
    completed_at_ms: Option<i64>,
    released_at_ms: Option<i64>,
    release_reason: Option<String>,
}

enum AssignmentLeaseState {
    Claimed,
    Attached(ThreadId),
    Completed,
    Released,
}
```

状态语义：

- `Claimed`：runtime 已经把 node 标记为 running，但 child thread 还没 attach。
- `Attached`：child thread 已经绑定该 node，结果只能写回该 node。
- `Completed`：subagent 正常结束并写回 result，`node.active_lease` 已清空。
- `Released`：spawn 失败、超时、崩溃或人工关闭导致 lease 被释放，`node.active_lease` 已清空。

`Completed` 和 `Released` 都是历史记录，不再阻止 node 被后续 agent 接管。

### NodeResult

`NodeResult` 是 node 内所有可恢复产出的统一记录，不只表示 subagent 最终回答。
主 agent 普通工具调用、主 agent 一轮行动摘要、subagent 结果、超时进展总结都写入同一个 `results` 表，
通过 `kind` 区分来源。

```rust
NodeResult {
    id: ResultId,
    task_id: TaskId,
    map_id: TaskMapId,
    node_id: NodeId,
    kind: NodeResultKind,
    summary: String,
    body: String,
    source_thread_id: ThreadId,
    source_refs: Vec<String>,
    created_at_ms: i64,
}

enum NodeResultKind {
    MainToolCall,
    MainTurnSummary,
    SubagentFinal,
    TimeoutProgress,
    SystemRepair,
}
```

tool guard 执行普通工具后，必须至少写入 `MainToolCall`：

- `source_thread_id` 填当前主 session/thread id。
- `summary` 保存工具名、退出状态、关键输出摘要、涉及文件或 symbol。
- `body` 保存完整工具结果或可恢复引用；过大输出可以保留截断 body + artifact/log ref。
- `source_refs` 保存本次工具触达的文件、URL、symbol。
- `result_id` append 到当前 `NodeState.result_ids`。

如果一轮主 agent 行动产生多个小工具结果，runtime 可以额外生成一个 `MainTurnSummary`，
但不能用它替代原始 `MainToolCall` attribution。I8 中“普通工具结果必须记录到当前 node”
具体就是写入 `NodeResultKind::MainToolCall`。

## Runtime 约束

进入 TaskSpace 后，runtime 必须满足：

```text
每次主 agent 行动前：
  必须有 active_task_id
  必须有 active task map
  必须有 current_binding
  current_binding.task_id/map_id/node_id 必须指向 active task 的 active map 中存在的 node

每次 subagent spawn 前：
  必须有 active_task_id
  必须有 active task map
  必须 claim 一个 ready node
  claim 成功后 node -> running
  lease attach 到 subagent thread

每次 subagent 完成后：
  result 写入对应 node
  node -> completed 或 blocked
  下游 ready 状态按边推进
```

runtime 不负责理解任务语义。runtime 负责状态机和硬约束。

agent 负责：

- 判断用户输入属于哪个 task。
- 创建或更新 task。
- 生成 task map nodes。
- 选择当前主 agent 行动绑定哪个 node。
- 决定当前行动是否应继续、停止、等待用户或记录阻塞。

## Task Routing

TaskSpace 每轮用户输入前都要完成 task routing，但 routing 的语义判断必须由主 agent 完成。

runtime 绝不能用关键词、BM25、向量检索、规则打分或其他传统检索/匹配算法自动选择 task。
原因很简单：task 归属是开放语义判断，runtime 承担不了这个能力，也不应该伪装成能承担。

runtime 只负责三件事：

- 暴露当前 TaskSpace manifest，让 agent 看见有哪些 task 可以继续。
- 接收主 agent 返回的 routing decision。
- 校验 decision 是否引用了存在的 task/map/node，是否满足状态机和权限约束。

主 agent 负责：

- 理解用户当前输入和会话上下文。
- 判断是继续当前 task、切换到 pending task、新建 task，还是询问用户。
- 给出选择理由和上下文更新。
- 在被 runtime 接受后，继续执行该 task 的 map/node。

因此这里的 `TaskRoutingDecision` 是 agent output，不是 runtime decision。

### routing 输入

```text
- 最新用户输入
- 当前 active task manifest
- pending task manifest
- 最近若干轮 session 摘要
- 可选：当前工作区变更摘要
```

### routing 输出

第一版输出只允许四种：

```rust
enum TaskRoutingDecision {
    ContinueTask {
        task_id: TaskId,
        context_update: String,
        main_node_id: NodeId,
    },
    SwitchTask {
        task_id: TaskId,
        reason: String,
        previous_task_context_update: String,
        target_task_context_update: String,
        main_node_id: NodeId,
    },
    CreateTask {
        title: String,
        objective: String,
        context_summary: String,
        initial_map: TaskMapDraft,
    },
    AskUser {
        question: String,
    },
}
```

runtime 校验输出：

- `ContinueTask.task_id` 必须存在。
- `SwitchTask.task_id` 必须存在。
- `ContinueTask.main_node_id` 必须存在于当前 active map。
- `SwitchTask.main_node_id` 必须存在于目标 task 的 active map。
- `CreateTask.initial_map.purpose` 必须是 `Initial`。
- `CreateTask.initial_map` 必须至少有一个 node。
- `TaskMapDraft.current_main_local_id` 必须引用 `nodes` 中存在的 `local_id`。
- `NodeDraft.local_id` 必须在本次 map draft 内唯一。
- edge 不能引用不存在的 `local_id`。
- active map 至少有一个 ready/running/current node。

创建 task/map 时，runtime 分配正式 `TaskId`、`TaskMapId`、`NodeId`，并把 draft 内部
`local_id` 映射为正式 id。主 agent 不能在 `CreateTask` 或 `TaskMapDraft` 中自造正式 id。
第一次 `CreateTask` 成功后，runtime 必须设置 `bootstrap_required = false`、`ever_bootstrapped = true`，
并把 `current_binding` 指向新 task 的 active map 和 main node。

`CreateTask` 和 `/task-reborn` 共享同一个 map 创建入口：

| 场景 | 输入 | runtime 映射 |
| --- | --- | --- |
| 新 task 初始化 | `CreateTask.initial_map`，`purpose=Initial` | 创建 task 后创建第一个 active map，`parent_map_id=None`，`historical=false` |
| task reborn | `RebornMap.draft`，`purpose=Reborn { reason }` | 保留 task，旧 map `historical=true`，新 map `parent_map_id=old_active_map_id` |

除 task id 和 parent/historical 字段外，两条路径必须调用同一个 `create_task_map_from_draft`。
这样避免初始 map 和 reborn map 形成两套 schema、两套校验、两套 id 分配逻辑。

### SwitchTask 应用语义

`SwitchTask` 是最容易造成上下文污染的控制动作，必须定义为一次原子切换：

```text
apply SwitchTask(target_task_id):
  1. 读取当前 active task 作为 previous_task
  2. 将 previous_task_context_update 追加到 previous_task.context_summary/task notes
  3. previous_task.last_main_node_id = current_binding.node_id
  4. previous_task.status -> pending
  5. target_task.status -> active
  6. 将 target_task_context_update 追加到 target_task.context_summary/task notes
  7. active_task_id = target_task_id
  8. current_binding = { target_task_id, target.active_map_id, main_node_id }
  9. target_task.last_main_node_id = main_node_id
```

running subagent 不因为 task switch 被强制中断。

原因：

- subagent 的行动已经绑定 node lease，结果仍然有明确归属。
- 切换 task 只是主 agent 当前焦点变化，不代表旧 task 的后台 node 无效。
- 强制把 running node 改成 blocked 会制造伪阻塞状态。

因此切走 task 后：

- 旧 task 的 running node 保持 running。
- 旧 task 的 subagent 完成后仍写回原 lease node。
- 结果写回时如果 task 不是 active，只更新该 pending task 的 node/result，不切换 active task。
- viewer 可以显示 pending task 中仍有 running lease。
- 如果用户回到该 task，agent 可根据最新 result 决定继续、创建新 node 或 reborn。

### 什么时候创建新 task

agent 判断用户输入和已有 task 的 objective/context 不属于同一主题时创建新 task。

典型场景：

- 用户从“项目质量分析”切到“解释某条命令报错”。
- 用户从“设计 TaskSpace”切到“修复 viewer 打不开”。
- 用户明确说“另一个问题”。

### 什么时候更新旧 task

用户输入是对当前任务的补充、纠偏、追问、继续执行，应更新旧 task。

典型场景：

- “继续”
- “按刚才方案做”
- “刚才那个设计里补一下压缩”
- “查看当前 map/task”

### 什么时候询问用户

当输入可能同时属于多个 pending task，且错误绑定会造成污染时，agent 应询问用户。

第一版可以保守处理：

```text
如果 top candidates 不确定，问用户：
“这个问题是继续任务 A，还是新建一个任务？”
```

不要做复杂置信分或语义评分。主 agent 第一版可以参考标题、objective、最近活跃时间、显式指代词辅助判断。
这些信息只作为 manifest 暴露给 agent，不形成 runtime 自动匹配逻辑。

## Agent 行动绑定

### 主 agent

主 agent 不需要 lease，但必须有 `current_binding.node_id`。

目的：

- 工具调用归属到 node。
- 结果摘要写回 node。
- 压缩时能知道主 agent 正在推进哪个工作单元。
- viewer 能显示主 agent 当前工作焦点。

主 agent 切换 node 的方式：

```rust
TaskActionBinding {
    task_id,
    map_id,
    node_id,
    reason,
}
```

`reason` 不参与权限判断，只写入 `task_main_node_bound` event、viewer 的当前焦点变更说明、
以及压缩摘要中的最近切换原因。tool guard 不读取 `reason`。

每次 turn 开始时，routing/bootstrap/sync 必须给出当前 node。runtime 将它写入
`TaskSpaceRuntimeState.current_binding`，并把同一个 node id 镜像到该 task 的 `last_main_node_id`。
tool guard 只能读取 `current_binding`，不能把 `last_main_node_id` 当成行动授权。

### agent 如何知道当前 id

不要要求 agent 在每一次普通工具调用里手工携带 `task_id` / `map_id` / `node_id`。
这会制造大量脆弱点：漏带、带错、复制旧 id、压缩后忘记 id、并行时串 id。

正确机制是：

```text
agent 只在 routing / binding / map draft 这类控制动作里显式引用 id
普通工具调用不携带 task/map/node id
runtime 从 session state 的当前 TaskActionBinding 自动归属
```

已有 active binding 时，turn-level hook 注入当前绑定：

```text
TaskSpace is enabled.

Current binding:
- task_id: task-3
- map_id: map-2
- current_node_id: node-5
- current_node_title: 方案审查

All ordinary tool calls in this turn will be attributed to this node.
If you need to switch task or node, emit a TaskRoutingDecision or TaskActionBinding first.
```

此时 agent 调用 shell/read/edit/search/patch 等普通工具时，不需要传 task 坐标。
tool-level guard 从 session state 读取：

```text
session.taskspace.current_binding.task_id
session.taskspace.current_binding.map_id
session.taskspace.current_binding.node_id
```

然后把工具调用和结果归属到当前 node。

没有 active binding 时，runtime 不允许普通工具调用，而是要求 agent 先输出结构化控制动作：

```text
TaskSpace bootstrap required.
Before ordinary work, return one TaskRoutingDecision:
- ContinueTask
- SwitchTask
- CreateTask
- AskUser
```

agent 返回 `CreateTask` 或 `SwitchTask` 后，runtime 生成或更新真实 id，并写入 session state。
后续普通工具调用继续走默认绑定。

切换 task/node 时，也不在普通工具参数里塞 id，而是先提交控制动作：

```rust
TaskActionBinding {
    task_id: TaskId,
    map_id: TaskMapId,
    node_id: NodeId,
    reason: String,
}
```

runtime 校验通过后更新当前 binding。

因此 agent 知道 id 的方式是：

- runtime 每轮注入 TaskSpace manifest 和 current binding。
- agent 只在需要创建、切换、reborn 或绑定 node 时显式引用 id。
- 普通行动由 runtime 默认归属到当前 binding。
- tool guard 负责防止无 binding 或非法 binding 的行动执行。

### hook 与 guard 分工

TaskSpace 约束需要两层入口：

```text
turn-level hook:
  在 agent 普通行动前检查是否需要 bootstrap/routing/binding
  缺少 active task/map/node 时，要求 agent 先输出结构化控制动作

tool-level guard:
  在工具 handler 真正执行前再次校验当前 TaskActionBinding
  校验失败则拒绝执行
  校验通过则把工具调用和结果写回当前 node
```

两层都需要。turn hook 负责提前引导，避免用户看到一串工具失败；tool guard 负责硬约束，防止 agent 绕过流程直接行动。

### 结构化控制动作协议

TaskSpace 第一版不要新增一堆自然语言约定。agent 和 runtime 之间只承认少量结构化控制动作。

```rust
enum TaskSpaceControlAction {
    Route { decision: TaskRoutingDecision },
    BindNode { binding: TaskActionBinding },
    CreateNode { map_id: TaskMapId, draft: NodeDraft },
    RebornMap { draft: TaskMapDraft },
    AskUser { question: String },
}
```

这些动作的职责：

| 动作 | 谁生成 | runtime 做什么 | 什么时候用 |
| --- | --- | --- | --- |
| `Route` | 主 agent | 校验 task/node，创建或切换 active task | 每轮用户输入后、无 binding 时、用户换话题时 |
| `BindNode` | 主 agent | 校验 node 属于 active map，并更新 `current_binding.node_id` 和 task 的 `last_main_node_id` 镜像 | 主 agent 要从一个 node 推进到另一个 node |
| `CreateNode` | 主 agent | 校验 `map_id == active_map_id` 和 node draft，追加到 active map | 发现新子任务且当前 map 没有对应 node |
| `RebornMap` | 主 agent | 校验 `draft.purpose=Reborn`，创建新 map 并切换 `active_map_id` | 用户执行 `/task-reborn` 后 |
| `AskUser` | 主 agent | 停止普通行动，把问题返回用户 | 无法安全 route、无法生成 map、缺关键上下文 |

`CreateNode` 的 `draft.local_id` 只用于这次控制动作内部。runtime 追加 node 时分配正式 `NodeId`，
并把该正式 id 返回到下一轮 TaskSpace manifest/current binding 中。主 agent 后续只能引用 runtime 暴露的正式 id。
新增 node 的初始状态固定为 `ready`，除非 agent 同一控制动作后立即 `BindNode` 到该 node。

`RebornMap` 只有在 `reborn_pending` 存在且 task/map 与当前 active task 匹配时才能被接受。
成功创建新 map 或 agent 返回 `AskUser` 后，runtime 清空 `reborn_pending`；校验失败时保留一次重试机会，
连续失败进入控制动作失败处理。

`TaskRoutingDecision::AskUser` 是 routing 阶段的结果，必须包在 `Route { decision }` 中提交。
`TaskSpaceControlAction::AskUser` 只用于非 routing 控制场景，例如 `/task-reborn` draft 无法生成或 binding 无法安全选择。

控制动作必须走专门解析路径，不能混在普通自然语言回答里。

第一版把控制动作实现成固定内部 pseudo-tool，而不是从自由文本中解析 JSON。
原因是 TaskSpace 的正确性依赖这些动作；如果靠 markdown code fence 或自然语言解析，失败模式会太多。

### 控制动作的物理通道

固定 tool name：

```text
taskspace_control
```

它是 runtime 内部 pseudo-tool：

- 不进入外部 tool registry。
- 不执行 shell、文件、网络或 MCP 副作用。
- 在普通 tool dispatch 之前被 TaskSpace runtime 拦截。
- 只用于提交 `TaskSpaceControlAction`。

wire schema：

```rust
TaskSpaceControlCall {
    action: TaskSpaceControlActionKind,
    payload: serde_json::Value,
}

enum TaskSpaceControlActionKind {
    Route,
    BindNode,
    CreateNode,
    RebornMap,
    AskUser,
}
```

runtime 解析顺序：

```text
handle_model_response(response_items):
  1. 预扫描本轮所有 tool call
  2. 如果发现多个 taskspace_control，拒绝整批 response items
  3. 如果发现一个 taskspace_control，先 schema 校验并应用控制动作
  4. 控制动作通过后，再处理同轮普通 tool call
  5. 控制动作失败时，丢弃同轮普通 tool call，普通 assistant text 不作为最终用户回答
  6. 如果 bootstrap_required 或 current_binding=None 且没有 taskspace_control，
     拒绝同轮普通 tool call，普通 assistant text 不作为最终用户回答
```

这意味着模型可以在同一轮先提交 `taskspace_control`，再调用普通工具；runtime 必须先应用控制动作，
然后才允许普通工具在新的 binding 下运行。如果控制动作不合法，本轮所有普通工具都不执行。

如果同一轮同时出现普通文本和控制动作：

- 控制动作优先。
- 控制动作通过前，普通文本不视为最终用户回答。
- 控制动作失败时，普通文本丢弃或作为 debug 附件记录，不能继续执行工具。

控制动作解析/校验失败的处理：

| 连续失败次数 | runtime 行为 |
| --- | --- |
| 1 | 记录 `last_control_error`，注入简短 error hint，让 agent 重新输出同一种控制动作 |
| 2 | 保持当前 binding 不变，禁止普通工具，要求 agent 选择 `AskUser` 或修正控制动作 |
| >=3 且 `bootstrap_required = true` 且 `ever_bootstrapped = false` | bootstrap 失败自动回滚到普通 session，清空 TaskSpace 临时状态，输出机械提示：`TaskSpace bootstrap 失败，已回到普通模式；可再次运行 /taskspace 重试。` |
| >=3 且已经成功 bootstrap 过 | 停止本轮自动推进，保留 TaskSpace 状态，向用户返回机械错误，请用户澄清或重试 |

失败时不能静默回退到上一条普通行动，也不能让 agent 绕过 TaskSpace 继续工具调用。

bootstrap 自动回滚不是用户可用的退出功能。它只发生在 `/taskspace` 启用后还没有创建过任何 active task/map 的阶段，
目的是避免用户被困在一个无法 bootstrap、又不能普通行动的 session 里。只要 `ever_bootstrapped = true`，
后续失败都不能自动退出 TaskSpace。

不要从自由文本正文里猜 JSON。约束是：

- 控制动作成功前，不执行普通工具。
- 控制动作失败时，返回机械错误和可恢复提示。
- 控制动作被 runtime 接受后，才更新 session state。
- 普通工具调用永远不负责创建或切换 task binding。

### 状态不变量

实现时要把这些不变量写成单元测试，而不是只靠 prompt。

| 编号 | 不变量 |
| --- | --- |
| I1 | `taskspace.enabled = false` 时，TaskSpace guard 不改变现有默认行为 |
| I2 | `taskspace.enabled = true` 时，普通工具调用必须有当前 `TaskActionBinding` |
| I3 | `active_task_id` 指向的 task 必须存在，且 status 必须为 `active` |
| I4 | 同一 session 同一时刻最多一个 task 为 `active` |
| I5 | active task 必须有 `active_map_id` |
| I6 | `active_map_id` 指向的 map 必须存在于该 task 的 `maps` |
| I7 | `current_binding.node_id` 必须存在于 active map |
| I8 | 普通工具结果必须记录到当前 node attribution |
| I9 | subagent spawn 必须先 claim ready node，再创建 child thread |
| I10 | 一个 node 同时最多有一个 active lease |
| I11 | subagent result 只能写回其 lease 绑定的 node |
| I12 | `/task-reborn` 校验失败时不得切换 `active_map_id` |
| I13 | 控制动作解析/校验失败时不得执行普通工具 |
| I14 | node claim 与 lease 创建必须在同一 session state 锁内完成 |
| I15 | task switch 不得中断旧 task 中已 attach 的 subagent lease |

### 失败处理契约

| 场景 | runtime 行为 | agent/user 可恢复路径 |
| --- | --- | --- |
| `/taskspace` 后没有 active task | 阻止普通工具，注入 bootstrap required | agent 返回 `CreateTask` 或 `AskUser` |
| agent 返回不存在的 task id | 拒绝控制动作，不改变 binding | agent 基于最新 manifest 重试或询问用户 |
| agent 返回不存在的 node id | 拒绝控制动作，不改变 binding | agent 选择已有 node 或创建新 node |
| 控制动作解析失败 | 增加 failure count，注入 error hint，禁止普通工具 | agent 重试结构化控制动作或 AskUser |
| active task 缺 active map | 阻止普通工具，要求 map draft | agent 基于 task context 创建 map |
| active map 缺 current node | 阻止普通工具，要求 `BindNode` | agent 选择 ready/running node |
| 普通工具调用时无 binding | tool guard 拒绝执行 | 回到 routing/binding |
| `CreateNode.map_id != active_map_id` | 拒绝控制动作，不追加 node | agent 基于最新 active map 重试 |
| subagent spawn 时无 ready node | 拒绝 spawn，不创建 child thread | agent 创建 node、等待 running node、或询问用户 |
| node lease attach 失败 | 释放已 claim lease，node 回到 ready 或 blocked | agent 可重试 spawn |
| subagent 超时 | 请求进展总结，写入 lease node result/blocker，释放 lease，node 置 blocked | 主 agent 决定继续、重试、创建新 node |
| `/task-reborn` draft 无效 | 不切换 active map | agent 修正 draft 或询问用户 |
| bootstrap 连续失败达到阈值且还没有成功创建过 task | 自动回滚到普通 session | 用户可继续普通对话，或再次执行 `/taskspace` |
| viewer read 失败 | 不影响 runtime；返回机械错误 | 用户可重试 `/task-show` |

### 现有代码落点

TaskSpace 不应该另造一套并行 runtime。第一版应从当前 Action Map 能力演进。

现有可复用落点：

| 能力 | 现有路径 | TaskSpace 改造方式 |
| --- | --- | --- |
| session state | `third_party/codex-cli/codex-rs/core/src/state/session.rs` | 从 `action_map_runtime` 演进为或包裹进 `taskspace` |
| developer context 注入 | `third_party/codex-cli/codex-rs/core/src/session/mod.rs` 的 `build_initial_context` 附近 | 注入 TaskSpace manifest、current binding、bootstrap required |
| mode/command handler | `third_party/codex-cli/codex-rs/core/src/session/handlers.rs` | `/taskspace`、`/task-show`、`/task-reborn` 复用现有 op 分发形态 |
| subagent spawn hook | `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs` | 从 `prepare_action_map_spawn_assignment` 演进为 task/node lease claim |
| subagent close/release | `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs` | 继续释放 node lease |
| wait timeout summary | `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs` | 继续把超时总结写回 lease node |
| child result writeback | `third_party/codex-cli/codex-rs/core/src/agent/control.rs` 与 `Session::record_action_map_child_result` | 演进为 task node result writeback |
| result body read | 现有 thread/actionMap read 路径与 tool handler 形态 | 新增只读 `read_task_result` wrapper，按 `ResultId` 返回 full body |
| viewer server | `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs` | 改名或包裹为 TaskSpace viewer，复用本地 HTTP + polling |
| app-server read API | `thread/actionMap/read` | 第一版可兼容，后续新增 `thread/taskspace/read` |
| TUI slash dispatch | `third_party/codex-cli/codex-rs/tui/src/chatwidget/slash_dispatch.rs` | 新增 `/taskspace`、`/task-show`、`/task-reborn`，旧命令保留别名 |

集成方式分级：

| 级别 | 策略 | 使用场景 |
| --- | --- | --- |
| A | 包裹/演进现有 Action Map 类型 | `ActionMapRuntimeState` 到 `TaskSpaceRuntimeState` 的迁移 |
| B | 在现有 handler 前后加 guard/hook | `spawn_agent` claim lease、tool guard、developer context 注入 |
| C | 新增兼容 API/命令别名 | `/taskspace`、`/task-show`、`thread/taskspace/read` |
| D | 直接替换上游流程 | 第一版禁止，除非无法通过 A/B/C 实现 |

凡是改 `third_party/codex-cli` 路径，必须满足：

- standard/default 行为不变。
- TaskSpace 未启用时 guard 是 no-op。
- 新逻辑尽量收敛在 `action_map` 或新 `taskspace` 模块内。
- handler 只调用小而明确的 runtime API，不内联复杂状态机。
- 回归测试覆盖 TaskSpace disabled 与 enabled 两种路径。

不允许第一版新增：

- 独立数据库。
- 独立消息总线。
- 独立 subagent runtime。
- 语义检索路由器。
- 后台常驻 task scheduler。

### subagent

subagent 沿用现有 node lease 设计。

```text
spawn_agent
  -> TaskSpace guard
  -> active task
  -> active map
  -> claim ready node
  -> create lease
  -> inject node context into child prompt
  -> attach lease to child thread
```

如果没有 ready node：

- agent 可以先创建新 node。
- agent 可以询问用户。
- agent 可以等待已有 running node 完成。
- runtime 不应允许无 node spawn。

### node lease claim 原子性

`claim ready node -> create lease -> node running` 必须是 session state 锁内的一次原子 mutation。
不能先读出 ready node，再在锁外创建 lease，否则并发 spawn 时会出现多个 subagent 抢同一 node。

原子步骤：

```text
claim_node_for_subagent(task_id, map_id, requested_node?):
  lock session state
  verify taskspace.enabled
  verify task_id == active_task_id
  verify map_id == active_map_id
  choose requested ready node or first ready node
  if no ready node:
    return ClaimRejected(NoReadyNode)
  verify node.active_lease == None
  create lease_id
  node.status = running
  node.active_lease = lease_id
  leases[lease_id] = AssignmentLease { task_id, map_id, node_id, state: Claimed }
  emit task_node_lease_created
  unlock session state
```

child thread 创建成功后，再执行 attach：

```text
attach_lease_to_child_thread(lease_id, child_thread_id):
  lock session state
  verify lease.state == Claimed
  lease.state = Attached(child_thread_id)
  emit task_node_lease_attached
  unlock session state
```

如果 child thread 创建失败，必须释放 lease：

```text
release_claimed_lease(lease_id, reason = "spawn_failed"):
  lock session state
  node.active_lease = None
  node.status = ready
  lease.state = Released
  lease.released_at_ms = now
  lease.release_reason = reason
  emit task_node_lease_released
  unlock session state
```

### subagent 完成与 lease 释放

subagent 结束后必须在同一处完成 result 写回、node 状态切换、lease 状态切换，不能只写 result。

正常完成：

```text
record_subagent_result(lease_id, result):
  lock session state
  verify lease.state == Attached(child_thread_id)
  verify lease.task_id/map_id/node_id still exist
  write full result body and summary into map.results
  append result_id to node.result_ids
  node.status = completed or blocked
  node.active_lease = None
  lease.state = Completed
  lease.completed_at_ms = now
  advance downstream ready nodes if dependency edges are satisfied
  update task.updated_at_ms and manifest digest
  emit task_node_result_recorded
  emit task_node_lease_released
  unlock session state
```

超时、child crash、用户关闭 child thread：

```text
release_attached_lease(lease_id, reason):
  lock session state
  if progress summary exists:
    write it as node result summary/body
  node.status = blocked
  node.active_lease = None
  lease.state = Released
  lease.released_at_ms = now
  lease.release_reason = reason
  update task.updated_at_ms and manifest digest
  emit task_node_lease_released
  unlock session state
```

如果 result 写回的是 pending task：

- 仍然按原 lease 写回原 node。
- 允许推进该 pending task 内部的下游 ready 状态。
- 更新该 task 的 `summary_digest`、`last_active_node_title`、`updated_at_ms`。
- 不切换 `active_task_id`，不把该 task 自动变成 active。
- 下一轮 manifest 注入时，主 agent 可以看到 pending task 摘要已经变化。

claim 失败要返回明确错误：

```rust
enum ClaimRejected {
    NoReadyNode,
    NodeAlreadyLeased { node_id: NodeId },
    TaskNotActive { task_id: TaskId },
    MapMismatch { requested_map_id: TaskMapId, active_map_id: TaskMapId },
}
```

测试必须覆盖：同一个 active map 只有一个 ready node 时，并发请求 3 次 spawn，只有一个 claim 成功，其余返回 `ClaimRejected`。

## Task Map 初始化

Task 创建时，不创建空 BaseMap。

流程：

```text
1. runtime 暴露 BaseMap metadata 和候选节点。
2. agent 根据用户输入和当前上下文生成上下文化 task map。
3. runtime 校验结构。
4. runtime 分配正式 task/map/node id，建立 `local_id -> NodeId` 映射。
5. runtime 创建 task + map + nodes。
6. runtime 绑定 `TaskMapDraft.current_main_local_id` 对应的正式 node。
```

BaseMap 只提供候选节点，不是实际 task。

BaseMap metadata 是静态内置数据，不做领域 map 注册系统。第一版只保留一个 `base`：

```rust
BaseMapMetadata {
    version: String,
    candidate_nodes: Vec<BaseMapCandidateNode>,
}

BaseMapCandidateNode {
    key: String,
    title: String,
    description: String,
    category: String,
}
```

这些候选节点在 task map 创建和 reborn map 创建时一次性暴露给主 agent。runtime 不根据候选节点自动建图，
也不在后续每轮反复注入完整候选列表。
工程上可以放在 taskspace 模块内的静态常量或同目录小型 manifest 文件；第一版不需要动态加载、插件注册或语义检索。

候选节点示例：

- 确定边界
- 搜索资料
- 代码架构梳理
- 质量问题扫描
- 方案设计
- 日志设计
- 方案审查
- 方案实施
- 代码审查
- 冒烟测试
- 回归测试
- 最终合成

agent 可以选择、删除、改名、细化这些节点。runtime 只校验结构合法。

## 上下文压缩设计

这是 TaskSpace 的关键部分。TaskSpace 不能被普通 session 压缩揉成一段自然语言摘要，否则 task 结构会被破坏，重要 result 会丢失。

压缩必须分层处理：

```text
Session Transcript Summary
TaskSpace Manifest
Active Task Pack
Referenced Task Packs
Cold Pending Task Refs
```

### 压缩目标

压缩必须同时满足：

- 保留 session 对话连续性。
- 保留 task 列表和状态。
- 保留 active task 的执行结构。
- 保留 node/result 的可恢复引用。
- 避免把所有 task 结果全部塞进 prompt。
- 避免把 map/node 结构压成不可解析的自然语言。
- 允许 agent 在后续 turn 精确恢复当前 task。

### Session Transcript Summary

只总结 session 层对话，不承载 task 结构。

包含：

```text
- 用户最近关注点
- 重要偏好
- 近期模式切换或命令
- 与当前 active task 相关但尚未写入 task 的上下文
```

不包含：

```text
- 完整 node 列表
- 完整 result body
- lease 细节
- task map edge 细节
```

### TaskSpace Manifest

始终注入短 manifest，帮助 agent 判断当前输入属于哪个 task。

```rust
TaskManifestEntry {
    task_id: TaskId,
    title: String,
    objective_short: String,
    status: TaskStatus,
    last_active_at_ms: i64,
    last_active_node_title: Option<String>,
    summary_digest: String,
}
```

Manifest 要短。每个 task 控制在 1 到 3 行。
pending task 的后台 subagent result 写回时，也必须更新 manifest 中的 `summary_digest`、
`last_active_node_title` 和更新时间。runtime 不主动切回该 task，但要让主 agent 下一轮能看到这个 task 已经有新进展。

示例：

```text
TaskSpace manifest:
- task-1 [active] TaskSpace runtime design: define task layer and compression policy. Current node: compression design.
- task-2 [pending] task-show viewer: browser live viewer implemented and verified.
```

### Active Task Pack

active task 必须高保真注入。

```rust
TaskContextPack {
    task_id: TaskId,
    title: String,
    objective: String,
    status: TaskStatus,
    context_summary: String,
    source_refs: Vec<String>,
    active_map_id: TaskMapId,
    current_binding: TaskActionBinding,
    nodes: Vec<NodeContextSummary>,
    edges: Vec<EdgeSummary>,
    latest_results: Vec<ResultSummary>,
    open_questions: Vec<String>,
    blockers: Vec<String>,
}
```

Active Task Pack 注入规则：

- 注入所有 node 的 id/title/status/context_summary。
- 注入 edge 的轻量关系。
- 注入 current node 的详细上下文。
- 注入 running/blocked node 的详细摘要。
- 注入最近重要 results 的摘要，不注入完整 body。
- 完整 result 只通过 id 引用，必要时由工具读取。

### NodeContextSummary

```rust
NodeContextSummary {
    node_id: NodeId,
    title: String,
    status: NodeStatus,
    context_summary: String,
    source_refs: Vec<String>,
    result_refs: Vec<ResultRef>,
    lease: Option<LeaseSummary>,
}
```

### ResultSummary

```rust
ResultSummary {
    result_id: ResultId,
    node_id: NodeId,
    kind: NodeResultKind,
    summary: String,
    source_thread_id: ThreadId,
    created_at_ms: i64,
    body_available: bool,
}
```

完整 result body 不默认进入 prompt。否则一两个 subagent 大结果就会把 active task pack 膨胀失控。

### 读取完整 result body

既然 prompt 默认只注入 `ResultSummary`，agent 必须有一个按需读取完整 body 的只读工具。
第一版提供内部只读工具：

```rust
ReadTaskResultArgs {
    result_id: ResultId,
}

ReadTaskResultOutput {
    result_id: ResultId,
    task_id: TaskId,
    map_id: TaskMapId,
    node_id: NodeId,
    summary: String,
    body: String,
    created_at_ms: i64,
}
```

工具名可以是：

```text
read_task_result
```

权限规则：

- 只能在 `taskspace.enabled = true` 时调用。
- `result_id` 必须出现在本轮已注入的 active task pack 或 referenced task pack 中。
- runtime 只按 id 查找，不做语义搜索。
- 该工具不改变 task/map/node 状态，只记录只读 audit event。
- 如果 result 属于 cold pending task，agent 必须先通过 `SwitchTask` 或显式 referenced pack 让该 task 进入本轮可见范围，不能绕过 manifest 随机读取历史结果。

“本轮已注入”的判定由 turn context 持有，不从 prompt 文本反推：

```rust
TaskSpaceTurnExposure {
    injected_task_ids: HashSet<TaskId>,
    injected_map_ids: HashSet<TaskMapId>,
    injected_result_ids: HashSet<ResultId>,
}
```

每次渲染 TaskSpace manifest / active pack / referenced pack 时同步生成这个集合。
`read_task_result` 只检查 `injected_result_ids`，不会扫描全部历史结果。

### Referenced Task Packs

当主 agent 判断用户输入指向 pending task，并返回 `SwitchTask` decision 时，runtime 注入对应 task pack。

runtime 不根据关键词或相似度主动选择 referenced task pack。它最多把 TaskSpace manifest
暴露给主 agent；主 agent 做出 `SwitchTask` 后，runtime 再按 task id 读取对应 pack。

Referenced pack 比 active pack 更短：

```text
- task objective
- context summary
- last active node
- completed/blocked node summary
- latest important result summary
```

不注入完整 edge 和完整 result body。

### Cold Pending Task Refs

较久未激活的 pending task 默认不注入完整 task pack，只保留 manifest 和可读取路径。

后续如果需要，可以通过 task viewer/API 查询。

### 防止结构破坏

压缩时必须保存结构化快照，而不是只保存自然语言。

建议新增：

```rust
TaskSpaceCompressionSnapshot {
    schema_version: u32,
    manifest: Vec<TaskManifestEntry>,
    active_task: Option<TaskContextPack>,
    referenced_tasks: Vec<TaskContextPack>,
    cold_pending_task_refs: Vec<TaskArchiveRef>,
    exposure: TaskPackExposureSnapshot,
}

TaskPackExposureSnapshot {
    active_task_id: Option<TaskId>,
    referenced_task_ids: Vec<TaskId>,
    cold_pending_task_ids: Vec<TaskId>,
}
```

压缩后的 developer context 由该 snapshot 渲染出来。

不要让模型从普通 session summary 中反推 task map。task map 必须由 runtime state 持有。

snapshot 存储位置：

- 最新 snapshot 保存在 `TaskSpaceRuntimeState.latest_compression_snapshot`。
- 每次创建 snapshot 时发出 `taskspace_compression_snapshot_created` rollout event，供 replay 恢复。
- 不新增独立文件或独立数据库。
- replay 时先用事件恢复 runtime state，再用最新 snapshot 渲染 developer context；snapshot 不能替代 authoritative task/map state。
- `exposure` 只用于压缩预算和调试，明确哪些 pending task 本轮被升温为 referenced pack；
  它不改变 `TaskStatus`，也不作为 task routing 的输入判断。

### 防止重要信息丢失

每个 node/result 写入时必须生成两层信息：

```text
full body       完整内容，用于 viewer/API 按需读取
summary         压缩友好摘要，用于 prompt 注入
```

summary 生成策略：

- subagent result 写入 node 时，先保存 full body。
- 如果 body 很短，summary 可以等于 body。
- 如果 body 很长，要求 agent 或 compactor 生成 node-local summary。
- summary 必须包含：
  - 结论
  - 关键证据
  - 涉及文件/符号
  - 未解决问题
  - 对下游 node 的影响

Result summary 不是质量评分，不判断对错，只保留可恢复信息。

### 压缩预算

第一版需要硬预算，而不是只给大致范围。

建议常量：

```rust
const TASKSPACE_MAX_PROMPT_TOKENS: usize = 32_000;
const TASKSPACE_MANIFEST_MAX_TOKENS: usize = 1_000;
const ACTIVE_TASK_PACK_MAX_TOKENS: usize = 12_000;
const REFERENCED_TASK_PACK_MAX_TOKENS: usize = 3_000;
const REFERENCED_TASK_PACK_MAX_COUNT: usize = 3;
const RESULT_FULL_BODY_INLINE_MAX_TOKENS: usize = 800;
```

预算目标：

```text
TaskSpace manifest: 1K tokens 以内
Active task pack: 4K-12K tokens
Referenced task pack: 每个 1K-3K tokens
Full result body: 默认不注入
Cold pending task refs: 只注入 manifest
```

如果 active task pack 超预算：

1. 保留 task objective。
2. 保留 current node。
3. 保留 running/blocked nodes。
4. 保留最近 important results。
5. completed nodes 只保留一行摘要。
6. 最后才裁剪 source_refs。

不要裁剪 task id、node id、result id。它们是恢复结构的主键。

整体超预算时的淘汰顺序：

1. 淘汰 referenced task packs，按 `last_active_at_ms` 从旧到新裁剪。
2. 裁剪 referenced task pack 中的 completed/blocked node summary，只保留 latest important result。
3. 裁剪 active task pack 中非 current、非 running、非 blocked 的 completed node summary。
4. 裁剪 active task pack 中 result summary 的低优先级 source refs。
5. 压缩 task context summary，但保留 objective、open questions、blockers。
6. 最后才裁剪 manifest 的 summary_digest，不能裁 task id/title/status。

Cold pending task 升温规则：

- runtime 不主动根据关键词升温。
- agent 返回 `SwitchTask(task_id)` 后，runtime 读取该 task pack 并在下一轮注入。
- 如果 task pack 过大，先注入 referenced pack，再由 agent 按需请求 viewer/API 读取 full body。

### 压缩后的恢复规则

压缩后下一轮 agent 必须看到：

```text
TaskSpace is enabled.
You must work inside active task and current node.
Do not infer task structure from chat history if TaskSpace snapshot disagrees.
Use TaskSpace snapshot as the source of truth.
```

如果 snapshot 损坏或 active task 缺失：

- runtime 阻止普通行动。
- 触发 taskspace repair。
- repair 只能基于 manifest/rollout 重建最小 task。
- repair 失败则询问用户。

### repair 机制

repair 不是语义修复，只修复 runtime state 的结构一致性。

触发点：

- session load / replay 完成后。
- 每轮 turn-level hook 前。
- viewer/API 读取 snapshot 前。
- tool-level guard 发现 invariant 失败时。

repair 输入：

```text
SessionState.taskspace
rollout taskspace events
latest TaskSpaceCompressionSnapshot
```

repair 规则：

| 失败不变量 | repair 行为 |
| --- | --- |
| `active_task_id` 指向不存在 task | 清空 `active_task_id`，设置 `bootstrap_required = true` |
| 没有 active task，但存在 status=active 的 task | 选择最近 `last_active_at_ms` 最大的 active task |
| 多个 task status=active | 只保留 `active_task_id` 指向的 task 为 active，其余置 pending |
| `active_task_id` 指向的 task status=pending | 将该 task status 修正为 active，记录 repair note |
| active task 缺 `active_map_id` | 设置 `bootstrap_required = true`，要求 agent 生成 map |
| `active_map_id` 不存在 | 选择该 task 最新非 historical map；没有则 bootstrap |
| `current_binding` 缺失或指向不存在 node | 优先用 active task 的有效 `last_main_node_id` 重建 binding；没有则选择 active map 第一个 ready/running node；仍没有则 bootstrap |
| lease 指向不存在 node | 释放 lease，记录 repair note |
| node.active_lease 指向不存在 lease | 清空 node.active_lease；如果 node.status=running，降级为 blocked，并追加 repair note |
| result 指向不存在 node | 保留 result 为 orphan result，不注入 prompt，只在 viewer debug 区显示 |

repair 不能做：

- 不能自动选择用户语义上应该继续哪个 task。
- 不能自动生成新 task map。
- 不能删除旧 map/result。
- 不能把 pending task 变成 active，除非 `active_task_id` 明确指向它。
- 不能把丢失 lease 的 running node 自动降级为 ready，因为原 subagent 是否仍在执行不可知。

repair 失败时：

- 阻止普通工具。
- 输出机械错误。
- 要求 agent 返回 `AskUser` 或重新 bootstrap。

## 持久化与 replay

第一版继续复用现有 session state + rollout event，不新增独立 DB。

需要新增事件：

```text
taskspace_enabled
task_created
task_activated
task_pending
task_note_recorded
task_context_updated
task_map_created
task_map_reborn
task_node_created
task_node_status_changed
task_main_node_bound
task_node_lease_created
task_node_lease_attached
task_node_lease_released
task_node_result_recorded
task_result_body_read
taskspace_control_action_failed
taskspace_bootstrap_failed_reset
taskspace_repair_applied
taskspace_compression_snapshot_created
```

replay 时：

1. 先恢复 taskspace enabled。
2. 重放 task lifecycle。
3. 重放 task map/node/lease/result。
4. 恢复 active_task_id。
5. 从 active task 的 `active_map_id` 和有效 `last_main_node_id` 重建 `current_binding`；如果无法重建则进入 bootstrap/repair。

如果事件不完整，优先恢复 manifest 和 active task；无法确认 lease 的 running node 必须保守降级为 blocked 并记录 repair note。

## Prompt 设计

### 进入 TaskSpace 后的系统约束

```text
TaskSpace is enabled for this session.
You must maintain work as task-scoped state.
Before ordinary tool use, code edits, or subagent spawning, select or create an active task and bind the current action to a task node.
When routing, binding, creating nodes, or creating a reborn map, call the internal taskspace_control pseudo-tool.
Do not express TaskSpace control decisions as natural-language JSON in the assistant message.
Do not expose internal map/lease terminology to the user unless debugging.
Use task/node state as the source of truth for ongoing work.
If the user changes topic, route the input to an existing task or create a new task.
If unsure, ask a short clarification question.
```

### Task routing prompt

```text
Given the latest user input and the TaskSpace manifest, choose exactly one:
1. continue the active task
2. switch to an existing pending task
3. create a new task
4. ask the user to clarify

Return only a structured task routing decision.
Submit it with taskspace_control(action=Route).
Do not perform ordinary work before this decision is accepted by runtime.
```

### Task map creation prompt

```text
Create a lightweight task path for the selected task.
Use the candidate node list only as guidance.
Prefer concrete nodes that match the user's current objective.
Every node must have a clear title and local context summary.
Use draft-local node ids only. The runtime will assign final node ids.
Keep the first map small; it can grow later.
Select the node that the main agent should work on now.
```

## Viewer 设计

用户命令：

```text
/taskspace
/task-show
```

`/taskspace` 成功开启后必须立即把 viewer URL 打印到对话流中。
这是用户第一次进入任务空间后的默认可观测入口，避免用户还要猜测 `/task-show` 的存在。

`/task-show` 用于后续再次查看同一个 viewer。

第一版 viewer 保持极简：

```text
left: task list
main: active task path
bottom/right: selected node results
```

不要做复杂样式，不引入前端构建。

API：

```text
thread/taskspace/read
```

返回：

```rust
TaskSpaceSnapshot {
    enabled: bool,
    active_task_id: Option<TaskId>,
    tasks: Vec<TaskSnapshot>,
}
```

其中 active task 带详细 map，pending task 默认只带 manifest。

URL 行为：

- viewer server 按 session/thread 复用，避免每次命令打开新端口。
- URL 可以是 session-scoped local URL，例如 `http://127.0.0.1:<port>/`。
- `/taskspace` 和 `/task-show` 打印同一个 URL。
- viewer 页面自动刷新读取 `thread/taskspace/read`，不是一次性静态快照。
- URL 打印是机械状态输出，不是 agent 自然语言回答。

V1 viewer 明确为只读：

- polling interval = 2s。
- 不提供写按钮。
- 不允许从 viewer 触发 node 状态变更、task switch、task reborn。
- 所有写操作必须走 slash command 或 agent 控制动作。
- viewer 读取失败只显示错误状态，不影响 runtime。
- session 结束后 viewer server 可以立即关闭；历史回看依赖后续 replay/viewer 能力，不是 V1 阻塞项。

## 与当前 Action Map 实现的迁移

不要一次重写所有代码。推荐三阶段。

### 阶段 1：包一层 TaskSpace

目标：行为尽量不变。

改动：

- 新增 `TaskSpaceRuntimeState`。
- 把现有 `ActionMapRuntimeState` 放入默认 active task。
- `/taskspace` 进入 TaskSpace 模式。
- 旧 `map-*` slash 命令不再作为用户可见命令保留。
- `/task-show` 打开 taskspace viewer。
- `/taskspace` enable 成功后自动启动或复用 viewer，并打印 URL。

### 阶段 2：强制 task bootstrap/sync

目标：解决“开启后没有 map/task”的问题。

改动：

- `/taskspace` 后设置 `bootstrap_required`。
- 下一轮 agent 行动前必须产生 task routing decision。
- 无 active task 时禁止普通工具调用。
- 有 active task 但无 current node 时禁止普通工具调用。
- 主 agent 工具调用归属到 current node。

### 阶段 3：多 task manifest

目标：一个 session 支持多个任务。

改动：

- agent routing decision 可以 create/switch task。
- active task 切走时旧 task -> pending。
- viewer 展示 task list。
- compression 注入 manifest + active task pack。

## 测试方案

### 单元测试

- `/taskspace` 只能 enable，不能 disable。
- `/taskspace` enable 成功后会打印 TaskSpace viewer URL。
- 重复执行 `/taskspace` 不改变 task 状态，但会重新打印同一个 viewer URL。
- `/task-show` 会打印或打开同一个 viewer URL，不改变 runtime 状态。
- enable 后无 active task 时，普通工具行动被阻止。
- `taskspace_control` pseudo-tool 会在普通 tool dispatch 前被解析和应用。
- `taskspace_control.action` 使用 enum 反序列化，非法 action 直接 schema error。
- 同一轮多个 `taskspace_control` 会拒绝整批 response items。
- `bootstrap_required` 或 `current_binding=None` 时，没有 `taskspace_control` 的普通文本和普通工具都不会成为最终行动。
- task routing create_task 后生成 active task。
- `CreateTask.initial_map` 与 `RebornMap.draft` 共享 `TaskMapDraft` 和 `create_task_map_from_draft`。
- `TaskMapDraft` 使用 `NodeDraft.local_id`，runtime 分配正式 `NodeId` 并正确改写 edge 和 main node。
- 控制动作解析/校验失败时，普通工具不会执行，并记录 failure count。
- bootstrap 前连续控制动作失败达到阈值时自动回滚到普通 session，成功 bootstrap 后不允许自动退出 TaskSpace。
- active task 必须有 active map 和 `current_binding`。
- `current_binding` 是主 agent 当前 node 的唯一权威，`TaskState.last_main_node_id` 只是恢复/viewer 镜像。
- 普通工具调用不需要携带 task/map/node id，会自动归属当前 `TaskActionBinding`。
- 普通工具结果会写入当前 node 的 `NodeResultKind::MainToolCall`。
- 无当前 `TaskActionBinding` 时，普通工具调用被 guard 拒绝。
- 切换 task/node 必须先提交结构化 routing/binding 控制动作。
- `CreateNode.map_id` 必须等于当前 `active_map_id`。
- `CreateNode` 创建的新 node 初始状态为 `ready`。
- `TaskNote` 存储在 `TaskState.notes`，SwitchTask 的 context update 能落盘。
- `EdgeKind::Dependency` 按所有上游 completed 的 AND 规则推进下游 ready，`Related` 不阻塞。
- 没有 incoming dependency 的 node 在 map 创建后初始为 `ready`。
- 并发 subagent spawn 争抢同一个 ready node 时，只有一个 claim 成功，其余返回 `ClaimRejected`。
- subagent 正常完成后，result 写入 node，lease -> Completed，`node.active_lease` 清空。
- subagent 超时/崩溃/关闭后，进展总结写入 node，lease -> Released，node -> blocked，`node.active_lease` 清空。
- task switch 会把旧 active task 置 pending，并保存 `previous_task_context_update`。
- task switch 不会中断旧 task 中已经 attach 的 subagent lease，后续 result 仍写回旧 node。
- pending task 的异步 result 写回会更新 manifest 摘要，但不会自动切换 active task。
- task reborn 会保留旧 map 为历史路径，并创建新 map。
- `RebornMap` 只有在 `/task-reborn` 设置 `reborn_pending` 后才能被接受。
- task reborn 不改变 task id，不删除旧 map/result，并重新打印 viewer URL。
- task reborn 的新 map 由主 agent 输出含 `local_id` 的 `TaskMapDraft`，runtime 分配正式 id。
- task reborn 校验失败时不切换 `active_map_id`。
- task reborn 后新 map 有 `parent_map_id`，active pack 只注入当前 map + 父 map摘要。
- repair 能修复 `active_task_id`、`active_map_id`、`current_binding` 指向不存在对象的结构错误。
- repair 遇到 `active_task_id` 指向 pending task 时会修正为 active 并记录 repair note。
- repair 遇到 running node 的缺失 lease 时必须降级 blocked，不能自动 ready。
- `read_task_result` 只能读取本轮 active/referenced pack 已暴露的 result id，且不改变 runtime 状态。
- `TaskSpaceTurnExposure.injected_result_ids` 决定 `read_task_result` 的可读范围。

### 压缩测试

- compression snapshot 保留 task ids、node ids、result ids。
- active task pack 保留 current node。
- full result body 不默认注入。
- result summary 保留关键 source refs。
- active/pending task 不会被揉进 session summary。
- 压缩后 developer context 仍包含 TaskSpace enabled 和当前 node 约束。
- TaskSpace prompt 注入总量不超过 `TASKSPACE_MAX_PROMPT_TOKENS`。
- referenced task packs 超预算时按 `last_active_at_ms` 从旧到新淘汰。
- compression snapshot 保存 `TaskPackExposureSnapshot`，能区分 active/referenced/cold pending。
- cold pending task 只有在 agent 返回 `SwitchTask` 后才升温为 referenced pack。

### 真实 E2E

场景 1：新 session 进入 taskspace 后做项目质量分析。

期望：

- agent 先创建 task。
- task 有上下文化 objective。
- task map 至少包含边界、架构梳理、质量扫描等节点。
- 主 agent 当前行动绑定 node。
- `/taskspace` 命令输出中已经包含 viewer URL。
- `/task-show` 可看到 task。

场景 2：同一 session 临时换话题。

期望：

- 当前 task 置 pending。
- 新话题创建新 task 或询问用户。
- 旧 task 不被污染。

场景 3：回到旧任务。

期望：

- runtime 暴露 task manifest。
- 主 agent 选择旧 pending task 并返回 `SwitchTask`。
- runtime 校验 task id 存在且状态为 pending。
- 注入 referenced task pack。
- 旧 task 恢复为 active。

场景 4：压缩后继续任务。

期望：

- task manifest 仍存在。
- active task map 结构仍存在。
- current node 没丢。
- agent 不把旧 session summary 当成 task source of truth。

场景 5：切换 task 时旧 task 有 running subagent。

期望：

- 主 agent 返回 `SwitchTask`。
- 旧 task 变 pending，并写入 `previous_task_context_update`。
- running lease 不被中断。
- subagent 完成后 result 写回旧 task 的原 node。
- active task 不被 subagent result 自动切回。

场景 6：用户执行 `/task-reborn`。

期望：

- runtime 构造 `RebornContext`。
- 主 agent 返回 `TaskMapDraft`。
- runtime 校验通过后创建新 map。
- 新 map 的 `parent_map_id` 指向旧 active map。
- 旧 map 标记为 historical。
- `active_map_id` 指向新 map。
- viewer 可以看到当前路径和历史路径。

场景 7：控制动作解析失败。

期望：

- 普通工具不执行。
- runtime 记录 `last_control_error` 和 `consecutive_control_failures`。
- bootstrap 前失败达到阈值后回滚普通 session，并提示用户可重新 `/taskspace`。
- bootstrap 成功后失败达到阈值只停止本轮推进，不退出 TaskSpace。

场景 8：pending task 的后台 subagent 完成。

期望：

- result 写回 pending task 的原 node。
- lease 释放，node 进入 completed 或 blocked。
- pending task manifest 摘要更新。
- active task 不被自动切换。
- 用户回到该任务时，agent 能看到最新摘要并决定继续。

场景 9：读取完整 result。

期望：

- prompt 只包含 `ResultSummary`。
- agent 可用 `read_task_result(result_id)` 读取本轮可见 result 的完整 body。
- 读取不改变 task/map/node 状态。
- 未在 active/referenced pack 暴露的 result id 被拒绝。

## 与旧概念的对应关系

```text
历史“进入 map 实验模式”命令  -> 新 /taskspace，旧 slash 命令不保留
旧 Action Map Runtime    -> 新 TaskSpace 内部 Task Map Runtime
旧 active_map_id         -> 新 active_task_id + task.active_map_id
历史“查看 map”命令         -> 新 /task-show，旧 slash 命令不保留
历史“重启 map”命令         -> 新 /task-reborn，旧 slash 命令不保留
```

## 非目标

第一版不做：

- task 质量分。
- 语义向量检索。
- 多 active task 并行主控。
- 跨 session task 接管。
- 独立数据库。
- 复杂可视化 UI。
- 用户可退出 TaskSpace。

跨 session task 发现可以后续做，但不是第一版阻塞项。

## 最小可交付标准

第一版 TaskSpace 成立的标准：

- 用户执行 `/taskspace` 后，下一次 agent 行动前必定产生 active task。
- agent 普通行动必须绑定 task/node。
- subagent 行动必须绑定 node lease。
- 一个 session 可以出现多个 task。
- task 切换不会污染旧 task。
- 压缩后 task manifest 和 active task map 结构不丢。
- viewer 展示的是 TaskSpace，而不是要求用户理解 map。
