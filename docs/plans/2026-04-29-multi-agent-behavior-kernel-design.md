# Multi-Agent 行为内核具体设计

日期：2026-04-29

## 结论

WhaleCode 下一阶段应先落 `Multi-Agent Behavior Kernel`，再继续做 Create、Debug、Primitive 和 Viewer UI 的上层设计。

这个内核不是一个“多聊天窗口”能力，而是三层可执行语义：

```text
Layer A: Multi-Agent Core Runtime
  Supervisor / Cohort / WorkUnit / Message Bus / Artifact Store / Gate / Budget

Layer B: DeepSeek-Aware Scheduling
  Flash-Pro 路由 / thinking 策略 / cache-aware fan-out / context slice / 并发治理

Layer C: Viewer Control Plane
  只读对抗审查 / Concern 事件 / phase 阻断 / resolution / 成本门禁
```

Search 已经是基础工具能力；在本设计里它只作为 Scout/Searcher 的可用工具，不主导行为内核。

## 目标与非目标

目标：

- 为所有多 agent 工作流提供统一生命周期。
- 让 agent 之间只通过结构化 artifact 协作，避免自由群聊。
- 让 Flash 大量并行探索，Pro 只在关键裁决点使用。
- 让 Viewer 成为控制面角色，而不是第二个 Reviewer。
- 让每个关键动作都有 session event，可 replay、可审查、可度量。
- 让 Create/Debug 后续只是在同一个 kernel 上声明不同 workflow plan。

非目标：

- 不在此文档设计完整 Create/Debug DAG。
- 不设计 Web Viewer 前端布局。
- 不重新设计 search provider。
- 不把多 agent 行为写成 prompt 模板。
- 不允许多个 agent 直接写共享工作区。

## 运行时对象

### `Supervisor`

唯一有权改变全局 workflow 状态的确定性调度器。

职责：

- 接收用户 goal。
- 构造 `SharedTaskPack`。
- 生成 `WorkUnit`。
- 启动、暂停、终止、重试 agent。
- 管理 cohort 宽度和预算。
- 接收 artifacts 并运行 gates。
- 执行 tournament、consensus、merge decision。
- 选择唯一写入共享工作区的 patch。
- 写 session event。

Supervisor 不是 LLM agent。它可以调用 Judge/Viewer 等 LLM 角色，但 phase transition 由确定性 gate 决定。

### `AgentInstance`

一次 agent 执行实例。

```rust
pub struct AgentInstance {
    pub id: AgentId,
    pub role: AgentRole,
    pub cohort_id: CohortId,
    pub work_unit_id: WorkUnitId,
    pub model_route: ModelRoute,
    pub tool_policy: ToolPolicyRef,
    pub context_slice: ContextSliceRef,
    pub state: AgentState,
}
```

`AgentState`：

```text
Created -> Ready -> Running -> WaitingTool -> SubmittedArtifact -> Completed
                         |              |              |
                         |              |              -> Rejected
                         |              -> TimedOut
                         -> Cancelled
```

Agent 不能自行 spawn 其他 agent，不能自行改变 phase，不能直接读取其他 agent 的 hidden reasoning。

### `CohortRun`

同一批次、同一协作目的的 agent 集合。

```rust
pub struct CohortRun {
    pub id: CohortId,
    pub kind: CohortKind,
    pub workflow: WorkflowKind,
    pub phase: PhaseId,
    pub width_plan: WidthPlan,
    pub diversity_policy: DiversityPolicy,
    pub budget: CohortBudget,
    pub stop_rules: Vec<StopRule>,
}
```

`CohortKind`：

- `Scout`
- `Analyst`
- `Implementer`
- `Reviewer`
- `Judge`
- `Verifier`
- `Viewer`

Viewer 使用相同事件和 artifact 协议，但不参与普通 cohort 竞争，不拥有实现任务。

### `WorkUnit`

Supervisor 分配给 agent 的最小可执行单元。

```rust
pub struct WorkUnit {
    pub id: WorkUnitId,
    pub title: String,
    pub objective: String,
    pub expected_artifact: ArtifactKind,
    pub constraints: Vec<ConstraintRef>,
    pub allowed_tools: Vec<ToolName>,
    pub ownership: OwnershipScope,
    pub context_slice: ContextSliceSpec,
    pub timeout: Duration,
    pub success_criteria: Vec<AcceptanceCriterion>,
    pub blocked_by: Vec<ArtifactRef>,
}
```

设计约束：

- 一个 WorkUnit 必须只要求一个主要产物。
- 如果任务需要多个产物，Supervisor 必须拆分。
- WorkUnit 必须声明 ownership，否则不能开放写工具。
- WorkUnit 必须声明 expected artifact，否则不能进入 cohort。

### `SharedTaskPack`

所有同一 workflow 内 agent 共享的稳定任务包。

内容：

- 用户 goal。
- 当前 repo 摘要。
- 相关约束。
- 当前 phase。
- 可用工具摘要。
- artifact schema 摘要。
- 已确认 facts。
- 风险边界。

DeepSeek fan-out 请求应尽量复用同一个 stable prefix：

```text
Stable System Prefix
  -> Whale role contract
  -> Tool and artifact schema
  -> SharedTaskPack
  -> WorkUnit-specific delta
```

这样可以提高 cache hit 概率，但正确性不能依赖 cache 命中。

### `Artifact`

Agent 之间协作的唯一事实载体。

```rust
pub struct ArtifactEnvelope<T> {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub schema_version: SchemaVersion,
    pub producer: AgentId,
    pub work_unit_id: WorkUnitId,
    pub trace_id: TraceId,
    pub confidence: Confidence,
    pub evidence_refs: Vec<ArtifactRef>,
    pub redaction_summary: RedactionSummary,
    pub cost: CostSummary,
    pub body: T,
}
```

第一阶段必须支持的 artifact：

| Artifact | 生产者 | 用途 |
| --- | --- | --- |
| `Finding` | Scout/Searcher | 文件、符号、外部来源、事实片段 |
| `AnalysisCandidate` | Analyst | 需求分解、风险、方案 lens |
| `Hypothesis` | Analyst/Debugger | Debug 假设 |
| `EvidenceRecord` | Scout/Verifier | 支持或证伪假设的证据 |
| `PlanCandidate` | Architect/Analyst | 设计候选 |
| `PatchArtifact` | Implementer | 隔离 patch 候选 |
| `ReviewFinding` | Reviewer | 审查问题 |
| `VerificationResult` | Verifier | 测试、构建、复现结果 |
| `JudgeDecision` | Judge | 候选排序、合成、拒绝理由 |
| `ViewerConcern` | Viewer | 对抗性风险 |
| `ConsensusReport` | Supervisor/Judge | 聚合结论 |

Artifact 必须可序列化到 JSONL session。后续 replay 只能依赖 event 和 artifact，不依赖 agent 私有上下文。

## Message Bus

Message Bus 不是聊天频道，而是 artifact/event 路由层。

### Envelope

```rust
pub struct BusEnvelope {
    pub id: MessageId,
    pub trace_id: TraceId,
    pub workflow_id: WorkflowId,
    pub phase: PhaseId,
    pub from: BusSender,
    pub to: BusTarget,
    pub topic: BusTopic,
    pub causality: Vec<MessageId>,
    pub payload: BusPayload,
}
```

### Topic

第一阶段 topic：

- `workunit.assigned`
- `artifact.submitted`
- `artifact.accepted`
- `artifact.rejected`
- `cohort.started`
- `cohort.completed`
- `gate.started`
- `gate.passed`
- `gate.failed`
- `judge.requested`
- `judge.decision`
- `viewer.requested`
- `viewer.concern`
- `viewer.resolved`
- `budget.updated`
- `phase.transition_requested`
- `phase.transition_blocked`
- `phase.transition_completed`

### 路由模式

| 模式 | 用途 | 约束 |
| --- | --- | --- |
| Unicast | Supervisor 给单 agent 分配 WorkUnit | 必须带 WorkUnitId |
| Cohort Broadcast | 同一 cohort 接收 shared facts | 不能携带其他候选的自然语言立场 |
| Artifact Subscribe | Reviewer/Viewer 订阅 artifact 类型 | 只能看到 artifact，不看 hidden reasoning |
| Request-Reply | Judge/Verifier 明确请求和响应 | 必须有 timeout |
| Control Event | budget、gate、phase 事件 | 只能由 Supervisor 写 |

禁止默认开放 agent-to-agent 自由聊天。需要 debate 时，也必须通过 artifact id 和新增 evidence 表达。

## Core Runtime 生命周期

### 1. Intake

输入：

- 用户 goal。
- 当前 repo 状态。
- 用户显式模式，例如 `/create`、`/debug`、`/review`。

输出：

- `WorkflowIntent`
- `RiskLevel`
- 初始 `SharedTaskPack`

规则：

- 自动分类低置信度时进入 clarify，而不是猜测工作流。
- 如果任务包含“先诊断再改”，默认 Debug 子流程先行。

### 2. Plan Cohorts

Supervisor 根据 intent 生成 cohort plan：

```rust
pub struct CohortPlan {
    pub phase: PhaseId,
    pub cohorts: Vec<CohortSpec>,
    pub gates: Vec<GateSpec>,
    pub required_artifacts: Vec<ArtifactKind>,
}
```

第一阶段默认 plan：

| 任务风险 | Scout | Analyst | Implementer | Reviewer | Judge | Viewer |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| low | 1-3 | 0-1 | 1 | 1 | 0 | final-only |
| medium | 3-8 | 2-4 | 1-3 | 2 | 1 | gate/artifact |
| high | 8-24 | 4-8 | 2-4 competitive | 2-4 | 1-2 | strict gates |

### 3. Spawn

Spawn 必须满足：

- Cohort budget 未超限。
- Tool permissions 已由 phase + role + ownership 计算。
- Context slice 已冻结。
- WorkUnit schema 校验通过。
- DiversityPolicy 已写入 session event。

### 4. Execute

Agent loop 的输出只能是：

- tool call。
- structured artifact draft。
- explicit blocked report。
- final artifact。

自然语言解释可以作为 artifact 的 `rationale` 字段存在，但不能作为跨 agent 协作的唯一载体。

### 5. Collect And Reduce

Supervisor 对 artifact 做三步处理：

1. Schema validation。
2. Evidence/reference validation。
3. Dedup/correlation scoring。

重复 finding 合并，不重复计算 agent 数量。`effective_agent_count` 由非冗余 artifact 决定。

### 6. Gate

Gate 是确定性检查，必要时调用 Judge/Viewer 辅助判断，但最终结果写成 gate event。

示例 gate：

| Gate | 输入 | 通过条件 |
| --- | --- | --- |
| `EvidenceGate` | `Hypothesis` + `EvidenceRecord[]` | 每个根因结论至少引用一条强证据 |
| `PatchApplyGate` | `PatchArtifact` | patch 可 dry-run apply，ownership 不冲突 |
| `ReviewGate` | `ReviewFinding[]` | 无 unresolved critical finding |
| `VerificationGate` | `VerificationResult[]` | smoke/regression 满足任务风险等级 |
| `ViewerGate` | `ViewerConcern[]` | 无 unresolved critical concern |

### 7. Decide

当存在多个候选时，Supervisor 触发 Judge：

```rust
pub struct JudgeRequest {
    pub candidates: Vec<ArtifactRef>,
    pub scorecards: Vec<ReviewFindingRef>,
    pub verifications: Vec<VerificationResultRef>,
    pub concerns: Vec<ViewerConcernRef>,
    pub decision_kind: JudgeDecisionKind,
}
```

Judge 输出必须包含：

- accepted candidate。
- rejected candidates。
- rejected rationale。
- evidence used。
- residual risks。
- whether synthesis is required。

禁止只按票数选择。Verifier 和强证据优先于多数一致。

### 8. Apply

只有 Supervisor 可以把最终 `PatchArtifact` 应用到共享工作区。

规则：

- 同一时刻共享工作区实际写入宽度始终为 1。
- 并行 Implementer 只能写 patch buffer 或隔离 workspace。
- 合成 patch 前必须重跑 ownership/conflict gate。
- 冲突不能交给 LLM 猜测合并，必须生成 `ConflictReport`。

### 9. Verify

Verifier matrix 按风险等级选择：

| 风险 | 必须验证 |
| --- | --- |
| low | targeted test 或可解释 smoke |
| medium | targeted test + affected area smoke |
| high | repro/smoke/regression + review closure |

缺少测试能力时，产出 `TestGapArtifact`，并阻止“完成”状态，除非用户显式接受 waiver。

### 10. Finalize

最终回答只能引用 accepted artifacts：

- 已应用 patch。
- 已通过验证。
- 未解决风险。
- Viewer unresolved warning。
- 用户需要知道的配置或测试结果。

## DeepSeek-Aware Scheduling

### ModelRoute

```rust
pub struct ModelRoute {
    pub provider: ProviderId,
    pub model: ModelId,
    pub thinking: ThinkingMode,
    pub max_output_tokens: TokenBudget,
    pub tool_mode: ToolMode,
    pub reason: RoutingReason,
}
```

模型能力必须来自 `ModelCapabilityProbe`，不能把 V4 的 context/output/pricing/thinking 行为写死。

### 默认路由

| Role / Task | 默认模型 | Thinking | 说明 |
| --- | --- | --- | --- |
| User-facing lead answer | `deepseek-v4-pro` | enabled | 保证交互质量 |
| Scout repo search | `deepseek-v4-flash` | disabled/auto | 高并发低成本 |
| Scout web/doc search | `deepseek-v4-flash` | disabled/auto | 只产 Finding |
| Analyst first pass | `deepseek-v4-flash` | auto | 多 lens 覆盖 |
| Architect design | `deepseek-v4-pro` | enabled | 高风险设计 |
| Implementer simple patch | `deepseek-v4-flash` | auto | 成本优先 |
| Implementer high-risk patch | `deepseek-v4-pro` 或 Flash+Pro review | enabled | 由风险路由 |
| Reviewer normal | `deepseek-v4-flash` | auto | 快速审查 |
| Reviewer critical/security | `deepseek-v4-pro` | enabled | 强审查 |
| Judge | `deepseek-v4-pro` | enabled | 候选裁决 |
| Viewer | `deepseek-v4-pro` | enabled | 对抗审查 |
| Context compaction | `deepseek-v4-pro` | enabled | 保真优先 |

### Thinking 内容处理

DeepSeek thinking + tool call 的关键约束：

- 同一用户 turn 内，如果模型在 thinking 中发起 tool call，后续 sub-turn 必须把当轮 `reasoning_content` 回传给 API。
- 新用户 turn 开始时，历史 `reasoning_content` 不进入上下文。
- 跨 agent 协作不能传 hidden reasoning，只传 artifact。
- Viewer 不能读取其他 agent 的 hidden reasoning，只能读取 artifact、diff、logs、gate result。

这要求 `AgentContextStore` 区分：

```text
private_subturn_reasoning: 仅当前 agent 当前 turn 可见
artifact_rationale: 可审查、可 redaction、可 replay
shared_task_pack: 所有相关 agent 可见
```

### Cache-Aware Fan-Out

DeepSeek context cache 是 best-effort 优化，不是可靠性机制。

调度策略：

1. 同 cohort 请求共享稳定 system prefix。
2. `SharedTaskPack` 放在 WorkUnit delta 前。
3. WorkUnit delta 尽量短，不在前缀前插入 agent 独有文本。
4. 大 artifact 用引用注入，避免把所有 Scout 输出复制给每个 agent。
5. 批量启动 cohort 时，先发送相同前缀请求，降低 cache miss。
6. cache hit/miss usage 写入 cost event，用于后续调度优化。

### Context Allocation

默认上下文切片：

| Agent | 输入内容 |
| --- | --- |
| Scout | SharedTaskPack + narrow question + allowed search scope |
| Analyst | SharedTaskPack + selected Finding refs + lens |
| Implementer | SharedTaskPack + accepted plan + owned files + constraints + tests |
| Reviewer | SharedTaskPack + PatchArtifact + relevant specs + tests |
| Verifier | Verification plan + commands + expected signals |
| Judge | Candidate refs + scorecards + verification + concerns |
| Viewer | Artifact/gate/diff/log summaries + risk policy |

禁止把所有原始搜索输出直接广播给所有 agent。

### ConcurrencyGovernor

DeepSeek 官方当前说明没有固定用户级 rate limit，但高负载时可能长期保持连接、返回 keep-alive，未开始推理超过 10 分钟会关闭连接。Whale 仍要把 429、超时、keep-alive、连接关闭都视为 provider pressure signal。

输入信号：

- in-flight requests。
- rolling p50/p95 first-token latency。
- streaming keep-alive duration。
- provider 429 / 5xx。
- 10-minute no-inference closure。
- token budget burn rate。
- cache hit rate。
- local CPU/process/tool pressure。

输出：

- cohort width increase/decrease。
- retry backoff。
- switch Flash/Pro ratio。
- stop low-value scouts。
- require Judge early convergence。

宽度调整规则：

```text
if provider_pressure_high:
  reduce scout/analyst width first
  keep verifier and final judge budget

if unique_evidence_ratio stops increasing:
  stop more scouts even if budget remains

if verification fails repeatedly:
  stop more implementation candidates
  route to Debug-style evidence collection
```

### Cost Modes

| Mode | 适用 | 行为 |
| --- | --- | --- |
| `economy` | 小改动 | 单候选实现，少量 Scout，Viewer final-only |
| `balanced` | 默认 | 中等 fan-out，关键点 Pro Judge/Viewer |
| `swarm` | 高风险/大任务 | 多 lens、多候选、strict Viewer、完整 verifier matrix |

Cost mode 影响 cohort width，不影响 gate 下限。

## Viewer Control Plane

### 定位

Viewer 是只读对抗控制面。它监听关键 artifacts 和 gates，提出 `ViewerConcern`，但不实现、不合并、不验证自己的建议。

Viewer 与 Reviewer/Judge 的区别：

| 角色 | 问题 | 输出 | 是否阻断 |
| --- | --- | --- | --- |
| Reviewer | 产物是否合格 | `ReviewFinding` | 通过 ReviewGate |
| Judge | 哪个候选更好 | `JudgeDecision` | 通过 DecisionGate |
| Viewer | 系统是否遗漏了隐藏风险 | `ViewerConcern` | critical 通过 ViewerGate 阻断 |

### Trigger Policy

默认触发：

- phase gate 前。
- Judge decision 后。
- patch apply 前。
- verification failed 或 verification gap。
- permission escalation。
- final answer 前。

Strict 模式额外触发：

- 每个 high-risk `PatchArtifact`。
- 每个 root-cause decision。
- 每个 waiver。
- Reviewer 无 finding 但 diff 风险高。

不触发：

- 普通 token delta。
- 低风险 Scout Finding。
- 已被 dedup 丢弃的重复 artifact。

### ViewerPromptPack

Viewer 只接收：

- artifact 摘要和引用。
- diff 摘要。
- gate result。
- verification result。
- redacted logs。
- risk policy。
- unresolved concerns。

Viewer 不接收：

- 其他 agent hidden reasoning。
- 未脱敏 secrets。
- 全量无关仓库内容。
- 用户未授权外部数据。

### Concern Schema

```rust
pub struct ViewerConcern {
    pub id: ConcernId,
    pub target: ConcernTarget,
    pub severity: ConcernSeverity,
    pub category: ConcernCategory,
    pub claim: String,
    pub evidence_refs: Vec<ArtifactRef>,
    pub missing_evidence: Vec<String>,
    pub suggested_gate: Option<GateSpec>,
    pub confidence: f32,
    pub status: ConcernStatus,
}
```

Severity：

- `critical`：阻止 phase transition / patch apply / release。
- `warning`：必须进入 Reviewer/Judge 上下文并被回应。
- `question`：高风险阶段必须回答，低风险可记录。
- `suggestion`：只作为改进建议。

### Concern 处理

```text
ViewerConcern raised
  -> Supervisor classifies route
  -> if critical: block gate
  -> assign resolution WorkUnit
  -> Reviewer/Judge/Verifier responds with artifact
  -> Viewer or deterministic gate checks resolution
  -> concern_resolved / concern_waived
```

Waiver 规则：

- 只能由用户或明确配置允许。
- 必须写入 session event。
- 最终报告必须展示 waiver 和残余风险。

### Viewer 成本门禁

Viewer 使用 Pro，必须受预算控制。

默认策略：

- low risk：final-only。
- medium risk：phase gate + patch apply。
- high risk：strict gates。
- provider pressure high：保留 patch apply / final answer 前触发，暂停普通 warning trigger。

Viewer 不能因为成本原因被完全关闭在高风险任务之外，除非用户显式配置。

## Session Events

新增或确认以下事件：

```text
shared_task_pack_created
cohort_plan_created
diversity_policy_created
cohort_started
workunit_assigned
agent_started
agent_tool_call_started
agent_tool_call_completed
artifact_submitted
artifact_accepted
artifact_rejected
cohort_completed
correlation_score_recorded
consensus_started
consensus_completed
judge_requested
judge_decision_recorded
viewer_requested
viewer_concern_raised
viewer_concern_resolved
gate_started
gate_passed
gate_failed
budget_updated
provider_pressure_recorded
patch_apply_started
patch_apply_completed
verification_matrix_started
verification_matrix_completed
phase_transition_blocked
phase_transition_completed
```

每个 event 必须包含：

- `schema_version`
- `trace_id`
- `workflow_id`
- `phase`
- `timestamp`
- `redaction_summary`
- `cost_summary`（如适用）

## 最小落地顺序

### Phase MA-1：协议和事件

交付：

- `WorkUnit`、`CohortRun`、`ArtifactEnvelope`、`BusEnvelope` schema。
- session event 枚举。
- replay reducer skeleton。
- fixture JSONL。

验收：

- fixture 能 replay 出 cohort、artifact、gate、viewer concern 状态。
- primitive 关闭后历史 event 仍可 replay。

### Phase MA-2：单进程 Supervisor + Scout/Analyst

交付：

- Supervisor 创建 read-only cohort。
- Scout 输出 `Finding`。
- Analyst 输出 `AnalysisCandidate`。
- dedup/correlation score。

验收：

- 同一任务可并行启动多个只读 Scout。
- 重复 finding 不增加 effective agent count。
- Agent 不能互看首轮输出。

### Phase MA-3：DeepSeek 调度

交付：

- `ModelRoute`。
- capability probe profile。
- thinking sub-turn handling。
- cache/cost usage event。
- ConcurrencyGovernor。

验收：

- Flash/Pro 路由可由角色和风险决定。
- thinking + tool call 回传规则有 mock 测试。
- provider pressure 能降低 Scout/Analyst 宽度。

### Phase MA-4：Patch League 与 Verifier Matrix

交付：

- isolated `PatchArtifact`。
- dry-run apply gate。
- reviewer scorecard。
- verifier matrix。
- single-writer apply。

验收：

- 多 Implementer 不能同时写共享工作区。
- 同文件候选必须走 tournament。
- 最终 patch 必须通过 verification gate。

### Phase MA-5：Viewer Control Plane

交付：

- Viewer trigger policy。
- `ViewerPromptPack`。
- `ViewerConcern` schema。
- critical concern gate。
- resolution / waiver event。

验收：

- critical concern 阻止 phase transition。
- warning concern 进入 Reviewer/Judge 上下文。
- Viewer 无写工具。
- replay 后 concern 状态一致。

## 设计引用

- Anthropic Engineering, [How we built our multi-agent research system](https://www.anthropic.com/engineering/built-multi-agent-research-system), 2025-06-13. 用于确认 orchestrator-worker、多 subagent 并行搜索、专门 citation/汇总 agent 的工程经验。
- Microsoft AutoGen documentation, [Group Chat](https://microsoft.github.io/autogen/dev/user-guide/core-user-guide/design-patterns/group-chat.html) and [AgentChat](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/index.html). 用于确认共享线程 group chat 是一种模式，但顺序 shared-context group chat 不适合作为 Whale 默认协作语义。
- OpenAI Agents SDK documentation, [Agents](https://openai.github.io/openai-agents-js/guides/agents/), [Handoffs](https://openai.github.io/openai-agents-python/handoffs/), and [Guardrails](https://openai.github.io/openai-agents-js/guides/guardrails/). 用于确认 handoff、manager、guardrail 的边界，并提醒 guardrail 作用域不能被误当成所有 agent/tool 的全局硬约束。
- DeepSeek API Docs, [Your First API Call](https://api-docs.deepseek.com/), [Models & Pricing](https://api-docs.deepseek.com/quick_start/pricing), [Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode), [Tool Calls](https://api-docs.deepseek.com/guides/tool_calls), [Context Caching](https://api-docs.deepseek.com/guides/kv_cache/), [Rate Limit](https://api-docs.deepseek.com/quick_start/rate_limit/). 用于确认 V4 Flash/Pro、thinking/tool calls、cache best-effort、长连接压力信号和价格/能力必须 runtime probe。

## 执行结论

下一步实现或继续细化时，不应直接从 Create/Debug UI 或某个 Primitive 开始，而应先把本设计中的 `WorkUnit -> Cohort -> Artifact -> Gate -> Decision -> Verify -> Finalize` 路径跑通。

一旦行为内核稳定：

- Create 只是 plan tournament + scaffolding gate + patch league。
- Debug 只是 hypothesis cohort + evidence race + root-cause judge。
- Viewer 只是同一 artifact/gate 流上的 control plane。
- Web Viewer 只是读取同一 event stream 的只读投影。
