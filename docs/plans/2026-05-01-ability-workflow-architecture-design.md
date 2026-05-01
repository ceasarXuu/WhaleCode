# Ability 与 Workflow 架构设计

---

## 一、结论

WhaleCode 的差异化能力不应该直接命名为一组散落的“模式”或 slash command。
更稳定的产品语言是两层：

```text
Ability: 产品内置的原子能力基建
Workflow: 面向具体软件工程场景的可执行流程组合
```

`Create` 和 `Debug` 不是孤立功能，而是第一批内置 workflow。
它们运行在一组更底层的 ability 之上：

- 任务建模。
- 证据链。
- 脚手架先行。
- 参考驱动设计。
- 工具链与环境探测。
- 日志与遥测建设。
- 测试与回归守卫。
- 权限与写效应控制。
- 多 agent 分工。
- Action Map DAG 编排。
- artifact schema 与 replay。
- Viewer 可观测性。

更具体的场景，例如“开发一个 macOS app”，不应该变成又一个硬编码大模式。
它应该是一个 `macos_app_development` workflow，由上述 ability 组合出阶段、产物、gate、工具策略和交付检查：

```text
需求/产品定义
  -> 架构与文档设计
  -> 开发环境和 Apple 工具链搭建
  -> 应用实现
  -> 测试、日志、崩溃诊断
  -> 签名、公证、App Store / 站外分发合规
  -> 发布、回滚、运维
```

因此，`ability` 是 WhaleCode 的“内置原子能力层”，`workflow` 是“场景化软件工程执行层”。
runtime 只能认识稳定 contract、schema、event、gate 和 replay reducer，不能认识每个行业/平台的自然语言流程细节。

本文仍处于设计早期，必须保持克制。第一版只定义足够支撑 `Create`、`Debug` 和一个场景化 workflow 原型的最小 contract，不提前建设完整市场、复杂策略语言、图形化编排器、跨团队治理模型或行业模板库。

---

## 二、设计分层

### 2.1 从上到下的产品模型

```text
User Goal
  -> Workflow Selection
  -> Workflow Definition
  -> Ability Plan
  -> Action Map DAG
  -> Agent / Tool / Artifact Execution
  -> Gate / Verification
  -> Session Event / Replay / Viewer
```

| 层级 | 职责 | 例子 |
|---|---|---|
| User Goal | 用户自然语言目标 | “帮我做一个 macOS 菜单栏 app” |
| Workflow | 场景化工程流程 | `macos_app_development`、`debug_regression`、`create_feature` |
| Ability | 可复用原子能力 | `scaffold_first`、`evidence_chain`、`apple_toolchain_probe` |
| Action Map | 可执行 DAG | node、edge、lease、result、blocked reason |
| Runtime Substrate | Codex-derived 执行底座 | shell、patch、session、permissions、MCP、skills、logs |
| Artifact/Event | 可持久化状态 | design doc、evidence record、test report、release checklist |
| Viewer/Replay | 可观测与可复盘 | DAG 进度、gate 结果、工具调用、关键证据 |

这套分层避免三个问题：

1. 把 `Create` / `Debug` 写死到 agent loop，导致后续新增场景都要改核心 runtime。
2. 把 macOS、Android、Web、CLI、库开发等流程都塞进提示词，导致无法验证、无法回放、无法淘汰。
3. 把 workflow 降级成 skills 或文档约束，导致 runtime 不能真正控制 phase、gate、权限、artifact 和 replay。

### 2.2 Ability 与 Workflow 的边界

`ability` 解决“能做什么、如何被约束、如何被度量”。

`workflow` 解决“在某个场景里，什么时候用哪些 ability，产出什么，如何过关”。

```text
Ability = capability primitive + schema + gate + policy + telemetry
Workflow = phase machine + DAG template + ability binding + scenario gate
```

Ability 可以扩展。新增 ability 应只补充一个可复用原子能力，例如 `apple_signing_diagnostics` 或 `runtime_smoke`，不能偷偷包含完整场景流程。

Workflow 可以自定义，也可以自由编排。一个 workflow 可以复用内置 ability、绑定第三方 ability、调整 phase 顺序、插入/删除节点、替换 gate，但必须保留 runtime 可理解的结构化状态。

| 问题 | 属于 Ability | 属于 Workflow |
|---|---|---|
| 如何建立证据链 | 是 | 否 |
| Debug 阶段是否允许写文件 | 部分，提供权限 overlay | 是，按 phase 决定 |
| macOS app 需要签名和 notarization | 否 | 是 |
| 如何记录工具调用和证据 | 是 | 否 |
| 是否先搭建日志/测试/约束再写功能 | 是 | 是，Create 类 workflow 强制启用 |
| 这个任务分几个阶段、每阶段交付什么 | 否 | 是 |
| gate 如何持久化和 replay | 是 | workflow 只消费结果 |

### 2.3 与既有 PrimitiveModule 的关系

`PrimitiveModule` 可以保留为工程实现层 contract。
`Ability` 是产品架构语言，`PrimitiveModule` 是 Rust runtime 中承载 ability 的注册机制。

```text
Ability Manifest
  -> PrimitiveModule manifest
  -> artifact schemas
  -> event schemas
  -> phase hooks
  -> permission overlays
  -> replay reducers
  -> viewer triggers
  -> eval specs
```

也就是说，现有“证据链 Debug”“脚手架先行 Create”“参考驱动设计”“独立 Viewer”“技能自进化”都应迁移为内置 ability。
`Create` / `Debug` 则变成使用这些 ability 的默认 workflow。

---

## 三、Ability 架构

### 3.1 Ability Manifest

每个 ability 必须有可机器读取的 manifest。

```rust
pub struct AbilityManifest {
    pub id: AbilityId,
    pub name: String,
    pub version: SemVer,
    pub stability: AbilityStability,
    pub category: AbilityCategory,
    pub default_enabled: bool,
    pub dependencies: Vec<AbilityId>,
    pub conflicts: Vec<AbilityId>,
    pub required_artifacts: Vec<ArtifactSchemaRef>,
    pub emitted_events: Vec<EventSchemaRef>,
    pub gates: Vec<GateSpec>,
    pub permission_overlays: Vec<PermissionOverlaySpec>,
    pub telemetry: AbilityTelemetrySpec,
    pub eval: AbilityEvalSpec,
}
```

关键原则：

- `id` 稳定，不能跟展示名绑定。
- `version` 用于 replay migration 和 workflow pinning。
- `default_enabled` 必须由 eval 结果支撑，不能因为愿景重要就默认打开。
- `dependencies` 只能依赖 ability id，不能依赖某个 workflow。
- `permission_overlays` 只描述能力边界，不直接执行工具。

### 3.2 Ability 分类

第一版建议按产品能力而不是代码模块分类：

| 分类 | 内置 Ability | 职责 |
|---|---|---|
| Planning | `goal_modeling`、`task_decomposition`、`action_map_planning` | 把用户目标变成可执行 DAG |
| Evidence | `evidence_chain`、`reproduction_capture`、`root_cause_judgment` | Debug 收敛与证据裁决 |
| Creation | `scaffold_first`、`constraint_modeling`、`implementation_slice` | Create 发散但不失控 |
| Research | `reference_scan`、`best_practice_audit`、`failure_case_scan` | 外部参考驱动设计 |
| Environment | `toolchain_probe`、`dependency_bootstrap`、`runtime_smoke` | 环境搭建和可运行性验证 |
| Quality | `test_harness_design`、`regression_guard`、`code_review` | 测试、回归、审查 |
| Observability | `logging_baseline`、`telemetry_trace`、`diagnostic_bundle` | 日志与问题定位基建 |
| Governance | `permission_policy`、`secret_redaction`、`compliance_check` | 权限、隐私、合规 |
| Collaboration | `agent_role_assignment`、`parallel_exploration`、`consensus_judge` | 多 agent 协同 |
| Lifecycle | `release_packaging`、`rollback_plan`、`ops_runbook` | 发布、回滚、运维 |

### 3.3 Ability 状态机

每个 ability 在一次 workflow run 中都有独立实例状态。

```text
Available -> Selected -> Planned -> Active -> Satisfied
                         |          |
                         |          -> Blocked
                         -> Skipped
```

状态定义：

| 状态 | 含义 |
|---|---|
| `Available` | runtime 知道这个 ability，当前 workflow 可以选择 |
| `Selected` | workflow 决定启用，但尚未展开为 DAG node |
| `Planned` | ability 已生成 node、artifact requirement 或 gate |
| `Active` | 正在执行相关 node 或等待工具结果 |
| `Satisfied` | ability 的 gate 已通过 |
| `Blocked` | 缺工具、缺权限、缺用户决策或证据不足 |
| `Skipped` | workflow 明确豁免，必须有理由和风险记录 |

### 3.4 Ability Artifact

ability 不能只产出自然语言段落。
每个关键能力都要产出结构化 artifact。

```rust
pub struct AbilityArtifact {
    pub id: ArtifactId,
    pub ability_id: AbilityId,
    pub schema_id: ArtifactSchemaId,
    pub schema_version: SemVer,
    pub workflow_run_id: WorkflowRunId,
    pub producer_node_id: MapNodeId,
    pub body: serde_json::Value,
    pub source_refs: Vec<ArtifactRef>,
    pub created_at_ms: i64,
}
```

示例：

| Ability | Artifact |
|---|---|
| `evidence_chain` | `DebugCase`、`Hypothesis`、`EvidenceRecord`、`RootCauseDecision` |
| `scaffold_first` | `ScaffoldPlan`、`LoggingPlan`、`TestingPlan`、`ConstraintPlan` |
| `reference_scan` | `ReferenceAudit`、`SourceClaim`、`DesignImplication` |
| `toolchain_probe` | `ToolchainInventory`、`MissingDependency`、`InstallRecord` |
| `release_packaging` | `ReleaseCandidate`、`SigningRecord`、`DistributionChecklist` |

### 3.5 Ability Gate

gate 是确定性检查，不是 agent 自称“完成”。

```rust
pub struct AbilityGateResult {
    pub gate_id: GateId,
    pub ability_id: AbilityId,
    pub status: GateStatus,
    pub blocking_reasons: Vec<String>,
    pub required_artifacts: Vec<ArtifactRef>,
    pub evidence_refs: Vec<ArtifactRef>,
    pub checked_at_ms: i64,
}
```

第一版 gate 只做低耦合检查：

- 必要 artifact 是否存在。
- artifact schema 是否有效。
- 当前 phase 是否允许写效应。
- 必要测试/冒烟命令是否执行。
- 外部引用数量是否满足要求。
- 是否存在 unresolved blocker。
- Viewer / Reviewer 是否有 critical concern。

---

## 四、Workflow 架构

### 4.0 Runtime 级别定位

Workflow 是 runtime 级别能力，不是 skills、提示词或团队规范的同义词。

Skills 可以给模型提供领域知识、操作说明和局部工具习惯，但它不能可靠地保证：

- phase transition 一定被检查。
- 写效应一定被当前阶段约束。
- 必要 artifact 一定按 schema 记录。
- gate 结果一定可 replay。
- Viewer 一定能看到真实执行状态。

因此，workflow 必须进入 session runtime：

```text
WorkflowDefinition
  -> WorkflowRun state
  -> AbilityInstance state
  -> ActionMap DAG
  -> GateResult
  -> SessionEvent
```

Skills 可以被 workflow 调用或引用，但不能替代 workflow。

### 4.1 Workflow Definition

workflow 是场景化工程流程，不是提示词模板。

```rust
pub struct WorkflowDefinition {
    pub id: WorkflowId,
    pub name: String,
    pub version: SemVer,
    pub domain: WorkflowDomain,
    pub entry_conditions: Vec<EntryCondition>,
    pub phases: Vec<WorkflowPhase>,
    pub ability_bindings: Vec<AbilityBinding>,
    pub base_map_template: BaseMapTemplate,
    pub completion_gates: Vec<GateSpec>,
    pub default_model_policy: ModelPolicy,
    pub telemetry: WorkflowTelemetrySpec,
}
```

第一版不需要完整 workflow DSL。可以先支持内置 Rust/JSON 定义：

- 固定字段描述 phases、ability bindings、base map template 和 completion gates。
- 允许用户通过配置选择 workflow、关闭某些 optional ability、调整少量 phase 参数。
- 暂不支持复杂条件表达式、循环、动态插件市场和图形化编辑。

长期目标是可自定义、可自由编排，但早期实现应先验证 runtime contract 是否正确。

### 4.2 Workflow Phase

workflow phase 是用户可理解的工程阶段。
ability phase hook 是 runtime 可执行的约束。

```rust
pub struct WorkflowPhase {
    pub id: PhaseId,
    pub title: String,
    pub purpose: String,
    pub required_abilities: Vec<AbilityId>,
    pub optional_abilities: Vec<AbilityId>,
    pub allowed_write_effect: WriteEffectPolicy,
    pub required_artifacts: Vec<ArtifactSchemaRef>,
    pub exit_gates: Vec<GateId>,
}
```

phase 需要表达：

- 这一阶段为什么存在。
- 可以读什么、写什么、执行什么。
- 产出哪些 artifact。
- 哪些 gate 通过才能进入下一阶段。
- 哪些失败能降级，哪些失败必须阻塞。

### 4.3 Workflow Run

每次执行 workflow 都要有可 replay 的 runtime state。

```rust
pub struct WorkflowRun {
    pub id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version: SemVer,
    pub status: WorkflowStatus,
    pub active_phase: PhaseId,
    pub selected_abilities: Vec<AbilityInstanceId>,
    pub action_map_id: ActionMapId,
    pub artifact_refs: Vec<ArtifactRef>,
    pub gate_results: Vec<GateResultRef>,
    pub created_from_user_goal: String,
}
```

这和 Action Map 的关系：

- `WorkflowRun` 是语义层。
- `ActionMapInstance` 是执行 DAG。
- `AbilityInstance` 是能力层状态。
- `SessionEvent` 是事实来源。
- replay reducer 从 session event 重建三者。

### 4.4 Workflow 选择

workflow selection 不应该靠关键词固定答复。
它应该是机械分类 + agent 确认的组合：

```text
User Goal
  -> detect candidate workflows
  -> ask model to map goal to workflow candidates
  -> if confidence high: start workflow
  -> if ambiguous: ask user for decision
  -> if no workflow: fall back to generic agent
```

候选 workflow 只影响约束和基建，不替代模型回答。
这符合“自然语言输入必须进入 Agent/Model 路径”的项目约束。

---

## 五、Create 与 Debug 的定位

### 5.1 Create Workflow

`create_feature` 是通用构建 workflow。

```text
Goal Intake
  -> Reference / Constraint Design
  -> Scaffold Plan
  -> Logging + Testing + Constraints
  -> Implementation Slices
  -> Verification
  -> Documentation / Handoff
```

默认启用 ability：

- `goal_modeling`
- `reference_scan`
- `scaffold_first`
- `logging_baseline`
- `test_harness_design`
- `constraint_modeling`
- `implementation_slice`
- `regression_guard`
- `code_review`

关键 gate：

| Gate | 要求 |
|---|---|
| `DesignGate` | 需求、约束、外部参考、风险边界已记录 |
| `ScaffoldGate` | 日志、测试、约束脚手架先于核心实现落地或被明确豁免 |
| `ImplementationGate` | 每个 slice 有 owner、依赖、验收标准 |
| `VerificationGate` | 冒烟、回归、关键运行时断言完成 |
| `HandoffGate` | 文档、日志入口、后续风险清楚 |

### 5.2 Debug Workflow

`debug_case` 是通用诊断 workflow。

```text
Symptom Intake
  -> Reproduction
  -> Hypotheses
  -> Evidence Plan
  -> Evidence Collection
  -> Root Cause Decision
  -> Fix
  -> Regression Guard
```

默认启用 ability：

- `reproduction_capture`
- `evidence_chain`
- `toolchain_probe`
- `logging_baseline`
- `diagnostic_bundle`
- `root_cause_judgment`
- `regression_guard`
- `code_review`

关键 gate：

| Gate | 要求 |
|---|---|
| `ReproductionGate` | 有复现、不可复现说明或用户允许静态诊断 |
| `HypothesisGate` | 至少两个可证伪假设，且每个假设有证据计划 |
| `EvidenceGate` | 关键证据已收集，写效应符合 phase 权限 |
| `RootCauseGate` | 根因有证据支持，替代假设被证伪或记录残余风险 |
| `FixGate` | 修复关联 RootCauseDecision |
| `RegressionGate` | 原症状消失，并新增或记录回归守卫 |

---

## 六、macOS App Development Workflow 示例

### 6.1 为什么这是 Workflow，不是 Ability

“开发一个 macOS app”包含产品、平台、工具链、合规、发布、运维。
它不是一个原子能力，而是组合能力：

- 需求建模。
- Apple 生态工具链探测。
- Swift / SwiftUI / AppKit 项目结构生成。
- 本地构建与运行。
- sandbox、entitlement、privacy manifest、签名、公证。
- App Store Connect 或站外分发。
- 崩溃日志、用户反馈、版本升级。

这些步骤会随 Apple 平台变化而变化，因此必须在 workflow 层可版本化、可替换。

### 6.2 阶段设计

```text
macos_app_development
  P0 Product Frame
  P1 Architecture & Design Docs
  P2 Local Toolchain Bootstrap
  P3 Project Scaffold
  P4 Feature Implementation
  P5 Quality & Diagnostics
  P6 Apple Compliance & Distribution
  P7 Release Operations
```

| Phase | 目标 | Required Ability | Exit Gate |
|---|---|---|---|
| P0 Product Frame | 明确用户、场景、交互、非目标 | `goal_modeling`、`constraint_modeling` | 产品目标和非目标可复述 |
| P1 Architecture & Design Docs | 决定 SwiftUI/AppKit、数据、权限、模块边界 | `reference_scan`、`design_artifact` | 设计文档和技术选择记录完成 |
| P2 Local Toolchain Bootstrap | 验证 Xcode、SDK、证书、notary、CI 可用 | `toolchain_probe`、`dependency_bootstrap` | 工具链 inventory 和缺口清单完成 |
| P3 Project Scaffold | 建项目、日志、测试、构建脚本、约束 | `scaffold_first`、`logging_baseline`、`test_harness_design` | scaffold 冒烟通过 |
| P4 Feature Implementation | 分 slice 实现功能 | `implementation_slice`、`agent_role_assignment` | slice 测试和集成检查通过 |
| P5 Quality & Diagnostics | UI、性能、崩溃、日志、回归 | `runtime_smoke`、`diagnostic_bundle`、`regression_guard` | 关键用户路径可运行 |
| P6 Apple Compliance & Distribution | 签名、公证、App Store 审核准备 | `compliance_check`、`release_packaging` | 签名/公证/提交材料检查完成 |
| P7 Release Operations | 发布、回滚、监控、反馈闭环 | `ops_runbook`、`telemetry_trace` | release runbook 和回滚路径完成 |

### 6.3 Apple 专属 Ability 扩展

macOS workflow 可以引入平台专属 ability，但它们仍应是原子能力：

| Ability | 职责 |
|---|---|
| `apple_toolchain_probe` | 检查 Xcode、Command Line Tools、SDK、simulator、notarytool、证书状态 |
| `apple_signing_diagnostics` | 诊断 signing identity、provisioning profile、entitlement mismatch |
| `macos_distribution_check` | 区分 App Store、Developer ID、内部分发路径 |
| `app_store_submission_prep` | 检查 metadata、截图、隐私、审核说明、版本号 |
| `macos_runtime_smoke` | 启动 app、检查日志、权限弹窗、崩溃、主窗口或菜单栏入口 |

这些 ability 不应该只服务 macOS app workflow。
例如 `apple_signing_diagnostics` 也能被 iOS、visionOS、独立 CLI notarization workflow 复用。

### 6.4 macOS Workflow 的 BaseMap

第一版可以用静态 BaseMap 表达：

```text
node: product_frame
node: architecture_decision
node: apple_toolchain_inventory
node: scaffold_project
node: logging_and_tests
node: implement_core_feature
node: runtime_smoke
node: signing_and_notarization
node: release_runbook

edges:
product_frame -> architecture_decision
architecture_decision -> apple_toolchain_inventory
apple_toolchain_inventory -> scaffold_project
scaffold_project -> logging_and_tests
logging_and_tests -> implement_core_feature
implement_core_feature -> runtime_smoke
runtime_smoke -> signing_and_notarization
signing_and_notarization -> release_runbook
```

后续可以根据项目类型扩展：

- 菜单栏 app。
- Document-based app。
- Electron/Tauri 包装 macOS app。
- CLI + LaunchAgent。
- App Store 分发。
- Developer ID 站外分发。

---

## 七、Runtime 集成

### 7.0 克制边界

这部分只定义 runtime 需要承载的最小状态和事件，不要求立刻实现完整产品平台。

第一版必须避免：

- workflow marketplace。
- 复杂可视化编排器。
- 独立 workflow 数据库。
- 另起一套 agent runtime。
- 把每个垂直场景做成独立代码分支。
- 过早定义全部 ability taxonomy。

第一版必须证明：

- workflow 可以选择一组 ability。
- ability 可以生成结构化 artifact 和 gate。
- workflow 可以生成 Action Map。
- phase 和 permission 能被 runtime 约束。
- session event 可以 replay 出 workflow/ability/map 状态。

### 7.1 新增模块边界

建议在现有 Action Map 之上增加更高一层：

```text
core/src/ability/
  manifest.rs
  registry.rs
  artifact.rs
  gate.rs
  telemetry.rs
  reducer.rs

core/src/workflow/
  definition.rs
  registry.rs
  selection.rs
  run.rs
  base_maps.rs
  gates.rs
```

职责：

- `ability` 只定义原子能力 contract，不知道具体 macOS workflow。
- `workflow` 组合 ability，生成 Action Map。
- `action_map` 执行 DAG、lease、result、restart、replay。
- `session` 持久化 event。
- `tui` 只提供机械入口，例如 `/workflow`、`/create`、`/debug`。

### 7.2 Event 体系

新增事件建议分两类：

```rust
pub enum EventMsg {
    AbilityRuntime(AbilityRuntimeEvent),
    WorkflowRuntime(WorkflowRuntimeEvent),
    MapRuntime(MapRuntimeEvent),
}
```

`WorkflowRuntimeEvent`：

- `WorkflowSelected`
- `WorkflowStarted`
- `PhaseEntered`
- `PhaseExited`
- `WorkflowBlocked`
- `WorkflowCompleted`
- `WorkflowAbandoned`

`AbilityRuntimeEvent`：

- `AbilitySelected`
- `AbilityPlanned`
- `AbilityActivated`
- `AbilitySatisfied`
- `AbilityBlocked`
- `AbilitySkipped`
- `AbilityGateEvaluated`
- `AbilityArtifactRecorded`

事件必须满足：

- JSON schema 稳定。
- 包含 version。
- 可从 rollout 重放。
- 不包含 secret。
- 能被 Viewer 直接消费。

### 7.3 Permission 与写效应

workflow phase 决定当前允许的写效应：

```text
ReadOnly
  -> WorkspaceWrite
  -> CommandWrite
  -> ExternalSideEffect
```

ability 提供 overlay：

- `evidence_chain` 在 hypothesis 阶段强制 ReadOnly。
- `scaffold_first` 允许创建测试、日志、配置脚手架。
- `release_packaging` 允许构建产物，但上传/发布属于 ExternalSideEffect。
- `compliance_check` 默认只读，除非 workflow 明确进入修复阶段。

任何写效应都要落 session event，避免 replay 时无法解释。

### 7.4 Viewer

Viewer 不应读取模型思考链来判断状态。
它读取：

- Workflow run。
- Active phase。
- Selected abilities。
- Action Map node/edge/result。
- Gate result。
- Artifact refs。
- Tool invocation summary。
- Telemetry counters。

展示层可以按三层组织：

```text
Workflow Timeline
Ability Matrix
Action Map DAG
```

---

## 八、测试与度量

### 8.1 Ability 测试

每个 ability 至少需要：

- manifest schema test。
- artifact validation test。
- gate deterministic test。
- replay reducer test。
- permission overlay test。
- telemetry field redaction test。

### 8.2 Workflow 测试

每个 workflow 至少需要：

- workflow definition schema test。
- phase transition test。
- BaseMap DAG validation test。
- required ability binding test。
- blocked state test。
- happy path replay fixture。

### 8.3 产品度量

不只看任务是否完成，还要看 ability 是否真的创造价值。

| 维度 | 指标 |
|---|---|
| 收敛 | Debug 假设数量是否下降，证伪是否减少无效修复 |
| 基建 | Create 是否先落日志/测试/约束 |
| 可靠性 | 回归测试是否覆盖原问题 |
| 可解释性 | Viewer 是否能还原关键决策 |
| 成本 | ability 带来的 token、工具调用、时间开销 |
| 可复用 | 同一 ability 是否被多个 workflow 使用 |
| 淘汰 | ability 默认启用后是否造成阻塞或低收益 |

---

## 九、参考依据

这些外部系统给这套设计提供了工程参照：

| 来源 | 可借鉴点 | 对 WhaleCode 的影响 |
|---|---|---|
| [Kubernetes Controllers](https://kubernetes.io/docs/concepts/architecture/controller/) | controller 持续观察当前状态并驱动到期望状态 | workflow/ability 应以 desired state、actual state、reconcile event 建模 |
| [GitHub Actions workflow syntax](https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions?s=09) | workflow 由 job、step、dependency、concurrency 组成 | Action Map DAG 应显式表达依赖和并发，而不是靠自然语言顺序 |
| [Backstage Software Templates](https://backstage.io/docs/features/software-templates) | scaffolder 用模板、参数、action 创建软件资产 | Create workflow 要把脚手架、参数、dry-run、审计作为一等对象 |
| [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/concepts/semantic-conventions/) | traces、metrics、logs 使用稳定语义字段 | ability/workflow event 应定义语义字段，方便 Viewer、分析和跨版本对齐 |
| [OpenFeature hooks](https://openfeature.dev/specification/sections/hooks) | hook 可以在 evaluation 生命周期中做校验、日志、遥测 | ability hook 应有明确生命周期，不应散落在 agent loop 分支里 |
| [Apple notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) | macOS 分发前需要签名、公证和自动化检查 | macOS app workflow 必须把签名、公证、分发合规作为独立 gate |
| [App Store Connect submit for review](https://developer.apple.com/help/app-store-connect/manage-submissions-to-app-review/submit-for-review/) | app 版本和内容提交需要经过审核流程 | Apple 发布 workflow 需要 metadata、审核说明、版本状态和材料检查 |

---

## 十、落地路线

### AW-0：文档与命名统一

目标：

- 用 `ability` / `workflow` 统一产品语言。
- 保留 `PrimitiveModule` 作为工程实现机制。
- 更新后续设计文档引用，避免 `primitive`、`mode`、`workflow` 混用。

验收：

- 新增本文档。
- 后续实现 issue / plan 使用 `Ability` 和 `Workflow` 命名。

### AW-1：Schema 与 Event

目标：

- 在 protocol 中新增 `AbilityRuntimeEvent`、`WorkflowRuntimeEvent`。
- 定义 `AbilityManifest`、`WorkflowDefinition`、`WorkflowRun` 的最小 schema。
- replay reducer 可以从事件恢复 workflow run 和 ability instance 状态。

测试：

- serialization roundtrip。
- replay fixture。
- gate result schema validation。

### AW-2：Registry 与 BaseMap 生成

目标：

- 新增 ability registry。
- 新增 workflow registry。
- `create_feature`、`debug_case` 作为第一批内置 workflow。
- workflow 可以生成 Action Map BaseMap。

测试：

- workflow -> ability binding -> base map validation。
- missing ability blocked state。
- DAG dependency test。

### AW-3：Slash Entry 与机械状态输出

目标：

- `/create` 选择 `create_feature` workflow。
- `/debug` 选择 `debug_case` workflow。
- `/workflow` 可查看/切换机械状态。
- slash command 只输出状态、错误、路径和配置结果，不伪装 agent 回答。

测试：

- TUI slash command snapshot。
- protocol op handling。
- no hardcoded natural-language reply regression。

### AW-4：macOS App Workflow 实验

目标：

- 新增 `macos_app_development` workflow。
- 新增 Apple 工具链相关 ability manifest。
- 第一版只做设计、环境探测、脚手架、签名/公证检查，不强行覆盖完整 App Store 自动发布。

测试：

- 在无 Xcode / 无证书环境下能给出 blocked artifact。
- 在有 Xcode 环境下能完成 toolchain inventory。
- signing/notarization gate 能区分缺证书、缺 entitlements、缺 notary 配置。

---

## 十一、关键决策

1. 当前仍是设计早期，第一版只做最小 runtime contract，不做完整 workflow 平台。
2. `ability` 是产品内置原子能力，不是某个 workflow 的私有步骤；ability 可以扩展和替换。
3. `workflow` 是 runtime 级别能力，不是 skills、提示词或泛泛规范。
4. `workflow` 由 ability 组织起来，可以自定义、自由编排，并生成 Action Map。
5. `Create` / `Debug` 是第一批 workflow，不是底层 agent loop 分支。
6. `PrimitiveModule` 保留为实现层，承载 ability 的 schema、gate、hook、reducer。
7. macOS app development 是 workflow；Apple signing、toolchain probe、notarization check 是 ability。
8. 所有关键状态都必须 event-sourced、schema 化、replayable、Viewer-visible。
9. 任何自然语言任务仍进入 Agent/Model 路径，workflow 只改变约束、基建和验证，不生成固定智能回复。
