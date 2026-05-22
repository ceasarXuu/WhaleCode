# TaskSpace Runtime 设计

日期：2026-05-22

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
- `/task-show`：查看任务空间状态。第一版也可以让 `/map-show` 作为兼容别名打开同一 viewer。
- `/task-restart`：当前任务换一条思路重新开始。第一版可继续用 `/map-restart` 作为兼容别名。

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
3. 下一次 agent 行动前必须执行 task bootstrap/sync。
4. bootstrap/sync 完成前禁止普通工具行动、代码修改和 subagent spawn。

如果当前 session 已经进入 TaskSpace：

1. 返回机械状态：TaskSpace already enabled。
2. 不重复创建 task。
3. 不改变 active task。

不提供 `/taskspace off`。如果用户需要完全回到普通 session，应新开 session。

### `/task-show`

打开本地 browser viewer，展示 task list、active task、task map、node、lease、result。

第一版可以复用现有 `/map-show` viewer 技术路径，但用户文案应改成 TaskSpace。

### `/task-restart`

重启当前 active task 的执行路径，而不是重启整个 session。

语义：

- 当前 active task 保留 task id 和目标。
- 当前 active task 的旧 map 只作为历史路径保留，不再参与当前执行。
- 创建新的 task map。
- agent 需要基于当前 task context 重新生成初始 nodes。

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
    next_task_seq: u64,
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

    current_main_node_id: Option<NodeId>,
    created_at_ms: i64,
    updated_at_ms: i64,
    last_active_at_ms: i64,
}
```

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

### TaskMapState

用户不直接看到 `TaskMapState` 这个概念。代码层面第一版可以继续复用现有 `ActionMapInstance`，但在协议和 UI 上称为 task path 或 task map。

```rust
TaskMapState {
    id: TaskMapId,
    title: String,
    status: TaskMapStatus,
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

## Runtime 约束

进入 TaskSpace 后，runtime 必须满足：

```text
每次主 agent 行动前：
  必须有 active_task_id
  必须有 active task map
  必须有 current_main_node_id

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
        context_update: String,
        main_node_id: NodeId,
    },
    CreateTask {
        title: String,
        objective: String,
        context_summary: String,
        initial_nodes: Vec<NodeDraft>,
        initial_edges: Vec<EdgeDraft>,
        main_node_id: NodeId,
    },
    AskUser {
        question: String,
    },
}
```

runtime 校验输出：

- `ContinueTask.task_id` 必须存在。
- `SwitchTask.task_id` 必须存在。
- `CreateTask` 必须至少有一个 node。
- `main_node_id` 必须存在于 active map。
- edge 不能引用不存在的 node。
- active map 至少有一个 ready/running/current node。

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

主 agent 不需要 lease，但必须有 `current_main_node_id`。

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

每次 turn 开始时，routing/bootstrap/sync 必须给出当前 node。runtime 将它记录到 task。

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

## Task Map 初始化

Task 创建时，不创建空 BaseMap。

流程：

```text
1. runtime 暴露 BaseMap metadata 和候选节点。
2. agent 根据用户输入和当前上下文生成上下文化 task map。
3. runtime 校验结构。
4. runtime 创建 task + map + nodes。
5. runtime 绑定 main_node_id。
```

BaseMap 只提供候选节点，不是实际 task。

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
    active_node_title: Option<String>,
    summary_digest: String,
}
```

Manifest 要短。每个 task 控制在 1 到 3 行。

示例：

```text
TaskSpace manifest:
- task-1 [active] TaskSpace runtime design: define task layer and compression policy. Current node: compression design.
- task-2 [pending] map-show viewer: browser live viewer implemented and verified.
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
    current_main_node_id: NodeId,
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
    kind: String,
    summary: String,
    source_thread_id: ThreadId,
    created_at_ms: i64,
    body_available: bool,
}
```

完整 result body 不默认进入 prompt。否则一两个 subagent 大结果就会把 active task pack 膨胀失控。

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
}
```

压缩后的 developer context 由该 snapshot 渲染出来。

不要让模型从普通 session summary 中反推 task map。task map 必须由 runtime state 持有。

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

第一版建议：

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

## 持久化与 replay

第一版继续复用现有 session state + rollout event，不新增独立 DB。

需要新增事件：

```text
taskspace_enabled
task_created
task_activated
task_pending
task_context_updated
task_map_created
task_node_created
task_node_status_changed
task_main_node_bound
task_node_lease_created
task_node_lease_attached
task_node_result_recorded
taskspace_compression_snapshot_created
```

replay 时：

1. 先恢复 taskspace enabled。
2. 重放 task lifecycle。
3. 重放 task map/node/lease/result。
4. 恢复 active_task_id。
5. 恢复 current_main_node_id。

如果事件不完整，优先恢复 manifest 和 active task，lease 可以降级为 unattached/running unknown。

## Prompt 设计

### 进入 TaskSpace 后的系统约束

```text
TaskSpace is enabled for this session.
You must maintain work as task-scoped state.
Before ordinary tool use, code edits, or subagent spawning, select or create an active task and bind the current action to a task node.
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
Do not perform ordinary work before this decision is accepted by runtime.
```

### Task map creation prompt

```text
Create a lightweight task path for the selected task.
Use the candidate node list only as guidance.
Prefer concrete nodes that match the user's current objective.
Every node must have a clear title and local context summary.
Keep the first map small; it can grow later.
Select the node that the main agent should work on now.
```

## Viewer 设计

用户命令：

```text
/task-show
```

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

## 与当前 Action Map 实现的迁移

不要一次重写所有代码。推荐三阶段。

### 阶段 1：包一层 TaskSpace

目标：行为尽量不变。

改动：

- 新增 `TaskSpaceRuntimeState`。
- 把现有 `ActionMapRuntimeState` 放入默认 active task。
- `/map-mode experiment` 的内部语义迁移为 `/taskspace`。
- 保留 `/map-mode` 作为 debug legacy 命令。
- `/map-show` 打开 taskspace viewer，兼容旧命令。

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
- enable 后无 active task 时，普通工具行动被阻止。
- task routing create_task 后生成 active task。
- active task 必须有 active map 和 current_main_node_id。
- subagent spawn 必须 claim ready node。
- task switch 会把旧 active task 置 pending。
- task restart 会保留旧 map 为历史路径，并创建新 map。

### 压缩测试

- compression snapshot 保留 task ids、node ids、result ids。
- active task pack 保留 current node。
- full result body 不默认注入。
- result summary 保留关键 source refs。
- active/pending task 不会被揉进 session summary。
- 压缩后 developer context 仍包含 TaskSpace enabled 和当前 node 约束。

### 真实 E2E

场景 1：新 session 进入 taskspace 后做项目质量分析。

期望：

- agent 先创建 task。
- task 有上下文化 objective。
- task map 至少包含边界、架构梳理、质量扫描等节点。
- 主 agent 当前行动绑定 node。
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

## 与旧概念的对应关系

```text
旧 /map-mode experiment  -> 新 /taskspace
旧 Action Map Runtime    -> 新 TaskSpace 内部 Task Map Runtime
旧 active_map_id         -> 新 active_task_id + task.active_map_id
旧 /map-show             -> 新 /task-show，旧命令保留为别名
旧 /map-restart          -> 新 /task-restart，旧命令保留为别名
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
