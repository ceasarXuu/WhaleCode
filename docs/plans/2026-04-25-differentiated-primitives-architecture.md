# WhaleCode 差异化原语架构设计

---

## 一、结论

WhaleCode 相比 Codex CLI、Claude Code、OpenCode、Pi 的差异不应只停留在愿景或提示词层。除了已经深化的多 Agent 群体协同，以下五项必须被实现为运行时原语：

1. 证据链 Debug。
2. 脚手架先行 Create。
3. 参考驱动设计。
4. 独立 Viewer。
5. 技能自进化。

这些原语共同遵循一个硬规则：

> Prompt 可以解释行为，但不能保证行为。保证行为必须靠 artifact schema、phase gate、permission policy、event log 和 replay。

同时，这些原语必须是 **可插拔 Primitive Module**，不能直接耦合进底层 agent loop、工具执行器或某种具体实现语言。WhaleCode 会长期实验很多区别于通用 coding agent 的特化能力，其中一部分可能被证明有效，一部分可能需要替换或淘汰。核心 runtime 只能提供稳定协议和注册机制，不能把某个特化策略写死成不可移除的底层假设。

---

## 二、统一架构模型

五个差异化原语不是五个孤立功能，而是通过 `PrimitiveRegistry` 注册到核心 runtime 的控制层：

```text
User Goal
  -> Supervisor
  -> PrimitiveRegistry
  -> Enabled Primitive Modules
  -> WorkUnit / Agent / ToolRuntime
  -> Artifact
  -> Gate
  -> Session Event
  -> Viewer / Verifier / Replay
```

每个原语都必须具备：

| 能力 | 要求 |
|------|------|
| Artifact-first | 产物是结构化 artifact，不是自然语言段落 |
| Gate-enforced | phase transition 由确定性 gate 判断，不由 agent 自报 |
| Event-sourced | 每个关键动作写入 JSONL session event |
| Replayable | session replay 能还原当前原语状态 |
| Permission-aware | 工具权限受 phase、role、workunit、ownership 约束 |
| Viewer-visible | Viewer 读取 artifact 和 gate result，而不是读取内部思考 |
| Reference-audited | 成熟底座走 Codex-first；自研差异点记录外部依据和本地证据 |
| Pluggable | 原语通过 manifest、schema、gate、hook、reducer 注册，默认内置但可关闭/替换 |
| Eval-gated | 原语是否继续默认启用必须由 fixture、真实 session metrics 和用户反馈验证 |

Phase 1 不需要完整多 agent，但必须先把这些 artifact 和 event schema 放进 `whalecode-protocol`，否则后续多 agent 会缺少稳定协作语言。

### 2.1 Primitive Module Contract

```rust
pub trait PrimitiveModule {
    fn manifest(&self) -> PrimitiveManifest;
    fn artifact_schemas(&self) -> Vec<ArtifactSchemaRef>;
    fn event_schemas(&self) -> Vec<EventSchemaRef>;
    fn gates(&self) -> Vec<GateSpec>;
    fn phase_hooks(&self) -> Vec<PhaseHookSpec>;
    fn permission_overlays(&self) -> Vec<PermissionOverlaySpec>;
    fn replay_reducers(&self) -> Vec<ReplayReducerSpec>;
    fn viewer_triggers(&self) -> Vec<ViewerTriggerSpec>;
    fn eval_specs(&self) -> Vec<PrimitiveEvalSpec>;
}

pub struct PrimitiveManifest {
    pub id: PrimitiveId,
    pub name: String,
    pub version: SemVer,
    pub stability: PrimitiveStability,
    pub default_enabled: bool,
    pub dependencies: Vec<PrimitiveId>,
    pub conflicts: Vec<PrimitiveId>,
    pub rollback_policy: RollbackPolicy,
}
```

核心 runtime 只依赖 `PrimitiveModule` contract：

- `Supervisor` 通过 `PrimitiveRegistry` 查询当前 workflow 启用哪些 gate 和 hook。
- `whalecode-session` 只存储 event，不理解每个原语的业务含义。
- `whalecode-protocol` 保存稳定 schema 和版本迁移。
- `apps/viewer` 根据 event schema 和 artifact schema 渲染，不绑定具体模块实现。
- `whalecode-workflow` 编排 phase，但不把某个特化策略写死进 phase machine。

### 2.2 可插拔原则

| 原则 | 要求 |
|------|------|
| Protocol before behavior | 先定义 artifact/event/gate schema，再实现模块行为 |
| Default-on is earned | 默认启用必须有 eval 证据；实验能力默认 opt-in |
| Replaceable by design | 模块可被新版、第三方实现或更简单策略替换 |
| Composable not tangled | 模块之间通过 artifact refs 通信，不直接调用内部状态 |
| Kill switch required | 每个非基础模块必须有 config flag 和 runtime disable path |
| No hidden core coupling | Agent loop、ToolRuntime、SessionStore 不能包含原语专属分支 |
| Migration explicit | schema 变更必须有版本和 replay migration |
| Measured usefulness | 每个模块定义自己的成功指标、失败指标和淘汰条件 |

这条原则的直接后果是：证据链、脚手架、参考驱动、Viewer、技能自进化都是 WhaleCode 的默认内置模块，但不是不可替换的底层设计语言。它们可以组合成 Create/Debug workflow，也可以在特定任务、成本模式或用户配置下关闭。

### 2.3 Generic Agent Substrate First

2026-04-27 更新：这里的 substrate 不再表示从 0 完成 Whale 自研 runtime。
它现在表示 Codex CLI whole-repo upstream substrate + Whale bridge。实现这些
特化能力之前，必须先完成一个 Codex-backed 通用 coding-agent runtime 底座。
这个底座不是产品定位上的退化，而是为了给所有 Primitive Module 提供稳定承载层：

```text
V1 Generic Agent CLI
  -> Codex upstream substrate
  -> Whale Codex bridge
  -> DeepSeek provider
  -> PrimitiveHost
  -> Built-in Primitive Modules
```

V1 的产品目标是先做到主流竞品级 coding agent CLI 能力：

- 能在真实 repo 内理解任务、读取文件、搜索代码、编辑文件、生成 patch。
- 能安全执行受控 shell/git/test/build 命令。
- 能做 read-before-write、diff preview、permission prompt、approval/deny。
- 能保持长会话、工具调用历史、session JSONL、replay 和 redaction。
- 能支持基础 slash commands，例如 `/status`、`/compact`、`/debug`、`/create`。
- 能通过 Codex upstream substrate 覆盖主流 coding agent 底座行为。

Primitive Module 的职责是在这个底座上增强或约束工作流，而不是补齐底座缺失能力。换句话说：

- 没有证据链模块，V1 也应该是可用的 coding agent CLI。
- 没有 Viewer 模块，V1 也应该能安全读写、执行、提交 patch。
- 没有技能自进化，Skills/MCP 也应该有静态可用版本。
- 差异化模块只能通过注册 gate/hook/reducer/permission overlay 扩展底座，不能要求 agent loop 为自己写专属分支。

---

## 三、证据链 Debug

### 3.1 目标

Debug 不是“先猜一个修复再试”。WhaleCode 的 Debug 必须像诊断系统一样工作：

```text
Symptom -> Reproduction -> Hypotheses -> Evidence Plans
        -> Evidence Records -> Root Cause Decision
        -> Fix Candidate -> Verification -> Regression Guard
```

核心约束：

- 没有复现或明确的不可复现说明，不能进入根因裁决。
- `HYPOTHESIZE` 阶段只读，不能写文件，不能执行会改变工作区的命令。
- 修复任务必须依赖 `RootCauseDecision`，不能直接依赖自然语言猜测。
- 被证伪的假设不得继续消耗 agent/tool 预算，除非 Judge 重新打开。

### 3.2 Runtime 组件

| 组件 | 职责 |
|------|------|
| `DebugCaseBuilder` | 从用户输入、错误日志、测试失败中提取 symptom、known facts、环境 |
| `ReproController` | 执行或记录复现步骤，产出 `ReproductionRecord` |
| `HypothesisRegistry` | 管理假设池、状态、置信度、互斥关系和版本 |
| `EvidencePlanner` | 把假设转成可执行、可验证的证据计划 |
| `EvidenceRunner` | 调用只读工具或受控测试命令收集证据 |
| `EvidenceEvaluator` | 更新 hypothesis 状态，计算支持/证伪/不确定 |
| `RootCauseJudge` | 在满足 gate 后裁决根因，记录残余风险 |
| `RegressionGuard` | 把复现步骤或最小失败用例固化为回归检查 |
| `StuckReporter` | 多轮仍不收敛时生成可交给用户的阻塞报告 |

### 3.3 Artifact Schema

```rust
pub struct DebugCase {
    pub id: DebugCaseId,
    pub goal: String,
    pub known_facts: Vec<KnownFact>,
    pub reproduction: Option<ReproductionRecord>,
    pub hypotheses: Vec<HypothesisId>,
    pub status: DebugStatus,
}

pub struct Hypothesis {
    pub id: HypothesisId,
    pub claim: String,
    pub mechanism: String,
    pub scope: Vec<ArtifactRef>,
    pub status: HypothesisStatus,
    pub confidence: f32,
    pub evidence_refs: Vec<EvidenceId>,
    pub invalidation_condition: String,
}

pub struct EvidencePlan {
    pub id: EvidencePlanId,
    pub hypothesis_id: HypothesisId,
    pub action: EvidenceAction,
    pub tool_policy: ToolPolicyRef,
    pub expected_signal: ExpectedSignal,
    pub cost: EvidenceCost,
    pub write_effect: WriteEffect,
}

pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub plan_id: EvidencePlanId,
    pub result: EvidenceResult,
    pub supports: Vec<HypothesisId>,
    pub refutes: Vec<HypothesisId>,
    pub reliability: EvidenceReliability,
    pub source_refs: Vec<ArtifactRef>,
}

pub struct RootCauseDecision {
    pub id: RootCauseDecisionId,
    pub selected_hypothesis: HypothesisId,
    pub supporting_evidence: Vec<EvidenceId>,
    pub refuted_alternatives: Vec<HypothesisId>,
    pub residual_risks: Vec<String>,
    pub fix_preconditions: Vec<String>,
}
```

### 3.4 Gate

| Gate | 通过条件 |
|------|----------|
| `ReproduceGate` | 有 confirmed reproduction；或明确 `not_reproducible` 且用户目标允许静态诊断 |
| `HypothesisGate` | 每个 hypothesis 可证伪；每个 hypothesis 有 evidence plan；至少包含一个替代假设 |
| `EvidencePlanGate` | plan 标明工具、目标、预期信号、写效应；写效应与当前 phase 权限匹配 |
| `RootCauseGate` | 至少一个 hypothesis 被强证据支持；主要替代假设被证伪或记录残余风险 |
| `FixStartGate` | 有 `RootCauseDecision`；有验证计划；Viewer 无 critical concern |
| `RegressionGate` | 修复后复现消失；至少一个回归检查被记录或明确豁免 |

### 3.5 评分与收敛

```text
evidence_score =
  reliability * relevance * reproducibility * signal_strength
  - collection_cost_penalty
```

收敛策略：

- 一轮最多 5 个活跃假设。
- 第二轮允许最多 8 个假设，但必须引用新增证据或被证伪假设。
- 连续 3 轮无强证据时输出 `stuck_report`。
- 多数 agent 同意不能替代证据；共识必须经过 `RootCauseGate`。

### 3.6 必测场景

- 写命令在 `HYPOTHESIZE` 阶段被拒绝。
- 被证伪假设不会继续派发 evidence work unit。
- 没有 `RootCauseDecision` 不能进入 fix。
- 修复后必须能 replay 出 symptom、evidence、decision、verification 的完整链路。

---

## 四、脚手架先行 Create

### 4.1 目标

Create 的风险不是“写不出代码”，而是边写边补基础设施，导致日志、测试、约束缺失。WhaleCode 的 Create 必须先建立最小可验证脚手架：

```text
Goal -> Design -> Scaffold Plan -> Scaffold Apply -> Scaffold Verify
     -> Feature Implementation -> Review -> Confirm
```

脚手架先行不是过度工程。它要求每个功能任务开始前，已经存在足够支撑该任务的日志、测试和约束。

### 4.2 Runtime 组件

| 组件 | 职责 |
|------|------|
| `ScaffoldPlanner` | 从设计 DAG 推导 logging/testing/constraints 需求 |
| `BaselineDetector` | 检测仓库已有脚手架，避免重复建设 |
| `ScaffoldDAGCompiler` | 把脚手架任务编译成功能任务的前置依赖 |
| `ScaffoldVerifier` | 执行最小验证命令，产出 `ScaffoldVerification` |
| `ConstraintRegistry` | 记录 fmt、lint、build、storybook、test 等项目约束 |
| `WaiverRegistry` | 记录无法建立某类脚手架时的显式豁免和原因 |

### 4.3 Artifact Schema

```rust
pub enum ScaffoldKind {
    Logging,
    Testing,
    Constraints,
    DeveloperWorkflow,
    Storybook,
}

pub struct ScaffoldRequirement {
    pub id: ScaffoldRequirementId,
    pub kind: ScaffoldKind,
    pub reason: String,
    pub required_for_tasks: Vec<TaskId>,
    pub minimum_contract: Vec<String>,
}

pub struct ScaffoldArtifact {
    pub id: ScaffoldArtifactId,
    pub requirement_id: ScaffoldRequirementId,
    pub files: Vec<PathBuf>,
    pub commands: Vec<VerificationCommand>,
    pub contract: ScaffoldContract,
}

pub struct ScaffoldVerification {
    pub artifact_id: ScaffoldArtifactId,
    pub command_results: Vec<CommandResultRef>,
    pub passed: bool,
    pub waivers: Vec<ScaffoldWaiver>,
}
```

### 4.4 Gate

| Gate | 通过条件 |
|------|----------|
| `ScaffoldPlanGate` | 对 logging、testing、constraints 三类给出 requirement 或豁免 |
| `ScaffoldApplyGate` | scaffold patch 只改基础设施文件；不混入 feature code |
| `ScaffoldVerifyGate` | 最小验证命令通过，或存在带风险说明的 waiver |
| `FeatureStartGate` | 每个 feature task 依赖至少一个通过验证的 scaffold artifact |
| `FeatureReviewGate` | feature patch 使用既有日志/测试/约束，而不是绕开 |

### 4.5 最小脚手架标准

| 类别 | 最小标准 |
|------|----------|
| Logging | 至少有结构化事件接口、trace/session/workunit id、redaction 边界 |
| Testing | 至少有可运行的测试入口、fixture 策略、mock model/tool 的方式 |
| Constraints | 至少有 fmt/lint/build 命令或明确说明项目暂不支持 |
| Developer Workflow | 本地运行、验证、调试命令记录在 repo 文档或脚本中 |
| Storybook | 仅当前端组件进入 scope 时启用；必须先有组件清单和 fixture |

### 4.6 必测场景

- feature task 没有 scaffold dependency 时 DAG validation 失败。
- scaffold patch 混入 feature code 时被拒绝。
- 测试入口不可运行时不能进入大规模实现。
- waiver 必须写入 session event，最终报告必须展示残余风险。

---

## 五、参考驱动设计

### 5.1 目标

参考驱动不是“给文档贴链接”。它要求每个重要设计选择都能回答：

1. 参考了什么。
2. 学到了什么。
3. 采用了什么。
4. 拒绝了什么。
5. 为什么适用于当前仓库。

Codex-first 审计覆盖成熟 coding-agent 底座；本节覆盖所有业务功能、工具选型、架构取舍和失败案例研究。

### 5.2 Runtime 组件

| 组件 | 职责 |
|------|------|
| `ReferenceTaskGenerator` | 从 design task 自动生成研究问题 |
| `SourceRanker` | 按权威性、时效、相关性、许可证给来源排序 |
| `ReferenceExtractor` | 把来源转为结构化 finding，而不是长摘录 |
| `ApplicabilityAssessor` | 判断参考是否适用于当前语言、框架、规模和约束 |
| `ReferenceGate` | 检查 design/patch 是否引用并吸收参考 |
| `CitationStore` | 保存 URL、本地路径、observed_at、license note、摘要 |

### 5.3 Source Tier

| Tier | 来源 | 使用规则 |
|------|------|----------|
| T0 | 当前 repo、AGENTS.md、现有测试、历史提交 | 总是优先；本地约束高于外部最佳实践 |
| T1 | 官方文档、标准、论文、项目维护者文档 | 高风险/易变领域必须优先使用 |
| T2 | Codex/OpenCode/Pi 等成熟参考实现 | coding-agent 基础设施优先使用 |
| T3 | 社区文章、issue、讨论、失败案例 | 可用于风险提示，不能单独决定架构 |
| T4 | 模型内部记忆 | 不可作为验收依据 |

### 5.4 Artifact Schema

```rust
pub struct ReferenceFinding {
    pub id: ReferenceFindingId,
    pub source: ReferenceSource,
    pub observed_at: DateTime<Utc>,
    pub claim: String,
    pub relevance: String,
    pub confidence: f32,
    pub license_note: Option<String>,
}

pub struct ReferenceDecision {
    pub id: ReferenceDecisionId,
    pub design_target: ArtifactRef,
    pub adopted: Vec<ReferenceFindingId>,
    pub rejected: Vec<ReferenceFindingId>,
    pub applicability_notes: Vec<String>,
    pub freshness_risk: FreshnessRisk,
}
```

### 5.5 Gate

| Gate | 通过条件 |
|------|----------|
| `DesignReferenceGate` | 重大设计必须引用 T0 + 至少一个 T1/T2；无法引用时记录原因 |
| `ImplementationReferenceGate` | 高风险实现任务必须有 pattern/source；小修可引用本地相似代码 |
| `FreshnessGate` | API、价格、法规、包版本、模型能力等易变信息必须 live verify |
| `LicenseGate` | 任何复制或改写代码前必须有 license boundary |
| `NoCitationLaunderingGate` | 引用必须影响设计决策；无关链接不计数 |

### 5.6 必测场景

- 只有无关链接的 design artifact 不能通过 gate。
- Codex-first 范围的模块没有 Codex 路径时不能进入实现。
- 易变 API 信息没有 `observed_at` 时 warning。
- license 未明确的参考只能 design-only，不能生成 copied implementation。

---

## 六、独立 Viewer

### 6.1 目标

Viewer 不是第二个 Reviewer。Reviewer 检查产物是否满足验收；Viewer 质疑产物背后的隐藏假设、证据不足、错误共识和流程偏离。

Viewer 的价值在于：

- 在 agent 自信但证据薄弱时打断。
- 在多数候选共享同一错误假设时保留少数路径。
- 在 phase gate 通过但语义风险仍高时提出 concern。
- 在 Reviewer 漏掉风险时审查 Reviewer。

### 6.2 Runtime 组件

| 组件 | 职责 |
|------|------|
| `ViewerScheduler` | 根据 trigger、budget、risk 决定是否启动 Viewer |
| `ViewerPromptPack` | 构造只含 artifact、evidence、gate result 的上下文 |
| `ConcernClassifier` | 将 Viewer 输出归类为 critical/warning/suggestion/question |
| `ConcernStore` | 持久化 concern、状态、resolution 和关联 artifact |
| `ConcernResolver` | 追踪 concern 是否被回应、修复、豁免或升级 |
| `ViewerBudget` | 控制 Pro 调用频率、token 和延迟 |

### 6.3 Trigger

| Trigger | 默认策略 |
|---------|----------|
| `phase_gate` | 默认开启 |
| `artifact_written` | 默认开启，但低风险 artifact 可采样 |
| `permission_escalation` | 默认开启，critical path |
| `root_cause_decision` | 默认开启 |
| `patch_apply_candidate` | 默认开启 |
| `reference_decision` | 高风险设计开启 |
| `evolution_proposal` | 默认开启 |
| `stuck_state` | 默认开启 |
| `every_token` | 默认关闭，仅 strict 模式允许 |

### 6.4 Artifact Schema

```rust
pub struct ViewerConcern {
    pub id: ConcernId,
    pub target: ArtifactRef,
    pub severity: ConcernSeverity,
    pub category: ConcernCategory,
    pub claim: String,
    pub evidence_refs: Vec<ArtifactRef>,
    pub counter_proposal: Option<String>,
    pub confidence: f32,
    pub status: ConcernStatus,
}

pub struct ConcernResolution {
    pub concern_id: ConcernId,
    pub action: ConcernAction,
    pub resolved_by: AgentId,
    pub evidence_refs: Vec<ArtifactRef>,
    pub residual_risk: Option<String>,
}
```

### 6.5 Routing

| Severity | Runtime 行为 |
|----------|--------------|
| `critical` | 阻止 phase transition 或 patch apply，必须 resolution |
| `warning` | 附加到下个 work unit 上下文，Reviewer 必须回应 |
| `question` | 高风险 phase 需要回答；低风险可记录 |
| `suggestion` | 记录到 session，默认不阻塞 |

### 6.6 独立性约束

- Viewer 只看 artifact、evidence、gate result、diff、logs 摘要。
- Viewer 不看其他 agent 的隐藏推理。
- Viewer 只读，唯一写能力是 `raise_concern`。
- Viewer 不参与实现，不拥有 patch。
- Viewer prompt 与实现/审查 agent 分离。
- Viewer concern 必须 event-sourced，可 replay，可被审查。

### 6.7 必测场景

- critical concern 阻止 phase transition。
- warning concern 会进入 Reviewer 的必须回应清单。
- Viewer 不能调用 write/edit/shell write 工具。
- session replay 后 concern 状态与 live 状态一致。

---

## 七、技能自进化

### 7.1 目标

技能自进化不是让 agent 随意改自己的 prompt。它是一个受控学习回路：

```text
Observe -> Detect -> Propose -> Evaluate -> Review -> Canary -> Promote/Rollback
```

Phase 1/2 只实现 telemetry 和 proposal schema。自动发布必须延后，且只能在有测试、回滚和 Viewer 审查后启用。

### 7.2 Runtime 组件

| 组件 | 职责 |
|------|------|
| `SkillTelemetryCollector` | 记录调用、结果、耗时、上下文、失败原因，默认 redacted |
| `SkillQualityAnalyzer` | 聚合成功率、重试率、用户修正、Viewer concern、rollback |
| `EvolutionTriggerEngine` | 根据阈值、批量窗口、用户反馈触发分析 |
| `EvolutionProposalGenerator` | 生成结构化 proposal，不直接修改 skill |
| `EvolutionSandbox` | 在 fixture / historical invocation 上评估变更 |
| `SkillVersionManager` | semver、canary、promote、rollback |
| `EvolutionReviewer` | Reviewer + Viewer 双重审查 proposal |

### 7.3 Artifact Schema

```rust
pub struct SkillInvocationEvent {
    pub id: SkillInvocationId,
    pub skill: SkillRef,
    pub version: SemVer,
    pub caller: AgentId,
    pub task_kind: WorkflowKind,
    pub result: SkillResultKind,
    pub duration_ms: u64,
    pub redacted_args_ref: ArtifactRef,
    pub error_class: Option<String>,
}

pub struct EvolutionProposal {
    pub id: EvolutionProposalId,
    pub skill: SkillRef,
    pub current_version: SemVer,
    pub trigger: EvolutionTrigger,
    pub evidence: Vec<SkillInvocationId>,
    pub proposed_change: SkillChangeSet,
    pub expected_impact: Vec<MetricDelta>,
    pub risk: EvolutionRisk,
    pub rollout_plan: RolloutPlan,
    pub rollback_plan: RollbackPlan,
}

pub struct SkillEvaluationRun {
    pub proposal_id: EvolutionProposalId,
    pub fixtures: Vec<ArtifactRef>,
    pub before_metrics: SkillMetrics,
    pub after_metrics: SkillMetrics,
    pub regressions: Vec<String>,
}
```

### 7.4 Gate

| Gate | 通过条件 |
|------|----------|
| `TelemetryGate` | proposal 必须引用足够 invocation 或用户反馈，不能凭空生成 |
| `PrivacyGate` | telemetry 不含 secret、私有路径、完整用户代码，除非本地 debug opt-in |
| `EvaluationGate` | proposal 至少跑过 fixture 或 historical dry-run |
| `ViewerGate` | Viewer 无 critical concern |
| `VersionGate` | semver bump 与行为变化一致 |
| `RollbackGate` | 每次发布都有可执行 rollback plan |

### 7.5 发布策略

| 阶段 | 行为 |
|------|------|
| Phase 1 | 只记录 telemetry，不生成自动 proposal |
| Phase 2 | 可生成 proposal，但不自动修改 skill |
| Phase 3 | 低风险 patch 可 canary，但必须本地配置显式开启 |
| Phase 4+ | minor/major 仍需 review；major 默认需要用户确认 |

### 7.6 风险与防护

| 风险 | 防护 |
|------|------|
| 数据投毒 | proposal 必须引用多次调用和不同上下文；异常反馈降权 |
| 隐私泄露 | telemetry 默认 redacted，export 前二次脱敏 |
| 过拟合 | EvolutionSandbox 用历史 fixture 对比 before/after |
| 自我强化错误 | Viewer 审查 proposal；rollback 指标进入质量分析 |
| 破坏兼容 | semver gate + canary + rollback |

---

## 八、跨原语协同

| 场景 | 协同方式 |
|------|----------|
| Create | 参考驱动产生 `ReferenceDecision`，脚手架先行产生 `ScaffoldArtifact`，Viewer 审查 design/scaffold/patch |
| Debug | 证据链产生 `RootCauseDecision`，Viewer 挑战证据不足，RegressionGuard 将修复固化 |
| Skill Evolution | telemetry 来自 tool/skill/session events，proposal 被 Viewer 审查，发布必须有 rollback |
| Multi-Agent | Scout/Analyst/Implementer/Judge 都只交换 artifact refs，不交换大段聊天 |
| Web Viewer | 只读消费这些 artifact 和 event，展示证据链、scaffold gate、reference pack、concern、evolution proposal |

---

## 九、事件模型

这些事件必须进入 `whalecode-protocol` 和 JSONL SessionStore：

```text
primitive_registered
primitive_enabled
primitive_disabled
primitive_eval_recorded
reference_task_created
reference_finding_recorded
reference_decision_made
scaffold_requirement_created
scaffold_artifact_created
scaffold_verified
debug_case_created
reproduction_recorded
hypothesis_created
evidence_plan_created
evidence_recorded
hypothesis_updated
root_cause_decided
regression_guard_created
viewer_concern_raised
concern_resolved
skill_invocation_recorded
skill_evolution_triggered
skill_evolution_proposed
skill_evaluation_completed
skill_version_published
skill_rollback_executed
```

所有事件必须包含：

- `schema_version`
- `session_id`
- `trace_id`
- `turn_id`
- `phase`
- `agent_id` 或 `system_actor`
- `redaction_summary`

---

## 十、落地顺序

本节只描述差异化原语的落地顺序。项目整体顺序必须先完成 Generic Agent Substrate，再启用这些特化模块。

### Phase 0 — Generic Agent CLI Substrate

前置交付：

- model streaming + tool-call sub-turn。
- single-agent loop。
- read/search/edit/write/shell/git tools。
- permission、sandbox、approval、grant。
- patch/workspace safety。
- JSONL session、redaction、replay。
- context compaction、fragment、artifact refs。
- primitive host skeleton。

验收：

- CLI 能像主流 coding agent 一样完成真实 repo 中的读、搜、改、测、解释和 diff。
- primitive host 关闭所有非基础模块后，CLI 仍然可作为通用 coding agent 使用。
- 后续每个 Primitive Module 都能单独启停并被测试。

### Phase 1A — Protocol First

交付：

- `PrimitiveManifest` / `PrimitiveModule` contract / `PrimitiveRegistry`。
- primitive enable/disable/config event。
- `ReferenceFinding` / `ReferenceDecision`
- `ScaffoldRequirement` / `ScaffoldArtifact` / `ScaffoldVerification`
- `DebugCase` / `Hypothesis` / `EvidencePlan` / `EvidenceRecord` / `RootCauseDecision`
- `ViewerConcern` / `ConcernResolution`
- `SkillInvocationEvent` / `EvolutionProposal`
- 对应 session events 和 replay reducer skeleton。

验收：

- fixture JSONL 能 replay 出 reference、scaffold、debug、viewer、skill evolution 状态。
- schema version 和 redaction summary 全部存在。
- primitive 被关闭后，对应 gate/hook 不再参与新 phase，但历史 session 仍可 replay。

### Phase 1B — Single-Agent Primitive Gates

交付：

- 单 agent Debug 证据链 gate。
- 单 agent Create 脚手架 gate。
- reference gate。
- Viewer concern 只记录，不默认阻断。

验收：

- Debug 无证据不能 fix。
- Create 无 scaffold 不能 feature implement。
- design 无有效 reference 不能进入 implement。

### Phase 2 — Multi-Agent Expansion

交付：

- Evidence Race。
- Plan Tournament + Scaffold planning。
- Viewer critical concern 阻断。
- 多 agent artifact refs。

验收：

- 并行 evidence runner 能证伪假设。
- Viewer 能挑战 Judge/Reviewer。
- Patch League 候选都能引用 scaffold/reference/debug artifacts。

### Phase 5 — Skill Evolution Runtime

交付：

- telemetry 聚合。
- proposal generation。
- evaluation sandbox。
- canary / promote / rollback。

验收：

- proposal 必须引用 telemetry。
- Viewer critical concern 阻止发布。
- rollback 可 replay。

---

## 十一、外部来源

1. Delta Debugging: Zeller and Hildebrandt, "Simplifying and Isolating Failure-Inducing Input". https://www.st.cs.uni-saarland.de/papers/tse2002/
2. OpenTelemetry documentation: traces, metrics, logs, context propagation and redaction-related guidance. https://opentelemetry.io/docs/
3. Google SRE Book: Monitoring Distributed Systems. https://sre.google/resources/book-update/monitoring-distributed-systems/
4. Reflexion: Language Agents with Verbal Reinforcement Learning. https://arxiv.org/abs/2303.11366
5. Self-Refine: Iterative Refinement with Self-Feedback. https://arxiv.org/abs/2303.17651
6. Can LLM Agents Really Debate? A Controlled Study of Multi-Agent Debate in Logical Reasoning. https://arxiv.org/abs/2511.07784
7. Markdown Architectural Decision Records. https://adr.github.io/madr/

---

## 十二、执行结论

这五个能力都要进入 runtime：

- V1 先交付 Codex-derived 竞品级通用 coding agent CLI 底座，差异化能力在其上插件化增强。
- 它们以可插拔 Primitive Module 进入 runtime，而不是写死进 agent loop。
- 证据链 Debug 是诊断模块，提供 DebugCase / Evidence / RootCause gates。
- 脚手架先行是 Create gate 模块，不是提示词建议。
- 参考驱动是 ReferenceDecision gate 模块，不是文档引用数量游戏。
- 独立 Viewer 是只读对抗控制面模块，不是第二个 Reviewer。
- 技能自进化是 proposal + evaluation + versioning + rollback 模块，不是自动改 prompt。
