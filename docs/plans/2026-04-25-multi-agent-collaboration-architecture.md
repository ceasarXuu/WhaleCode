# WhaleCode Multi-Agent 架构设计

日期：2026-04-25
更新：2026-04-30

## 结论

WhaleCode 的 multi-agent 第一版只验证一个核心假设：

```text
Action Map + 结构化 Artifact + Gate
是否比当前写信式 subagent 委派更稳定、更可控、更可复盘。
```

除此之外，所有未经真实任务验证、缺乏强推理实证的概念都不进入核心 runtime。

本文只描述当前要实现和验证的最小 runtime。未被真实任务证明必要的组织形态、投票机制、竞争机制和控制面机制，不在本文保留。

## 最高原则：Occam-first

设计约束：

- 不提前实体化没有被验证的协作概念。
- 不用角色清单解释系统能力。
- 不把 prompt 约定伪装成 runtime contract。
- 不为了“multi-agent first”而制造多 agent 仪式。
- 不维护两套表达同一规则的对象。
- 不让实验模式破坏当前可用的 Codex subagent 默认行为。
- 不设计全局质量分。复杂 agent 任务没有客观准确的单一质量分，系统只能记录证据、验证结果、阻塞点和人工/模型审查意见。

任何新 runtime 概念进入核心前，必须能回答：

```text
+ 没有它，当前系统出现了什么明确失败？
+ 这个失败是否能在真实任务或最小实验中复现？
+ 引入它以后，是否能通过可复现失败、可观察症状或明确人工反馈证明它减少了问题，而不是制造复杂度？
```

答不上来，就不进入核心设计。

## 模式开关

Multi-agent 框架必须可插拔。

```text
/multi-agents standard
  当前默认模式。
  继续使用 Codex-style subagent/thread/message/wait 行为。
  主 agent 可直接 spawn/send/wait/close。
  不强制 Action Map、Artifact、Gate。

/multi-agents standart
  兼容拼写别名，行为等同 standard。
  CLI 应提示 canonical name 是 standard。

/multi-agents experiment
  实验模式。
  启用 Action Map Runtime。
  后续 multi-agent 行为必须绑定 map node、artifact 和 gate。
```

第一阶段状态应是 session-scoped，不改变全局默认值。runtime 只需要记录当前模式、active map id 和切换 turn。

切换规则：

- `standard -> experiment`：下一次需要 multi-agent 协作时创建或复用 `ActionMapInstance`。
- `experiment -> standard`：停止对新行为施加 Action Map 约束；已运行 subagent 不强杀。
- 切换不清空 session、rollout、compact 历史或 agent registry。
- 每次切换必须写 session event，便于 replay。

## 最小运行模型

核心链路只有这些对象：

```text
UserTask
  -> ActionMapTemplate
  -> ActionMapInstance
  -> MapNode
  -> NodeExecution
  -> AgentAssignment
  -> Artifact
  -> Gate
  -> MapEvent
```

这些对象的职责边界：

| 对象 | 职责 |
| --- | --- |
| `ActionMapTemplate` | 任务类型的父类方法论模板 |
| `ActionMapInstance` | 当前任务的小队行动图和事实源 |
| `MapNode` | 一个可执行行动点 |
| `NodeExecution` | 某个节点本次如何执行 |
| `AgentAssignment` | 发给 agent 的具体工作包 |
| `Artifact` | agent 提交的结构化产物，记录证据、结论和限制 |
| `Gate` | 根据可检查条件判断节点或阶段是否可推进 |
| `MapEvent` | 状态变化记录，用于 replay 和审计 |

系统主循环：

```mermaid
flowchart TD
    U["User Task"] --> T["Select ActionMapTemplate"]
    T --> M["Instantiate ActionMapInstance"]
    M --> N["Ready MapNode"]
    N --> E["NodeExecution"]
    E --> A["AgentAssignment"]
    A --> R["Agent executes freely inside assignment"]
    R --> F["Artifact"]
    F --> G["Gate"]
    G -->|pass| NX["Next Node / Complete"]
    G -->|fail| RV["Revise Node / Ask User / Retry"]
    NX --> M
    RV --> M
```

并行只是 `NodeExecution` 的一种策略，不再单独抽象成一套组织系统。

## Action Map Template

Template 是父类地图，不是固定流程。它只提供某类任务的默认思路。

第一阶段只维护少量模板：

| Template | 适用任务 | 默认思路 |
| --- | --- | --- |
| `ArchitectureMapTemplate` | 架构梳理、优化、治理 | 定界 -> 现状建模 -> 风险扫描 -> 方案 -> 验证 |
| `BugDiagnosisMapTemplate` | 复杂 bug | 现象固定 -> 假设 -> 证据 -> 根因 -> 验证 |
| `FeatureMapTemplate` | 新功能 | 需求定界 -> 设计 -> 实现 -> 测试 -> 验收 |
| `RefactorMapTemplate` | 重构 | 行为基线 -> 小步改造 -> 回归验证 |

Template 不能规定 agent 每一步怎么思考，只能定义：

- 默认节点类型。
- 默认上下文边界。
- 默认 artifact 类型。
- 默认 gate 条件。

## Action Map Instance

Instance 是当前任务的小队运行状态，也是 experiment 模式的事实源。

```rust
pub struct ActionMapInstance {
    pub id: ActionMapId,
    pub template_id: TemplateId,
    pub user_goal: String,
    pub status: MapStatus,
    pub graph_version: GraphVersion,
    pub nodes: Vec<MapNode>,
    pub artifacts: Vec<ArtifactRef>,
    pub events: Vec<MapEventRef>,
}
```

`MapStatus` 第一版只需要：

```text
created -> running -> completed
              |
              -> blocked
              -> paused
              -> aborted
```

不要先做复杂 phase machine。是否需要 phase，等真实任务证明 node/gate 不够用以后再引入。

## MapNode

Node 是行动点，不是角色，也不是 agent。

```rust
pub struct MapNode {
    pub id: NodeId,
    pub title: String,
    pub purpose: String,
    pub status: NodeStatus,
    pub dependencies: Vec<NodeId>,
    pub context_boundary: ContextBoundary,
    pub required_artifacts: Vec<ArtifactKind>,
    pub gate: GateSpec,
    pub version: NodeVersion,
}
```

`NodeStatus` 第一版只需要：

```text
pending -> ready -> running -> completed
                       |
                       -> blocked
ready -> skipped
```

节点完成必须满足：

- required artifacts 已提交。
- gate 通过。
- 没有 stale context。
- 没有 unresolved blocker。

## NodeExecution

`NodeExecution` 只描述一个节点本次怎么执行。

```rust
pub struct NodeExecution {
    pub id: ExecutionId,
    pub node_id: NodeId,
    pub strategy: ExecutionStrategy,
    pub assignments: Vec<AgentAssignment>,
    pub expected_artifacts: Vec<ArtifactKind>,
}

pub enum ExecutionStrategy {
    Single,
    Parallel,
    Review,
    Verify,
}
```

第一版只保留四种策略：

| Strategy | 用途 |
| --- | --- |
| `Single` | 一个 agent 执行节点 |
| `Parallel` | 多个 agent 分片执行同一节点 |
| `Review` | 对已有 artifact 做审查 |
| `Verify` | 对已有 artifact 做验证 |

第一版不做候选竞赛、投票和共识聚合。这些不是最小验证闭环的一部分。

策略选择由 runtime 根据节点决定：

```text
small/simple node -> Single
large/read-only scan -> Parallel
artifact needs critique -> Review
artifact needs proof -> Verify
```

## AgentAssignment

Agent 是执行资源，不是固定角色。

```rust
pub struct AgentAssignment {
    pub id: AssignmentId,
    pub node_id: NodeId,
    pub objective: String,
    pub context_pack: ContextPack,
    pub allowed_tools: Vec<ToolName>,
    pub expected_artifact: ArtifactKind,
    pub constraints: Vec<String>,
}
```

agent 可以在 map 上移动，但每次移动都必须生成新的 assignment 和 context pack。

## ContextPack

上下文必须由 runtime 分配，不能让 agent 无限自由继承所有材料。

```rust
pub struct ContextPack {
    pub id: ContextPackId,
    pub graph_version: GraphVersion,
    pub node_version: NodeVersion,
    pub required_sources: Vec<ContextSource>,
    pub artifacts: Vec<ArtifactRef>,
    pub constraints: Vec<String>,
}
```

第一版只做版本检查：

```text
fresh if:
  assignment.graph_version == current.graph_version
  assignment.node_version == current.node_version
  required artifact versions unchanged

stale if:
  upstream artifact changed
  node status changed
  relevant file changed after context pack was issued
```

如果 stale，artifact 不能直接通过 gate，必须刷新或重跑。

## Artifact

正式结论必须是 artifact，不能只是 mailbox 文本。

```rust
pub struct ArtifactEnvelope<T> {
    pub id: ArtifactId,
    pub node_id: NodeId,
    pub assignment_id: AssignmentId,
    pub producer: AgentId,
    pub kind: ArtifactKind,
    pub base_graph_version: GraphVersion,
    pub base_node_version: NodeVersion,
    pub evidence_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub body: T,
}
```

第一版 artifact 类型只保留：

| Artifact | 用途 |
| --- | --- |
| `Finding` | 文件、符号、日志、事实证据 |
| `Analysis` | 对事实的解释或方案分析 |
| `PatchProposal` | 候选改动说明或 patch 引用 |
| `ReviewResult` | 对 artifact 的审查意见 |
| `VerificationResult` | 测试、构建、复现、静态检查结果 |
| `Blocker` | 阻塞原因和需要的输入 |

不做额外评分、投票或共识类产物。

## Gate

Gate 是唯一准出机制。

Gate 不评估“质量分”，只检查明确条件是否满足。它可以阻断明显缺证据、上下文过期、验证缺失或存在 blocker 的节点，但不能声称某个复杂结果已经被客观量化为“高质量”。

```rust
pub struct GateSpec {
    pub required_artifacts: Vec<ArtifactKind>,
    pub checks: Vec<GateCheck>,
}

pub enum GateResult {
    Pass,
    Fail,
    Blocked,
    Stale,
}
```

第一版 gate 只检查：

- artifact 类型是否齐全。
- artifact schema 是否有效。
- base version 是否仍然 fresh。
- required verification 是否通过。
- 是否存在 blocker。
- artifact 是否显式记录关键限制和未验证部分。

```mermaid
flowchart TD
    A["Artifact submitted"] --> B{"Schema valid?"}
    B -->|no| F["Gate Fail"]
    B -->|yes| C{"Context fresh?"}
    C -->|no| S["Gate Stale"]
    C -->|yes| D{"Required artifacts present?"}
    D -->|no| F
    D -->|yes| E{"Blocker exists?"}
    E -->|yes| BL["Gate Blocked"]
    E -->|no| P["Gate Pass"]
```

## MapEvent

状态变化必须是 append-only event。

```rust
pub enum MapEvent {
    ModeChanged,
    MapCreated,
    NodeAdded,
    NodeStarted,
    AssignmentIssued,
    ArtifactSubmitted,
    GateEvaluated,
    NodeCompleted,
    NodeBlocked,
    MapCompleted,
}
```

第一版不要做完整 event sourcing 框架，但事件必须足够 replay 当前任务的关键决策。

## 通信规则

图是主要沟通介质。

```text
Map is the source of truth.
Messages are hints.
Artifacts are durable claims.
Events are state transitions.
```

允许直接消息，但直接消息不能：

- 标记节点完成。
- 证明风险消失。
- 选择 patch。
- 推进 gate。
- 成为最终事实来源。

如果消息内容影响任务结论，必须转成 artifact。

## 与当前 Codex 基建的关系

现有 Codex subagent 机制继续作为执行底座：

| Codex substrate | experiment 模式中的用途 |
| --- | --- |
| `AgentControl` | 创建和管理 subagent thread |
| `AgentPath` | agent 的稳定路径标识 |
| `AgentRegistry` | live agent/thread 状态 |
| mailbox | 临时通知和唤醒 |
| session events | 承载 MapEvent |
| tools/sandbox/approval | 执行工具和权限边界 |

`standard` 模式不变。

`experiment` 模式只是包一层：

```text
Ready MapNode
  -> NodeExecution
  -> AgentAssignment + ContextPack
  -> existing spawn/send/wait runtime
  -> Artifact ingestion
  -> Gate
```

## MVP 实施顺序

### MA-0：模式开关

- 实现 `/multi-agents standard`。
- 实现 `/multi-agents standart` alias。
- 实现 `/multi-agents experiment`。
- session state 记录当前模式。
- 模式切换写 event。
- `standard` 行为保持现状。

验收：关闭 experiment 后，当前 spawn/send/wait/close 行为不变。

### MA-1：Map 与 Node

- 定义 `ActionMapInstance`。
- 定义 `MapNode`。
- 根据用户任务和模板生成最小 map。
- 支持查看当前 map。

验收：一个架构优化任务能生成 3-8 个可解释节点。

### MA-2：Assignment 与 ContextPack

- Ready node 可生成 assignment。
- assignment 携带 context pack。
- artifact 记录 base version。
- stale artifact 被拒绝或要求重跑。

验收：上游节点变化后，下游旧 artifact 不能直接通过 gate。

### MA-3：Artifact 与 Gate

- agent 提交结构化 artifact。
- gate 检查 artifact、version、blocker。
- node completion 只能由 gate 触发。

验收：自然语言“我完成了”不能直接完成节点。

### MA-4：Review / Verify 策略

- NodeExecution 支持 `Review`。
- NodeExecution 支持 `Verify`。
- verification 结果可以阻断 gate。

验收：实现类节点在缺少验证时不能完成。

## 参考来源

外部参考只作为背景，不直接生成 runtime 概念。

| 来源 | 本设计中的用法 |
| --- | --- |
| DeepSeek API Docs: https://api-docs.deepseek.com/ | 确认模型、上下文、tool calls、cache、rate limit 能力边界 |
| Anthropic multi-agent research system: https://www.anthropic.com/engineering/built-multi-agent-research-system | 参考委派式多 agent 的工程挑战 |
| Microsoft AutoGen Selector Group Chat: https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/selector-group-chat.html | 参考动态参与者选择，但不采用自由群聊 |
| OpenAI Agents SDK Handoffs: https://openai.github.io/openai-agents-js/guides/handoffs/ | 将 handoff 收敛为 assignment |
| OpenAI Agents SDK Guardrails: https://openai.github.io/openai-agents-js/guides/guardrails/ | 将 guardrail 收敛为 gate |
| Martin Fowler Optimistic Offline Lock: https://martinfowler.com/eaaCatalog/optimisticOfflineLock.html | 参考 stale write 检测 |
| Martin Fowler Event Sourcing: https://www.martinfowler.com/eaaDev/EventSourcing.html | 参考事件化状态变更和 replay |
