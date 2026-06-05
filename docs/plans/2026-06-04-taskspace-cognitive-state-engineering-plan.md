# TaskSpace 问题状态管理工程化落地方案

日期：2026-06-04

## 目标

本方案把 [TaskSpace E3 负收益后问题状态与模型管理重构基线](./2026-06-04-taskspace-cognitive-state-runtime-after-e3.md) 落到工程实施路径。

核心目标不是继续扩展 map/node 形式，而是把 TaskSpace 从“行动绑定和观测图”升级为“问题状态管理 runtime”：

```text
主 agent 管理问题状态
subagent 生产证据包
runtime 管结构协议、trace、sentinel、lease、snapshot；promotion/collapse 属于 v1.1
viewer/audit 在 MVP 观察契约、provenance 和结果采信；v1.1 再扩展到完整事实、假设、决策和开放问题
```

第一阶段必须直接回应 E3 暴露的三类失败：

| 失败 | 机制缺口 | 工程响应 |
|---|---|---|
| `hello-world` BOM / `heterogeneous-dates` UTF-16 | 输出契约缺失 | Output Contract Sentinel + `output_contracts` |
| `jsonl-aggregator` 自造数据自证 | provenance 缺失、假设污染 facts | Data Provenance Sentinel + facts/assumptions 分层 |
| map/node 过度生长 | node 没有状态转换目标，result 只是 summary | `state_delta_intent` 先作为 report-only 观察指标，MVP 主要依赖 claims/evidence/validity 收敛 result |

## 当前真实代码落点

本方案以现有机制改造为主，不新造平行 runtime。

| 能力 | 当前位置 | 当前状态 | 改造方向 |
|---|---|---|---|
| 数据模型 | `third_party/codex-cli/codex-rs/core/src/action_map/map.rs` | `TaskState` 只有 title/objective/status/map_ids；`MapNode` 只有 title/kind/status/context/result refs；`NodeResult` 只有 body summary | 扩展 task cognitive state、node state delta、result evidence package |
| runtime gate / lease / result | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` | 强制 active task/map/node/lease；按 `NodeKind` 校验 action class；维护 barrier；记录 tool result 和 lifecycle result | 加 direct trace、sentinel、result validity guard、问题状态更新；promotion/barrier hard gate 延后到 v1.1 |
| action class contract | `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs` | 当前 action-class gate 的实际规则来源 | cognitive state 不能绕开现有 action-class contract，新增 state delta 只做问题状态目标 |
| 控制工具 schema | `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs` | `taskspace_control` 支持 `start_task/route_task/create_node/bind_node/finish_node/block_node`；字段主要是 title/context/result summary | 扩展字段，保持向后兼容；首轮引入 `mark_result_validity` 等 MVP action，`promote_taskspace` 延后到 v1.1 |
| 控制 handler | `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs` | 解析自由文本字段并调用 session/runtime | 解析新字段，传入 runtime；旧调用路径继续可用 |
| prompt / BaseMap | `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs` | candidate node + decomposition methodology，仍主要是 prompt | 改为问题状态管理纪律和 evidence package 输出协议 |
| protocol/snapshot source | `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`、`third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/common.rs` 与 generated TS/JSON schema | `ActionMapSnapshot*` 展示 task/map/node/result/lease；当前 generated schema 可能已存在 `tool_success` 漂移 | 先修复 schema freshness，再增量添加 cognitive state、result evidence、sentinel warning 状态和 `promotion_not_in_mvp` report marker |
| viewer | `third_party/codex-cli/codex-rs/app-server/src/codex_message_processor.rs` 与 Web viewer | `/task-show` 读取 `thread_taskspace_read` snapshot | MVP 展示 output contracts、fact sources、claims/evidence/validity、sentinel warning；v1.1 再展示完整 facts/assumptions/obligations/decisions/open_questions 与 promotion/collapse/barrier 生命周期 |
| E2/E3 | `scripts/run-action-map-*.ps1`、`benchmarks/taskspace` | 已有 graph health、natural-user、external benchmark、audit 基础 | 增加认知收益指标、sentinel 触发证据、wrong-premise containment 检查 |

## 设计原则

1. 复用现有 ActionMap/TaskSpace runtime，不新增第二套 runtime。
2. runtime 不做语义选择，不判断事实真假，不做质量分。
3. runtime 负责结构协议、trace、sentinel、lease、snapshot、audit 可见性；promotion/collapse 是 v1.1 成本层级能力。
4. 主 agent 负责语义路由、问题状态更新、结果采信、任务决策。
5. 第一阶段优先修复 E3 失败闭环，不追求完整知识图谱。
6. 用户视角可以是 TaskSpace enabled，但内部不是重型 planning always-on。
7. 所有新增能力必须进入测试和观测，不只进入 prompt。

## 对抗审查后的工程收敛

三路对抗审查指出原方案仍有五类 blocking 风险：

- `direct_trace` / `LightKernelState` 可能和 `TaskState.cognitive_state` 形成第二套权威状态。
- sentinel 如果靠解析 shell 命令、preview 或自然语言 result，会把 runtime 变成隐藏语义判断器。
- result validity 如果没有 result/evidence 引用模型，就无法执行“questioned/invalid 不得作为下游依据”。
- protocol/generated TS/JSON schema 已可能漂移，新增字段前必须先修复 schema freshness。
- 首轮字段和 action 太大，容易变成机械填字段。

本方案因此收敛为一个更窄的 MVP：

```text
Phase A: protocol/schema freshness
Phase B: append-only trace event，不作为权威状态
Phase C: output contract + data provenance + result evidence package
Phase D: viewer/audit/benchmark 能回答 why accepted/questioned/invalid
```

首轮不实现完整 facts/assumptions/decisions/open_questions 图谱，不做完整 promote/collapse 状态机，不把所有字段一次性塞进 `taskspace_control`。首轮只做 E3 失败闭环所需的最小问题状态：输出契约、数据 provenance、result claims/evidence/validity。

## 最小化预实验后的计划修正

预实验见：

- 测试提交：`5a9049a9d test: add taskspace cognitive preflight checks`
- 审查记录：`vs_review/2026-06-04-taskspace-cognitive-preflight-tests-review.md`

这轮预实验的价值不是证明 cognitive runtime 已经可用，而是在正式开工前确认现有基建和方案边界。结论如下：

| 暴露点 | 现象 | 对工程计划的修正 |
|---|---|---|
| contract-sketch 测试容易制造假信心 | 自包含 helper 能验证规则自洽，但不能证明生产 runtime、tool schema、snapshot、viewer 已支持这些规则 | 所有这类测试必须命名为 `contract_sketch` / `audit_contract`，只能作为设计契约检查；不得计入生产覆盖率和 E3 收益证据 |
| 当前 `ActionMapSnapshotResult` 有可复用 join key | 真实结果 snapshot 已能提供 `assignmentId/mapId/nodeId/toolSuccess` 等可审计连接点 | 正式实现优先复用现有 result/snapshot join key，不新增平行 result index；新增字段必须挂到现有 snapshot/result 链路 |
| 当前 `taskspace_control` 还没有 cognitive MVP 协议 | 真实 tool schema 仍只暴露 `start_task/route_task/create_node/bind_node/finish_node/block_node` | Phase 0.1 固定当前 tool schema gap；Phase 4 新增 action/field 后必须同时补正反向 schema 测试 |
| future cognitive snapshot restore 尚无法验证 | cognitive 字段尚未存在，预实验只能确认当前 join key 和 tool schema gap | Phase 3 不允许只加字段；必须同步提供 legacy snapshot restore、default、`cognitive_schema_version`、viewer 缺字段空态测试 |
| output contract 是真实缺口 | 审查要求补上“缺 output contract 必须失败”的负例 | Output contract 从“建议记录”升级为 MVP hard gate：没有 output contract 的 final artifact audit 必须失败 |
| 审计必须可机械 join | 仅把信息写进 result body 无法让 audit 判断 accepted/questioned/invalid 的来源 | 每个新增事件、contract、fact source、claim、validity transition 都必须有稳定 ID，并能 join 到 task/map/node/result/final artifact |

由此，正式实现前新增一个“Phase 0.1：预实验契约落账”：

```text
Phase 0.1: 将 contract-sketch 测试、真实 taskspace_control tool schema gap 测试、真实 snapshot join-key 测试固定为开工护栏
```

Phase 0.1 的通过标准：

- `cognitive_preflight_contract_sketch_audit_*` 只能描述 MVP hard gate，不得引用生产未实现字段来伪造通过。
- `ActionMapSnapshotResult` 序列化测试必须覆盖 `assignmentId/mapId/nodeId/toolSuccess`，并防止 snake_case 泄漏到 JSON。
- `taskspace_control` 当前协议测试必须明确 cognitive MVP 字段缺失；正式加字段后，这个测试要改成“新字段存在且旧 action 兼容”。
- Phase 0.5 只负责 generated JSON/TypeScript/Rust schema freshness，不再混入 tool schema gap 归属。
- 审查报告必须明确区分“预实验通过”和“生产 runtime 未实现”，禁止把预实验结果写成 E2/E3 utility 结论。
- `git diff --check` 和相关 targeted cargo tests 必须进入实现前后的固定回归命令。

### 单一权威状态原则

`direct_trace` 只能是 append-only observation log，不能持有 objective、success criteria、facts 或 open questions 这类权威问题状态。

权威状态只允许存在于：

```text
TaskState.cognitive_state
```

promotion 的含义是：

```text
direct_trace events -> agent 生成 promotion payload -> materialize into TaskState.cognitive_state
```

collapse 的含义是：

```text
TaskState.cognitive_state 保留审计摘要；后续执行成本层级降低，但权威状态不迁回 trace。
```

### Sentinel 输入契约

runtime 不解析 shell preview/body、不从自然语言 result 推断语义。现有 tool/session 层可以继续用 `classify_shell_text` 把命令分类为 `ActionClass::Read/Edit/Test/Build` 等机械执行类别；TaskSpace runtime 只能消费这个已经结构化的 `action_class` 和 `tool_success`，不得再从原始命令、preview 或 result body 推断输出契约、数据来源、事实真假或最终产物语义。

sentinel 只能读取结构化事件：

```rust
pub(crate) struct TaskSpaceTraceEvent {
    pub(crate) id: String,
    pub(crate) kind: TraceEventKind,
    pub(crate) action_class: Option<ActionClass>,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) map_id: Option<ActionMapId>,
    pub(crate) node_id: Option<MapNodeId>,
    pub(crate) result_id: Option<NodeResultId>,
    pub(crate) call_id: Option<String>,
    pub(crate) tags: Vec<TraceEventTag>,
    pub(crate) artifact_refs: Vec<String>,
    pub(crate) created_at_ms: i64,
}
```

`TraceEventTag` 第一版只允许来自两类来源：

- tool/session 层能机械识别的结构化事实，例如 tool kind、action class、tool success、validator exit、known file write API。
- agent 通过 `taskspace_control` 显式记录的事实，例如 `record_output_contract`、`record_fact_source`、`mark_result_validity`。

如果事件来自普通 shell 且无法结构化识别，runtime 只能标记 `UnclassifiedShellAction`，不得靠字符串猜测它是否“写最终输出”或“生成测试数据”。

Phase 2 sentinel 可以把 `ActionClass::Test` + `tool_success=false` 当作 validator failure warning 输入；但不能因为命令文本里出现 `python`、`cat`、`echo`、`pytest` 等词，就推断文件编码、数据 provenance、final artifact 或业务事实。

### Evidence 引用原则

第一版先引入 `EvidenceRef`，不急于给所有 edge 增加复杂 result dependency。

```rust
pub(crate) struct EvidenceRef {
    pub(crate) result_id: Option<NodeResultId>,
    pub(crate) claim_id: Option<String>,
    pub(crate) fact_source_id: Option<String>,
    pub(crate) trace_event_id: Option<String>,
    pub(crate) artifact_ref: Option<String>,
    pub(crate) validator_ref: Option<String>,
}
```

runtime 首轮 hard gate 只做可机械判断的约束：

- `Accepted` result 必须有 `claims` 和 `evidence_refs`。
- `TaskCognitiveState.facts` 只能引用 `Accepted` result 或明确 `ObservedFromEnvironment` / `ProvidedByUser` fact source。
- `GeneratedForTestOnly` 和 `Unknown` provenance 不得写入 active facts。
- `Questioned` / `Invalid` result 不得作为 `update_cognitive_state` 的 evidence source。
- `Questioned` / `Invalid` result 不得进入 final artifact 的依赖链；如果最终产物只能追溯到这类 result，本轮 audit 必须 hard fail。

“implementation node 是否唯一依赖 questioned result”首轮只对 final artifact dependency 做 hard gate：只要该 questioned/invalid result 被声明为最终产物依据，就必须失败；普通中间实现节点的依赖关系先保留为 audit 指标，等 result dependency 模型稳定后再扩大到所有下游节点。

### Versioned Snapshot Schema

新增字段前必须先补上 schema freshness gate：

- Rust protocol source。
- generated JSON schema。
- generated TypeScript schema。
- viewer/app-server consuming type。

必须能检测当前这类漂移：Rust snapshot result 有 `tool_success`，但 generated TS/JSON schema 缺 `toolSuccess`。

预实验已经确认真实 `ActionMapSnapshotResult` 可以作为 audit join key 来源，因此新增 cognitive 字段时必须沿着现有 snapshot/result 链路扩展，不允许另建只给 audit 使用的影子结果结构。新增字段后，至少要有三类测试同时通过：

- 当前 snapshot result 仍能序列化出 `assignmentId/mapId/nodeId/toolSuccess`。
- legacy snapshot 缺 cognitive 字段时 restore 不失败，并给出空 cognitive state/default。
- 新 snapshot 带 `cognitive_schema_version`，viewer/app-server 能读取但不会因未知字段或缺字段崩溃。

新增 cognitive snapshot 必须带版本号：

```rust
pub(crate) struct ActionMapSnapshot {
    pub(crate) cognitive_schema_version: Option<String>,
    ...
}
```

第一版版本：`taskspace-cognitive-v1`。

## 目标数据模型

### TaskCognitiveState

新增到 `TaskState` 或作为 `TaskState.cognitive_state`：

```rust
pub(crate) struct TaskCognitiveState {
    pub(crate) success_criteria: Vec<String>,
    pub(crate) fact_sources: Vec<FactSource>,
    pub(crate) output_contracts: Vec<OutputContract>,
    pub(crate) facts: Vec<CognitiveClaim>,
    pub(crate) assumptions: Vec<CognitiveClaim>,
    pub(crate) decisions: Vec<DecisionRecord>,
    pub(crate) open_questions: Vec<OpenQuestion>,
    pub(crate) risk_notes: Vec<String>,
}
```

第一版字段都可以是自然语言，不引入复杂 schema。关键是显式分层，不能把 assumption 直接写入 fact。

### FactSource

```rust
pub(crate) struct FactSource {
    pub(crate) id: String,
    pub(crate) provenance: DataProvenance,
    pub(crate) description: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

pub(crate) enum DataProvenance {
    ObservedFromEnvironment,
    ProvidedByUser,
    GeneratedForTestOnly,
    Inferred,
    Unknown,
}
```

硬规则：

```text
GeneratedForTestOnly 不得作为 final output 的事实依据。
Unknown 不得直接进入 accepted facts。
```

### OutputContract

```rust
pub(crate) struct OutputContract {
    pub(crate) id: String,
    pub(crate) kind: OutputContractKind,
    pub(crate) description: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

pub(crate) enum OutputContractKind {
    Artifact,
    Format,
    Encoding,
    Schema,
    Validator,
    NonGoal,
}
```

第一版不需要把 encoding/schema 完全解析为强类型。只要能在 task/node/result/viewer/audit 中明确出现，就能防止当前 E3 的主要失败。

### NodeStateDelta

扩展 `MapNode.context` 或新增字段：

```rust
pub(crate) struct NodeStateDelta {
    pub(crate) intent: StateDeltaIntent,
    pub(crate) scope: String,
    pub(crate) excluded_scope: String,
    pub(crate) expected_result: String,
    pub(crate) evidence_required: Vec<String>,
    pub(crate) acceptance_hint: String,
    pub(crate) why_now: String,
    pub(crate) stop_condition: String,
}

pub(crate) enum StateDeltaIntent {
    EstablishFact,
    TestAssumption,
    SatisfyObligation,
    ProduceArtifact,
    ValidateArtifact,
    ResolveOpenQuestion,
    CompareOptions,
    ContainRisk,
    SynthesizeDecision,
}
```

这不替代现有 `NodeKind`。`NodeKind` 继续用于 action class gate；`StateDeltaIntent` 用于认知目标和 audit。

### NodeResultEvidencePackage

扩展 `NodeResult`，保留 `body` 作为兼容 summary：

```rust
pub(crate) struct NodeResultEvidencePackage {
    pub(crate) claims: Vec<CognitiveClaim>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
    pub(crate) changed_artifacts: Vec<String>,
    pub(crate) validation: Vec<ValidationRecord>,
    pub(crate) remaining_uncertainty: Vec<String>,
    pub(crate) recommended_state_updates: Vec<RecommendedStateUpdate>,
    pub(crate) validity: ResultValidity,
    pub(crate) validity_reason: String,
}
```

```rust
pub(crate) enum ResultValidity {
    PendingReview,
    Accepted,
    Questioned,
    Superseded,
    Invalid,
}
```

硬规则：

- 新的 node lifecycle result 默认 `PendingReview`。
- `Accepted` 必须包含 claims 和 evidence refs。
- `Questioned` result 不得作为 `update_cognitive_state` 的 evidence source；如果它进入 final artifact dependency，第一版必须 hard fail；普通中间 implementation 唯一依赖规则第一版只做 audit，不做 hard gate。
- `Invalid` result 保留历史，但不得进入 active facts。
- `Superseded` 必须指向替代 result 或 node。

## 首轮 MVP 闭环

首轮实现只覆盖三个闭环：

| 闭环 | 必须实现 | 暂不实现 |
|---|---|---|
| Output contract | 记录 output contract、写入 snapshot、viewer/audit 可见、E3 能检出是否存在 | 自动理解所有 shell 写文件语义 |
| Data provenance | 记录 fact source provenance、禁止 generated/unknown 写入 active facts、audit 可追踪 | 自动判断所有数据来源真假 |
| Result evidence | result 带 claims/evidence_refs/validity，accepted 必须有证据，validity transition 有事件 | 完整 result dependency graph 和自动质量评分 |

首轮完成后，只能声明：

```text
TaskSpace cognitive-state MVP 能阻止 E3 中已观察到的同形失败。
```

不能声明：

```text
TaskSpace 已证明在外部 benchmark 上有总体产品收益。
```

## MVP Pass / Fail 契约

第一版必须把“机制生效”定义成可复盘的 gate，而不是人工读报告后的主观判断。每次 E2/E3 运行必须生成 run-level gate records。

MVP hard fail gate：

| Gate | 失败条件 | 说明 |
|---|---|---|
| `schema_freshness_pass` | `false` | Rust protocol、JSON schema、TS schema、viewer consuming type 任一不一致即失败。 |
| `required_output_contract_missing` | `> 0` | 需要最终产物或 validator 的任务没有记录 output contract。 |
| `source_provenance_missing` | `> 0` | final facts / final artifact dependency 缺少事实来源。 |
| `accepted_result_missing_evidence` | `> 0` | `Accepted` result 没有 claims 或 evidence refs。 |
| `generated_or_unknown_provenance_in_active_fact` | `> 0` | `GeneratedForTestOnly` / `Unknown` 进入 active facts。 |
| `self_generated_data_leakage` | `> 0` | 自造测试数据污染最终判断或最终产物依据。 |
| `questioned_or_invalid_result_in_cognitive_state_update` | `> 0` | `Questioned` / `Invalid` result 被写入权威 cognitive state。 |
| `questioned_or_invalid_final_artifact_dependency` | `> 0` | 最终产物依赖链包含 `Questioned` / `Invalid` result。 |
| `sentinel_warning_uncleared_for_final_artifact` | `> 0` | 与最终产物有关的 sentinel warning 到 run 结束仍未被清除、接受风险或改写契约。 |
| `audit_why_chain_missing` | `> 0` | 任一最终产物无法追溯到 result / claim / evidence / validator / fact source。 |

MVP warning 或 report-only 指标：

| 指标 | 第一版处理 |
|---|---|
| `sentinel_warning_triggered` | 记录并展示，不直接失败；只有未清除且影响最终产物时失败。 |
| `promotion_trigger` | report-only，MVP 报告必须显式写 `promotion_not_in_mvp=true`。 |
| `promotion_latency` | report-only，MVP 不用它判断成功失败。 |
| `collapse_rate` | report-only，MVP 不实现 collapse 判定。 |
| `state_delta_intent_present` | report-only，用于观察 map 质量，不做 MVP hard gate。 |
| `new_fact_per_node` | report-only，避免把 node 机械逼成产出事实。 |
| `main_direct_tool_ratio` | report-only，用于观察主 agent 是否退回线性执行。 |

promotion/collapse 不属于首轮 MVP 实现范围。它们不是“已修复”的能力，而是显式延期到 v1.1：MVP 报告必须带 `promotion_not_in_mvp=true`，Phase 8 开始前必须先补 promotion replay test，证明 trace refs 能被继承到 promotion payload 和初始 cognitive state。

## 分阶段实施

### Phase 0：实现边界冻结、schema freshness 与回归基线

目标：锁定当前行为，防止重构过程中不知道哪里坏了。

改动：

- 新增 `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`。
- 新增或更新 `/vs_review/` 评审报告。
- 明确当前代码落点和旧 schema 边界。
- 修复并固化 protocol generated schema freshness 检查。
- 把 `tool_success` 这类现有 Rust/TS/JSON schema 漂移作为第一条验证样本。

验证：

- 文档链接检查。
- `git grep` 确认旧 Planner 文档路径无残留。
- schema generation/freshness check 能发现 Rust protocol 与 generated TS/JSON 不一致。
- 不跑代码功能测试，因为本阶段只写设计文档和审查报告。

退出条件：

- 工程方案通过对抗性审查，且无未处理 blocking finding。

### Phase 1：Append-only Direct Trace

目标：在 TaskSpace enabled 情况下先低成本记录 direct path，但 trace 不是权威问题状态。

涉及文件：

- `core/src/action_map/runtime.rs`
- `core/src/session/handlers.rs`
- 可能涉及 rollout reconstruction / snapshot 恢复路径

实现：

1. 在 runtime state 增加 append-only `taskspace_trace_events`。
2. 在 `prepare_main_tool_call` / `record_main_tool_result` 记录结构化 trace event：
   - tool call count
   - action class
   - tool success
   - known structured file/artifact refs when available
   - validator/test failure when available from structured execution result
   - explicit tags produced by `taskspace_control`
3. snapshot 暴露 trace summary 和 trace event refs，而不是把所有 raw tool 输出塞进 viewer。

注意：

- 不修改标准模式。
- 不改变现有 `taskspace_control` 行为。
- trace 只做观测和 promotion 输入，不阻塞普通执行。
- trace 不持有 objective/facts/open questions；这些只属于 `TaskState.cognitive_state`。
- 无法结构化识别的 shell 行为只能标记为 `UnclassifiedShellAction`。

测试：

- 单元测试：standard mode 不记录 TaskSpace trace。
- 单元测试：experiment mode 下 read/edit/test action trace event 正确。
- 单元测试：shell preview 不被字符串解析成 output/data/provenance 语义。
- 回归测试：现有 `run-action-map-regression.ps1` 通过，并默认包含 TaskSpace trace session-path tests。
- E2 smoke：`single-file-fast-fix` 不应因为 trace 失败。

实施记录（2026-06-04 Phase 1）：

- 已实现 `taskspace_trace_events` append-only runtime 状态。
- 已在 `record_main_tool_result_with_class` 的成功落库路径记录 `main_tool_result` trace event；标准模式不记录。
- snapshot 已暴露 `trace_summary` 与 `trace_events` 引用，不暴露 raw preview/body。
- runtime 已发出 `taskspace_trace_event_recorded` 事件；事件顺序为先记录 `node_result_recorded`，再记录 trace event，随后才可能触发 maintenance barrier。
- restore snapshot 会恢复 trace refs 并续接 `trace-*` 序号；未知 tag 会被过滤到允许集合，避免旧/坏 snapshot 注入未来 sentinel 语义。
- session 生产路径已覆盖：`prepare_action_map_main_tool_call` + `record_action_map_main_tool_result` 会触发 live trace event；standard mode 同路径不记录 trace。
- 当前 `taskspace_control` 没有显式 tag 输入面；Phase 1 只记录工具结果可机械判定的 tags（tool success/failure、validator success/failure、unclassified shell action）。`taskspace_control` 显式 tag 属于 Phase 2 sentinel/tag 输入设计，不通过字符串解析补齐。
- 回归证据：`scripts/run-action-map-regression.ps1` 通过，报告 `target/test-reports/action-map-20260604-184129-465/report.md`；默认矩阵已包含 `core-taskspace-trace` 与 `core-session-standard-trace`，避免 session-path tests 只存在于手工 targeted 命令。
- 真实安装版 smoke：`single-file-fast-fix` paired run 通过，报告 `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260604-183001-885\pair-001\pair-report.md`；TaskSpace 侧 1 map / 4 nodes / 3 edges / 0 edge order violations，rollout 中可见 `taskspace_trace_event_recorded` 与 `traceSummary.totalEventCount=8`。

### Phase 2：MVP Sentinel Warning

目标：对 E3 暴露的真实风险做低成本 warning。第一版只读取结构化 trace event 和显式 taskspace_control 记录，不做字符串解析。

涉及文件：

- `runtime.rs`
- tool action classifier 相关代码
- `taskspace_control` tool description
- E2/E3 audit scripts

#### Output Contract Sentinel

触发：

- `TaskState.cognitive_state.output_contracts` 缺失，且出现结构化 `OutputArtifactRequired` 或 `ValidatorContractKnown` trace tag。
- agent 显式声明要产出最终 artifact，但未记录 output contract。

行为：

- 若当前 task 没有相关 `output_contracts`，先记录 `output_contract_required` sentinel warning。
- hard barrier 延后到 warning 路径稳定后再启用。
- warning 不要求 agent 建更细 node，而要求显式记录 output contract。
- 允许主 agent 通过 `taskspace_control` 写入 contract 后继续。

#### Data Provenance Sentinel

触发：

- 出现结构化 `DataGeneratedForTest`、`InputSourceUnknown` 或 `DataTransformed` trace tag。
- agent 显式记录 fact source provenance 为 `GeneratedForTestOnly` 或 `Unknown`。

行为：

- generated data 必须标记 `GeneratedForTestOnly`。
- `GeneratedForTestOnly` 不得进入 active facts。
- 若 final artifact 依赖 unknown/generated provenance，先记录 `provenance_risk` sentinel warning。
- hard barrier 延后到 v1.1。

#### Failed Hypothesis Sentinel

触发：

- 出现结构化 `ValidationFailedAfterPatch`、`ValidatorMismatch`、`RepeatedFailedAction` trace tag。

行为：

- 要求记录 failed hypothesis、evidence、next open question。
- 如果 agent 连续尝试同类 patch/test 而不更新假设，先记录 `failed_hypothesis_loop` warning。
- hard barrier 延后到 v1.1。

测试：

- 新增 unit tests 覆盖三类 warning。
- 证明 warning 来自结构化 event tag，不来自命令字符串解析。
- `hello-world` fixture 验证写文件前能出现 output contract。
- `jsonl-aggregator` 变体验证 generated data 不得进入 accepted facts。

实施记录（2026-06-04 Phase 2A）：

- 已实现 trace-driven sentinel warning 子集：`validator_failure` 与 `unclassified_shell_action`。
- warning 由 `TaskSpaceTraceEvent.tags` 触发，runtime 不读取 shell preview/body、不读取自然语言 result，也不从原始命令文本推断 output contract、provenance、final artifact 或业务事实。
- warning 当前只做观测：写入 snapshot `sentinel_summary` / `sentinel_warnings`，并发出 `sentinel_warning_raised` runtime event；不阻塞工具调用，不改变 node 状态，不创建 maintenance barrier。
- session 生产路径已覆盖：validator failure 的 main tool result 会在同一事件流中发出 `taskspace_trace_event_recorded` 和 `sentinel_warning_raised`，snapshot 同步带 warning。
- protocol/generated schema 已暴露 `ActionMapSnapshotSentinelSummary` 与 `ActionMapSnapshotSentinelWarningRef`，legacy snapshot 缺字段时默认为空。
- 当时尚未实现 output contract sentinel、data provenance sentinel、clear action、hard barrier、E2/E3 audit hard gate；这些需要 Phase 3/4 的 cognitive state/result evidence 数据模型和 control actions 之后才能闭环。后续 Phase 7 已补齐 MVP final-artifact audit hard gate，Phase 7D 补齐 `sentinel_warning_cleared` 事件进入报告/audit 的闭环；runtime 主动清除命令和 hard barrier 仍不在 MVP 内。

实施记录（2026-06-04 Phase 3A）：

- 已实现最小 cognitive 数据模型：`TaskState.cognitive_state` 承载 `success_criteria`、`fact_sources`、`output_contracts`、`facts`、`assumptions`、`risk_notes`；`NodeResult.evidence_package` 承载 `claims`、`evidence_refs`、`changed_artifacts`、`validator_refs`、`remaining_uncertainty`、`validity`、`validity_reason`。
- snapshot 已带 `cognitive_schema_version = taskspace-cognitive-v1`，并沿现有 task/result 链路输出 `cognitiveState` 与 `evidencePackage`，没有新增影子 result index。
- legacy snapshot 缺 cognitive 字段时默认空状态；未知 `cognitive_schema_version` 的 payload 不被旧 runtime 采信，restore 后降级为空 cognitive state / `unreviewed` result evidence。
- protocol 入口已对 cognitive/evidence 容器做宽容反序列化：缺字段、`cognitiveState: {}`、`evidencePackage: {}`、`null`、非对象容器、局部缺字段、未知未来版本中的未来形状或错型字段，都不能在 JSON 读取阶段打断 snapshot restore；runtime 只在 `cognitive_schema_version == taskspace-cognitive-v1` 时采信当前结构。
- protocol/generated JSON/TypeScript schema 已暴露 cognitive state、evidence refs、output contracts、fact sources、result evidence package，并有 schema fixture freshness 测试。
- Phase 4A 已实现 `taskspace_control` 的 `record_output_contract`、`record_fact_source`、`record_fact`、`mark_result_validity` 生产路径；尚未实现 viewer cognitive side panel、final artifact audit hard gate、sentinel clear action；这些继续进入 Phase 5/6/7。

### Phase 3：MVP 数据模型与 Snapshot

目标：把最小问题状态和证据包进入数据模型、snapshot 和 generated schema。

涉及文件：

- `core/src/action_map/map.rs`
- `core/src/action_map/runtime.rs`
- `core/src/action_map/contracts.rs`
- `tools/src/taskspace_tool.rs`
- `core/src/tools/handlers/taskspace_control.rs`
- `app-server-protocol/src/protocol/common.rs`
- generated schema JSON/TS

实现顺序：

1. 先修复 schema generation/freshness。
2. 数据模型只增加 MVP 字段：
   - `TaskCognitiveState.output_contracts`
   - `TaskCognitiveState.fact_sources`
   - `TaskCognitiveState.facts`
   - `NodeResultEvidencePackage.claims`
   - `NodeResultEvidencePackage.evidence_refs`
   - `NodeResultEvidencePackage.validity`
   - `NodeResultEvidencePackage.validity_reason`
   - `TaskSpaceTraceEvent`
   - `SentinelRecord`
3. snapshot 暴露 MVP 字段并带 `cognitive_schema_version = taskspace-cognitive-v1`。
4. handler 解析 MVP 字段，旧字段继续进入 summary。
5. restore snapshot 保持兼容：缺字段时使用默认空值。
6. full facts/assumptions/decisions/open_questions、完整 `NodeStateDelta` 延后到 v1.1。

测试：

- snapshot restore 兼容旧 snapshot。
- taskspace_control 新旧参数都能工作。
- invalid `validity` 返回明确错误；`state_delta_intent` 仅作为 report-only / v1.1 测试债务，不进入 MVP parser/schema/action hard requirement。
- `Accepted` result 缺 claims/evidence_refs 时被拒绝或降级为 `PendingReview`。
- Rust protocol、JSON schema、TS schema freshness test 通过。

### Phase 4：MVP 控制动作

目标：主 agent 可以显式记录 output contract、fact source、result validity 和最小 cognitive state，而不是把所有内容塞进 result summary。

首轮控制动作：

- `record_output_contract`
- `record_fact_source`
- `record_fact`
- `mark_result_validity`

延后控制动作：

- `promote_taskspace`
- `update_cognitive_state` 完整版
- `record_decision`
- `record_open_question`

取舍：

- 第一版可以继续复用 `taskspace_control`，但只新增 MVP action，避免一次塞入大量 optional fields。
- 如果 MVP action 已让 schema 过大，再拆出 `taskspace_state` 工具，而不是继续扩 `taskspace_control`。

runtime 校验：

- `record_fact_source` 必须带 provenance。
- `mark_result_validity=accepted` 必须引用 claims/evidence。
- `record_fact` 不能把 `GeneratedForTestOnly` 或 `Unknown` provenance 写入 active facts。
- `mark_result_validity` 必须产生 `result_validity_changed` event。

测试：

- action parser tests。
- runtime state update tests。
- invalid transition tests。

实施记录（2026-06-04 Phase 4A）：

- 已继续复用现有 `taskspace_control`，未新增第二套 state 工具；新增 action 只限 MVP 范围：`record_output_contract`、`record_fact_source`、`record_fact`、`mark_result_validity`。
- session 层只新增薄 wrapper，最终仍调用 `ActionMapRuntime` 并复用同一套 `emit_action_map_events_for_turn` 事件流。
- runtime 只做机械可判定约束：必须处于 TaskSpace experiment mode；必须有当前 session 持有的 active task/map；ID、描述、理由不能为空；新增 record 类 action 必须带 evidence refs；空 evidence ref 被拒绝。
- evidence ref 现在可引用 `result_id`、`claim_id`、`fact_source_id`、`trace_event_id`、`artifact_ref`、`validator_ref`。其中 `result_id` 必须能 join 到 active map 的真实 result，`fact_source_id` 必须能 join 到 active task 的 fact source，`trace_event_id` 必须能 join 到已记录 trace。
- `mark_result_validity=accepted` 必须同时提供 top-level evidence refs 和 claims，每个 claim 也必须有 evidence refs；接受结果时写入 `NodeResult.evidence_package`，不再只依赖自然语言 result body。
- `record_fact` 只能引用已 accepted result，或引用 provenance 为 `observed_from_environment` / `provided_by_user` 的 fact source；`generated_for_test_only`、`inferred`、`unknown` provenance 以及 `unreviewed/questioned/invalid` result 都不得被提升为 active facts。
- 新增 runtime event 只暴露最小引用：`cognitive_state_updated` 带 `task_id/map_id/update_kind/record_id`；`result_validity_changed` 带 `task_id/map_id/node_id/result_id/validity`。事件不携带完整 description、claims、evidence refs、validity reason，权威状态仍在 snapshot 的 `TaskState.cognitive_state` 和 `NodeResult.evidence_package`。
- protocol snapshot 与 app-server generated schema 已同步 `EvidenceRef.factSourceId`；`MapRuntimeEvent` 当前不属于 app-server generated TypeScript schema 导出面，由 protocol 单测守住 minimal-ref 事件序列化。
- Phase 4A 当时延后项：viewer cognitive side panel、final artifact audit hard gate、sentinel clear action、prompt/developer context 注入、promotion/collapse 仍不属于 Phase 4A 完成范围。后续 Phase 6 已补齐 viewer cognitive side panel，Phase 7 已补齐 MVP final artifact audit hard gate 与 sentinel clear event 的报告/audit 消费；runtime 主动 clear command、sentinel hard barrier、promotion/collapse 仍为后续。
- 验证结果：`scripts/run-action-map-regression.ps1` 通过，报告 `target/test-reports/action-map-20260604-223439-306/report.md` 显示 10 个 cargo run、3 个脚本 run 全部 PASS，199 passed、0 failed、0 relevant crash events。

### Phase 5：Prompt / Developer Context 改造

目标：让模型知道 MVP 新协议，并避免继续把 TaskSpace 当行动日志。

涉及文件：

- `core/src/action_map/basemap.rs`
- session developer context 相关代码
- tool description

更新内容：

- 主 agent 是问题状态与模型管理者，不是线性 worker。
- 首轮必须显式记录 output contract 和 fact source provenance。
- subagent result 必须产出 claims/evidence refs/uncertainty。
- `Accepted` 必须有 claims/evidence refs。
- generated/test-only data 不得作为 final fact。
- direct trace 是内部审计日志，不暴露给用户。
- 不得对用户提 task/map/node/subagent 等内部概念。

测试：

- prompt snapshot tests。
- 自然用户 E2E prompt leak 检查继续保持。

实施记录（2026-06-04 Phase 5A）：

- 已在 `core/src/action_map/basemap.rs` 增加共享 `TaskSpace cognitive protocol (MVP)`，并复用到 BaseMap metadata。协议明确主 agent 是“问题状态与模型管理者”，不是线性 worker；它需要维护 task map、分配 bounded nodes、整合 evidence，并在行动前更新任务模型。
- 已在 `ActionMapRuntimeState::build_developer_context()` 的活动 task path 上注入同一协议；当没有 active map 时，通过 BaseMap metadata 暴露协议，避免新增并行 prompt 注入通道。
- 已在 developer context 中压缩展示当前 active task 的 `output_contracts`、`fact_sources`、`facts`，以及 active map 中已经写入 evidence package 的 result。展示内容只来自 `TaskState.cognitive_state` 与 `NodeResult.evidence_package`，不从 event body 或自然语言 result 中重新解析，保持唯一权威状态来源。
- result evidence package 在 developer context 中只展示摘要计数、validity、claims preview、summary 和 uncertainty preview；未写入 evidence package 的 result 会被提示为不可直接当作 accepted fact 使用。
- 已在 `taskspace_control` tool description 中补充认知状态使用纪律：用户要求先记录 output contract；用户/环境/validator 事实先记录 fact source；`generated_for_test_only`、`inferred`、`unknown` 不能锚定 active facts 或最终用户结论；subagent/node result 必须经 `mark_result_validity` 才能进入任务模型。
- 已把 subagent assignment 从“free-form result”调整为“result package”：要求返回 claims、evidence refs 或具体文件/命令/validator、changed artifacts、remaining uncertainty、blockers，并明确父 agent 才负责 review 和 `mark_result_validity`。
- 已收紧 `mark_result_validity` 的 evidence scope：给当前 result 标记 validity 时，`result_id` evidence 必须指向当前 result；`trace_event_id` 必须属于当前 task/map 且引用当前 result，防止复制旧 result/trace 把无关结果标成 accepted。
- 已更新 `format_action_map_snapshot()` 的 Results 区域，展示 `validity`、claims/evidence/validator counts；未 accepted result 显示 `trust=not_accepted_fact`，避免 `/task-show` 文本快照把 unreviewed result body 呈现成普通事实。
- 已在 snapshot restore 后校验 active task/map binding 的一致性：`active_task_id`、`active_map_id`、`task.active_map_id`、`task.map_ids`、`map.task_id` 任何不一致都会清除 active binding 并强制下一轮 routing，避免把错态 cognitive state 暴露给模型。
- 本阶段不新增 user-turn/message evidence ref schema、不新增 final artifact hard gate，也不实现 viewer cognitive side panel。后续 Phase 6/7 已分别补齐 viewer cognitive side panel 与最终产物 cognitive audit gate；`provided_by_user` 的可 join 消息证据仍不在 Phase 5A 偷偷用自由文本 `artifact_ref` 替代。
- 新增/更新测试覆盖 BaseMap prompt、TaskSpace developer context、tool schema description、已有 cognitive records 在 developer context 中的可见性、cross-result evidence contamination、restore mismatch repair、snapshot result trust marker、subagent assignment result package；继续断言 `promote_taskspace`、`promotion_not_in_mvp`、`collapsed-direct` 不进入 MVP 提示面。

### Phase 6：Viewer 与 Snapshot 展示

目标：用户和审计者能看懂 TaskSpace 是否真的管理问题状态。

涉及文件：

- app-server protocol
- app-server `thread_taskspace_read`
- Web viewer
- `scripts/export-action-map-observability.ps1`

展示内容：

- Task objective / success criteria。
- MVP cognitive side panel（只展示 MVP 字段，不是完整 cognitive graph 面板）：
  - output contracts。
  - fact sources + provenance。
  - result claims/evidence/validity。
  - sentinel warning records。
  - `promotion_not_in_mvp` 等 report-only 标记。
- promotion/collapse/barrier 不展示为 MVP 已闭环能力。
- 完整 facts/assumptions/decisions/open_questions 的可编辑/可审计面板不属于 MVP，延后到 v1.1。

UI 约束：

- 继续保持极简、极客风格。
- graph 仍展示 node/edge，但右侧/详情面板展示 cognitive state。
- 自动刷新不能破坏展开/选择状态。

测试：

- snapshot JSON schema test。
- viewer smoke test。
- Playwright 级 E2E：展开节点、选中文本、等待自动刷新后，断言展开和选择状态仍保留。

实施记录（2026-06-04 Phase 6A）：

- `/task-show` 本地 web viewer 已在现有 `thread/taskspace/read` 和 `snapshot.json` 轮询机制上扩展 cognitive side panel，没有新增第二套 viewer server。
- 展示面板读取 snapshot 中的 `task.cognitiveState`、`map.results[].evidencePackage`、`sentinelWarnings`，展示 task objective/status、success criteria、output contracts、fact sources、facts/assumptions、result validity、claims/evidence/validator 计数、sentinel warning。
- graph 仍由 map/node/edge 驱动，认知信息只放在右侧详情面板；结果详情展示 body 之前先展示 validity、claims、evidence refs、changed artifacts、validator refs、remaining uncertainty。
- 自动刷新继续保留当前 task/node/result 的展开与选择状态；已有 Rust viewer HTML smoke test 覆盖 cognitive 面板关键字符串。
- 当前 viewer 只做只读可观察性，不允许在页面中编辑 cognitive state，不承担 runtime gate。

实施记录（2026-06-05 Phase 6B）：

- `scripts/run-tui-taskspace-viewer-e2e.ps1` 已升级为真实浏览器交互 E2E：启动安装版 `whale.exe`，真实输入 `/taskspace`、`/task-reborn`、一个自然 coding request、`/task-show`，再打开 localhost viewer；用户 prompt 不出现完整 marker，避免把 prompt echo 误判成 assistant 完成。
- 浏览器层复用现有 viewer server 与 `snapshot.json` 轮询，不新增第二套 viewer/app server；脚本按需在 `target/pty-tools` 安装 `playwright-core`，并使用本机 Chrome/Edge executable path，不下载独立浏览器。
- E2E 现在验证三类过去真实暴露的问题：展开的 `details[data-key]` 在自动刷新后仍保持展开；选中 thread/meta 文本时自动刷新不会打断 selection；graph 经过缩放与拖拽后，`graph-world` transform 在刷新后保持。浏览器 probe 只在 active snapshot 已有 node/result 后启动，避免只验证空页面。
- 浏览器 probe 记录每次 `/snapshot.json` 响应的 status、时间戳、hash 与 map/node/edge/result 计数，并保存 `browser-snapshot-*.json`、`node-initial-empty-snapshot.json`、`node-active-snapshot.json` 和 `browser-summary.json`。这让刷新是否发生、刷新发生时 UI 状态是否保持、viewer 是否读到 task/map/result 生长可以被复核。
- `scripts/run-action-map-regression.ps1` 新增 `-IncludeTuiViewerE2E` 开关，可以把真实 TUI viewer E2E 纳入同一回归报告；默认回归仍不跑该长链路，避免把常规 unit/script 回归绑定到真实模型和本机浏览器。wrapper 同时汇总 viewer 的刷新次数、图交互、selection/details 保持、snapshot 计数和 console/network 计数，崩溃事件窗口覆盖到脚本测试结束。
- 验证证据：`scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E` 通过，报告 `target/test-reports/action-map-20260605-132009-632/report.md`；结果显示 10 个 cargo run、4 个脚本 run 全部 PASS，`total_passed_tests=218`、`total_failed_tests=0`、`relevant_crash_events=0`。viewer 子报告 `target/test-reports/action-map-20260605-132009-632/run-tui-taskspace-viewer-e2e.ps1/artifacts/report.md` 显示 `browser_interaction_ok=true`、`browser_refresh_count=4`、`browser_snapshot_status_ok=true`、`browser_snapshot_active_ok=true`、`refresh_during_detail_ok=true`、`refresh_during_graph_ok=true`、`refresh_during_selection_ok=true`、`snapshot_map_count=1`、`snapshot_node_count=1`、`snapshot_result_count=3`、`assistant_marker_observed=true`、`user_prompt_contains_marker=false`；`browser-summary.json` 显示浏览器刷新响应全部 200，刷新窗口内 resultCount 从 1 增长到 2，`consoleErrors=[]`、`networkFailures=[]`，favicon 404 单独计数为 `faviconConsoleErrorCount=1`。

### Phase 7：Benchmark / Audit 升级

目标：观察认知收益，而不是只看 map 是否存在。

脚本指标：

- `output_contract_present`
- `source_provenance_present`
- `claims_evidence_present`
- `unreviewed_result_dependency_count`
- `self_generated_data_leakage`
- `new_fact_per_node`
- `assumption_to_fact_conversion_rate`
- `state_delta_intent_present`
- `promotion_trigger`
- `promotion_latency`
- `collapse_rate`
- `wrong_premise_containment`

首轮 hard gate 只启用 MVP 指标：

- `output_contract_present`
- `source_provenance_present`
- `claims_evidence_present`
- `accepted_result_has_evidence`
- `self_generated_data_leakage`
- `questioned_or_invalid_result_in_cognitive_state_update`
- `questioned_or_invalid_final_artifact_dependency`
- `sentinel_warning_uncleared_for_final_artifact`
- `result_validity_transition_present`
- `audit_why_chain_missing`

`sentinel_warning_triggered`、promotion/collapse、state_delta_intent、new_fact_per_node 等指标先进入报告，不作为 hard gate；MVP 报告必须显式写 `promotion_not_in_mvp=true`。

样本：

- `hello-world`：输出编码契约。
- `heterogeneous-dates`：输出编码与 validator 读取契约。
- `jsonl-aggregator`：data provenance 和 wrong-premise containment。
- `multi-file-order-pipeline`：node state delta 和 result reuse。

验收：

- E2 clean 不回退。
- E3 小样本中 `jsonl-aggregator` 不再出现 self-generated data leakage。
- `hello-world` / `heterogeneous-dates` 不再因 BOM/UTF-16 失败。
- TaskSpace 不一定立即优于 standard，但必须能证明认知控制机制生效。

新增审计 fixture：

- `questioned-result-final-artifact`：构造一个被 `Questioned` result 支撑的最终产物，审计必须触发 `questioned_or_invalid_final_artifact_dependency` hard fail。
- `invalid-result-state-update`：构造 `Invalid` result 被写入 cognitive state 的路径，审计必须触发 `questioned_or_invalid_result_in_cognitive_state_update` hard fail。
- `result-validity-dependency-matrix`：参数化覆盖 `Questioned` / `Invalid` × `cognitive_state_update` / `final_artifact_dependency` 四种组合，四种都必须触发对应 hard fail。
- `uncleared-sentinel-final-artifact`：构造 output/provenance warning 影响最终产物且未清除，审计必须触发 `sentinel_warning_uncleared_for_final_artifact` hard fail。
- `audit-why-chain-complete`：给定一个最终产物，审计器必须能输出 artifact hash -> output contract -> result -> claim -> evidence -> validator/fact source 的完整链。
- `promotion-report-only`：MVP 报告必须包含 `promotion_not_in_mvp=true`，并且 `promotion_trigger/promotion_latency` 只能是 report-only 字段。
- `mvp-scope-regression`：报告或文档生成器如果把 promotion/collapse/barrier 计入 MVP pass 条件，测试必须失败。

实施记录（2026-06-04 Phase 7A）：

- `scripts/export-action-map-observability.ps1` 已扩展到读取 task、cognitive state、sentinel warning、result evidence package，并输出到 reduced JSON / Markdown / HTML。
- 审计逻辑拆入 `scripts/action-map-cognitive-audit-lib.ps1`，报告渲染拆入 `scripts/action-map-observability-report-lib.ps1`；主导出脚本只保留事件归集和 reduced model 生成，避免形成难维护的大脚本。
- 当前审计是 `mvp-structural-subset` gate，不做语义质量判断，也不代表完整 MVP final artifact why-chain 已完成：只检查 output contract 是否存在、fact source 是否存在、result 是否带 claims/evidence、accepted result 是否缺 evidence、active fact 是否引用可 join 且可信的 fact source、result validity transition 是否出现。
- 报告显式输出 `auditSchemaVersion=taskspace-cognitive-audit-v1`、`auditScope=mvp-structural-subset`、`fullMvpHardGateImplemented=false`、`promotionNotInMvp=true`、逐 gate records、unsupported MVP gate ids 和 metrics，避免把 promotion/collapse/final-artifact why-chain 误报为 MVP 完成能力。
- 使用旧 E3 benchmark artifact 导出时，审计会因为缺 output contract / fact source / claims evidence / validity transition 而失败；这不是回归，而是证明旧数据只有 map 结构、没有认知状态记录，不能被计入 cognitive MVP clean evidence。
- 已有 PowerShell 库测试覆盖 evidence package 派生字段、positive cognitive audit、self-generated data leakage、unsourced active fact、unknown fact source、黑盒导出 fixture、HTML trace-data JSON parse；尚未实现 final artifact dependency why-chain、artifact hash、questioned/invalid result 进入 final artifact dependency、sentinel clear lifecycle、完整 E2/E3 生产路径重跑。

实施记录（2026-06-05 Phase 7B）：

- `scripts/action-map-final-artifact-audit-lib.ps1` 已新增 final artifact why-chain 审计，不新增 parallel runtime，只复用 snapshot 中的 `outputContracts`、`NodeResultEvidencePackage.claims/evidenceRefs/changedArtifacts/validatorRefs`、`sentinelWarnings` 和现有 result/task/map join key。
- `scripts/action-map-observability-lib.ps1` 的 reduced model 补充 `taskId/mapId` 到 map/result，解决 final artifact dependency 无法稳定归属 task 的问题。
- `scripts/export-action-map-observability.ps1` 新增可选 `-ArtifactRoot`，E2/E3 相关导出入口已传入该 root。当 result `changedArtifacts` 或 evidence `artifactRef` 指向文件时，审计只允许解析 artifact root 内的文件并输出 SHA-256；绝对路径或相对路径逃逸 root 时不得 hash，必须触发 `final_artifact_hash_missing`。
- `cognitiveAudit.auditScope` 已升级为 `mvp-final-artifact-why-chain`，`fullMvpHardGateImplemented=true`，`unsupportedMvpGateIds=[]`。这只表示 MVP 审计 gate 已进入生产导出路径，不表示 TaskSpace 已有 E3 utility 正收益。
- 新增 hard gates：
  - `questioned_or_invalid_result_in_cognitive_state_update`
  - `output_contract_result_mismatch`
  - `non_accepted_final_artifact_dependency`
  - `questioned_or_invalid_final_artifact_dependency`
  - `sentinel_warning_uncleared_for_final_artifact`
  - `audit_why_chain_missing`
  - `final_artifact_hash_missing`
- `active fact` 的 evidence anchor 规则已纠正为：可以引用同 task 的可信 `factSourceId`，也可以引用同 task 的 `accepted resultId`；跨 task result、`questioned/invalid resultId`、`unreviewed resultId` 都不得进入权威 cognitive state。
- final artifact dependency 只接受 `accepted` result；`unreviewed` 会触发 `non_accepted_final_artifact_dependency`，`questioned/invalid` 同时触发更窄的 `questioned_or_invalid_final_artifact_dependency`。
- output contract 与 final artifact 的 join 不再用“同 task 任意 artifact 满足任意 contract”的宽松规则；必须通过显式 `artifactRef/path` 或 contract evidence refs 中的 `resultId` 与产物来源 result 机械 join。无法 join 的 contract 进入 `audit_why_chain_missing`。
- 如果 output contract 同时声明 `artifactRef/path` 与 `resultId`，该 contract 的 result refs 必须与该 task/artifact 实际 resultIds 至少有一个交集；否则触发 `output_contract_result_mismatch`。这样可以拒绝 path/result 错配，同时避免同一 task 同一路径存在多个 result 时误杀正确的最终 result。
- final artifact identity 使用 task-scoped key，避免两个 task 产出相同相对路径时被错误合并。
- `scripts/test-action-map-observability-lib.ps1` 已覆盖完整 why-chain 正例、invalid result 同时污染 active fact/final artifact dependency、unreviewed result 依赖、orphan artifact contract、contract path/result mismatch、缺 artifact hash、ArtifactRoot absolute/traversal containment、未清除 sentinel 影响 final artifact、跨 task accepted result source、同路径多 task 产物不合并、黑盒 export/report/HTML parse。
- Markdown report 已统一转义表格单元格中的 `|` 和换行，避免审计文本破坏人工报告表格；JSON/HTML 仍作为结构化 source of truth。
- 使用旧 E3 artifact 重跑导出时，新的 hard gate 仍会失败，但失败原因是旧运行缺 cognitive 记录：`required_output_contract_missing`、`required_fact_source_missing`、`result_claims_evidence_missing`、`result_validity_transition_missing`；这符合预期。
- 尚未进入本阶段的内容：dedicated final-artifact runtime event、runtime 主动 clear command、sentinel hard barrier、promotion/collapse、浏览器端 final artifact 交互图、完整 E2/E3 benchmark 重跑。

Phase 7B closure review 后续加固项：

- Windows reparse-point 指向 ArtifactRoot 外文件的 containment 负例已加入自动化测试：`scripts/test-action-map-reparse-containment.ps1` 使用非管理员可创建的 junction 指向 root 外文件，要求 audit 不接受 resolved path、不计算 artifact hash，并触发 `final_artifact_hash_missing`。
- wrapper 已接入 `ArtifactRoot`，但完整 utility benchmark 仍属于后续评估，不作为本轮 Phase 7B 关闭条件。
- `/task-show` viewer 的真实 TUI/browser 级路径已在 Phase 6B 补齐；它证明可观察页面的 live refresh 与 graph 交互可用，但不替代 E3 utility benchmark。

实施记录（2026-06-05 Phase 7C）：

- `Resolve-FinalArtifactPath` 现在通过 `Resolve-ReparseAwarePath` 逐段识别 Windows reparse point，将 junction/symlink 的剩余路径拼到真实 target 后再做 ArtifactRoot containment 判断。指向 root 外的 reparse path 不会被 hash，也不会被视为合法 final artifact。
- ArtifactRoot 自身如果因过深/循环/空 target 等原因无法解析真实路径，resolver 现在 fail closed，直接拒绝 artifact path，不允许绝对外部 artifact 走无 containment 的 hash 路径。
- `scripts/test-action-map-reparse-containment.ps1` 已接入 `scripts/run-action-map-regression.ps1` 默认 script matrix，避免该安全边界只停留在手工测试；非 Windows host 会输出 `Overall: SKIP`，wrapper 会记录 `skipped_script_runs`，不会把未覆盖误报为 PASS。
- 验证证据：`scripts/test-action-map-reparse-containment.ps1` 通过，覆盖 junction escape 与 unresolved root fail-closed 两个负例；`scripts/run-action-map-regression.ps1` 通过，报告 `target/test-reports/action-map-20260605-134247-920/report.md` 显示 10 个 cargo run、4 个脚本 run 全部 PASS，`skipped_script_runs=0`、`total_passed_tests=218`、`total_failed_tests=0`、`relevant_crash_events=0`。额外完整长链路 `scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E` 也通过，报告 `target/test-reports/action-map-20260605-134926-545/report.md` 显示 10 个 cargo run、5 个脚本 run 全部 PASS，`skipped_script_runs=0`，且 viewer E2E 的 `browser_interaction_ok=true`。

实施记录（2026-06-05 Phase 7D）：

- `export-action-map-observability.ps1` 已把 `sentinel_warning_cleared` 纳入 runtime event 白名单，并通过 `Add-Or-Update-SentinelWarning` 复用 raised/snapshot/cleared 的 warning 聚合逻辑。clear event 只更新 `status=cleared`、`clearanceAction`、`clearedAtMs`，不覆盖原始 `traceEventIds`，避免破坏 warning 触发证据链。
- `Get-FinalArtifactAuditSummary` 现在接收 timeline，并从 `sentinel_warning_cleared` details 中读取 `sentinelId`、clear action 与 task/map/node/result 上下文。影响 final artifact 的 warning 只有在 snapshot 自身 `status=cleared` 且 `clearanceAction` 合法，或 timeline 存在同 id、合法 action、上下文匹配、时间不早于 warning 的 clear event 时，才被视为已清除。
- 为保持脚本 API 兼容，`Get-FinalArtifactAuditSummary` 的 `$Timeline` 参数位于 `$ArtifactRoot` 之后；旧的 5 参位置调用仍把第 5 个参数解释为 `ArtifactRoot`。
- MVP 只允许三种 clear action：`FixApplied`、`RiskAcceptedByMainAgent`、`ContractRevised`。非法 action、自由文本 action、清错 sentinel id 都不会清除 `sentinel_warning_uncleared_for_final_artifact` gate。
- `scripts/test-action-map-sentinel-clearance.ps1` 已接入 `scripts/run-action-map-regression.ps1` 默认 script matrix。它覆盖 active warning 失败、三种合法 clear event 通过、非法 action 失败、错 sentinel id 失败、同 id 错上下文失败、clear event 早于 warning 失败、snapshot cleared + 合法 action 通过、旧 5 参 direct audit 兼容，以及 exporter 黑盒消费 clear event / 错上下文 / 早到 clear 的正反例。
- 验证证据：`scripts/test-action-map-sentinel-clearance.ps1` 通过；`scripts/test-action-map-observability-lib.ps1` 通过；`scripts/run-action-map-regression.ps1` 通过，报告 `target/test-reports/action-map-20260605-153412-167/report.md` 显示 10 个 cargo run、5 个脚本 run 全部 PASS，`total_passed_tests=218`、`total_failed_tests=0`、`skipped_script_runs=0`、`relevant_crash_events=0`。
- 边界：本阶段没有实现 runtime 主动 clear command，也没有实现 sentinel hard barrier。它只是让生产报告/audit 链路在事件存在时能做正确 hard gate，避免 `sentinel_warning_cleared` 停留在文档概念。

实施记录（2026-06-05 Phase 7E 完成审计）：

- 结论：问题状态管理首轮 MVP 已闭环。这里的“完成”只指机制闭环：agent 有结构化问题状态协议，runtime 能记录和约束 output contract、fact source、fact、result validity，viewer/audit 能解释 result 与 final artifact 的证据链，回归脚本能覆盖关键负例。不表示 TaskSpace 已经证明 E3 utility 正收益，也不表示 v1.1 的 promotion/collapse/sentinel hard barrier 已实现。
- 完整回归：`scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E` 通过，报告 `target/test-reports/action-map-20260605-154108-379/report.md` 显示 10 个 cargo run、6 个脚本 run 全部 PASS，`total_passed_tests=218`、`total_failed_tests=0`、`skipped_script_runs=0`、`relevant_crash_events=0`。
- Viewer 真实路径：同一报告中的 TUI viewer E2E 显示 `browser_interaction_ok=true`、`browser_snapshot_status_ok=true`、`browser_snapshot_active_ok=true`、`detail_state_ok=true`、`selection_state_ok=true`、`graph_zoom_ok=true`、`graph_pan_ok=true`、`refresh_during_detail_ok=true`、`refresh_during_graph_ok=true`、`refresh_during_selection_ok=true`、`snapshot_result_count=3`、`console_error_count=0`、`network_failure_count=0`。这证明 `/task-show` 的 live snapshot、自动刷新、选中态、详情展开、图拖拽/缩放路径可用。
- Direct trace：`taskspace_trace_event_recorded`、`sentinel_warning_raised`、`cognitive_state_updated`、`result_validity_changed`、`sentinel_warning_cleared` 都有稳定 join key；event 只承载最小 ID/状态，不复制 claims、reason、description，避免 direct trace 成为第二套权威状态。
- Output contract：`record_output_contract` 已进入 `taskspace_control` 生产 action，runtime 要求非空 ID/description/evidence refs；final artifact audit 会用 artifact path 或 result evidence refs 做机械 join，错配进入 `output_contract_result_mismatch`，缺链进入 `audit_why_chain_missing`。
- Data provenance：`record_fact_source` 与 `record_fact` 已进入生产 action；`record_fact` 只能引用同 task 的 accepted result，或 provenance 为 `observed_from_environment` / `provided_by_user` 的 fact source。`generated_for_test_only`、`inferred`、`unknown`、unreviewed/questioned/invalid result 都不能进入 active facts。
- Result evidence：`mark_result_validity` 已进入生产 action；`accepted` 必须有 top-level evidence refs 和 claims，每个 claim 也必须有 evidence refs；cross-result evidence、跨 task trace/result、无 evidence 的 accepted 都被 runtime 拒绝。
- Viewer/audit why-chain：snapshot、text snapshot、TUI viewer、observability export 都能展示 cognitive state 与 result evidence package。final artifact audit 能追溯 artifact hash -> output contract -> result -> claim -> evidence -> validator/fact source，并对 non-accepted/questioned/invalid final artifact dependency 触发 hard gate。
- Schema freshness：protocol snapshot、app-server generated schema、TypeScript/JSON fixture 的 action map cognitive/evidence 字段已经纳入 schema fixture 测试；`codex-app-server-protocol --test schema_fixtures` 在完整回归中通过。
- Sentinel clearing：observability export 与 final-artifact audit 已消费 `sentinel_warning_cleared`；只有 `FixApplied`、`RiskAcceptedByMainAgent`、`ContractRevised` 三种 action 可以清除影响 final artifact 的 warning。非法 action、错 id、错上下文、clear 早于 warning 都保持 hard fail。
- 安全边界：ArtifactRoot containment 包含 Windows reparse point 负例；junction/symlink 指向 root 外不会被 hash，不会被接受为合法 final artifact，并触发 `final_artifact_hash_missing`。
- 与 E3 负收益的关系：本轮解决的是 E3 暴露出的“map 没有真正管理问题状态、result 被当普通摘要、最终产物缺证据链、warning 无闭环”这些机制缺陷。是否能在成对 benchmark 中获得成功率/成本净收益，仍需新的 E3 utility benchmark 复测。
- 明确未完成项：runtime 主动 clear command、sentinel hard barrier、promotion/collapse、完整 facts/assumptions/decisions/open_questions 可编辑面板、dedicated final-artifact runtime event、浏览器端 final artifact 交互图、完整 E3 utility 正收益证明，全部不属于首轮 MVP 完成声明。

## Runtime Event 与 Audit Join Key

新增能力必须有结构化事件，不允许只写进 result body。

Phase 4A 已实现事件：

```text
taskspace_trace_event_recorded
sentinel_warning_raised
result_validity_changed
cognitive_state_updated
```

Phase 4A 事件只做 minimal ref notification：

- `cognitive_state_updated` 只携带 `task_id`、`map_id`、`update_kind`、`record_id`。
- `result_validity_changed` 只携带 `task_id`、`map_id`、`node_id`、`result_id`、`validity`。
- `validity_reason`、claims、evidence refs、description 等语义内容只存在于 `NodeResult.evidence_package` 或 `TaskState.cognitive_state`，不复制到 event 中，避免 event 成为第二套权威状态。

Phase 6/7 或 v1.1 事件：

```text
sentinel_warning_cleared
fact_source_recorded
output_contract_recorded
node_result_evidence_recorded
sentinel_barrier_raised
sentinel_barrier_cleared
taskspace_promoted
taskspace_collapsed
promotion_aborted
```

Phase 7D 已实现 `sentinel_warning_cleared` 在 observability export 与 final-artifact audit 中的消费；Rust runtime 仍未提供用户/agent 可调用的主动清除 action，后续要做时必须复用同一事件 schema。

事件公共字段：

```text
event_id
trace_id
task_id
map_id
node_id
result_id
call_id
source_thread_id
actor
created_at_ms
schema_version
```

事件特有字段：

| 事件 | 必须字段 |
|---|---|
| `taskspace_trace_event_recorded` | `trace_event_id`, `kind`, `action_class`, `tags`, `artifact_refs` |
| `sentinel_warning_raised` | `sentinel_id`, `sentinel_type`, `sentinel_status`, `severity`, `trigger_event_ids`, `clearance_action` |
| `result_validity_changed` | `task_id`, `map_id`, `node_id`, `result_id`, `validity` |
| `cognitive_state_updated` | `task_id`, `map_id`, `update_kind`, `record_id` |
| `sentinel_warning_cleared` Phase 6/7 | `sentinel_id`, `sentinel_status`, `clear_action`, `cleared_by`, `cleared_at_ms`, `clear_event_ids` |
| `output_contract_recorded` Phase 6/7 | `output_contract_id`, `kind`, `path_or_artifact`, `format`, `encoding`, `validator_refs` |
| `fact_source_recorded` Phase 6/7 | `fact_source_id`, `provenance`, `evidence_refs`, `confidence` |
| `node_result_evidence_recorded` Phase 6/7 | `claim_ids`, `evidence_refs`, `artifact_refs`, `validator_refs` |
| `sentinel_barrier_raised` v1.1 | `barrier_id`, `sentinel_id`, `sentinel_status`, `trigger_event_ids`, `barrier_reason`, `clearance_action` |
| `sentinel_barrier_cleared` v1.1 | `barrier_id`, `sentinel_id`, `sentinel_status`, `clear_action`, `cleared_by`, `cleared_at_ms`, `clear_event_ids` |
| `taskspace_promoted` v1.1 | `promotion_id`, `trigger_event_ids`, `inherited_trace_refs`, `promotion_payload_ref`, `initial_cognitive_state_ref` |
| `taskspace_collapsed` v1.1 | `promotion_id`, `collapse_reason`, `clear_event_ids`, `collapsed_at_ms`, `retained_cognitive_state_ref` |
| `promotion_aborted` v1.1 | `promotion_id`, `trigger_event_ids`, `abort_reason`, `aborted_by`, `aborted_at_ms` |

v1.1 事件字段现在先定义 schema 口径，首轮 MVP 不要求实现，也不允许在 MVP 报告中把 promotion/barrier 视为已闭环。

### Audit Artifact Schema

E2/E3 和手动回归必须输出可机械 join 的 audit artifact。第一版不要求审计器理解语义正确性，但必须能解释某个最终产物为什么被接受、质疑或污染。

```rust
pub(crate) struct CognitiveAuditRecord {
    pub(crate) audit_schema_version: String,
    pub(crate) run_id: String,
    pub(crate) pair_id: Option<String>,
    pub(crate) task_id: TaskId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: Option<MapNodeId>,
    pub(crate) final_artifact_id: Option<String>,
    pub(crate) final_artifact_path: Option<String>,
    pub(crate) artifact_hash: Option<String>,
    pub(crate) result_id: Option<NodeResultId>,
    pub(crate) result_validity_event_id: Option<String>,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
    pub(crate) fact_source_ids: Vec<String>,
    pub(crate) output_contract_ids: Vec<String>,
    pub(crate) validator_refs: Vec<String>,
    pub(crate) sentinel_ids: Vec<String>,
    pub(crate) promotion_id: Option<String>,
    pub(crate) promotion_not_in_mvp: bool,
    pub(crate) dependency_edges: Vec<FinalArtifactDependencyEdge>,
    pub(crate) gate_records: Vec<RunGateRecord>,
}

pub(crate) struct FinalArtifactDependencyEdge {
    pub(crate) from_kind: AuditDependencyKind,
    pub(crate) from_id: String,
    pub(crate) to_kind: AuditDependencyKind,
    pub(crate) to_id: String,
    pub(crate) validity_at_use: Option<ResultValidity>,
}

pub(crate) enum AuditDependencyKind {
    FinalArtifact,
    OutputContract,
    NodeResult,
    Claim,
    Evidence,
    FactSource,
    Validator,
    Sentinel,
    TraceEvent,
}

pub(crate) struct RunGateRecord {
    pub(crate) gate_name: String,
    pub(crate) expected: String,
    pub(crate) observed: String,
    pub(crate) source_artifact: String,
    pub(crate) fixture_id: String,
    pub(crate) pass: bool,
}
```

审计硬要求：

- `audit_schema_version` 必须显式存在。
- 最终产物存在文件时必须写 `artifact_hash`；没有文件产物时必须写明 `final_artifact_id` 和来源。
- 每个 final artifact dependency edge 必须能回溯到 result、claim、evidence、fact source、output contract 或 validator。
- `Questioned` / `Invalid` result 出现在 final artifact dependency edge 时，`questioned_or_invalid_final_artifact_dependency` gate 必须失败。
- sentinel warning 如果影响 final artifact 且没有有效清除记录，`sentinel_warning_uncleared_for_final_artifact` gate 必须失败。有效清除记录只能来自 snapshot `status=cleared` + 合法 `clearanceAction`，或 timeline 中同 id、合法 clear action、task/map/node/result 上下文至少一项匹配且无冲突、时间不早于 warning 的 `sentinel_warning_cleared`。
- `sentinel_warning_cleared.clear_action` 第一版只允许三种枚举值：`FixApplied`、`RiskAcceptedByMainAgent`、`ContractRevised`。不要用自由文本表达风险接受或契约修正，非法 action 等价于未清除。
- MVP 报告必须写 `promotion_not_in_mvp=true`；如果字段缺失，审计失败。

E3 audit artifact 必须能用这些 join key 回答：

```text
某个 final artifact 依赖哪些 accepted claims？
这些 claims 的 evidence 是什么？
这些 evidence 来自哪个 result / trace event / validator？
是否存在 GeneratedForTestOnly 或 Unknown provenance 泄漏？
为什么某个 result 被 accepted/questioned/invalid？
哪个 sentinel 触发过，怎么清除？
```

## 兼容和迁移策略

### Snapshot 兼容

- 新字段全部提供 default。
- restore 老 snapshot 时缺字段不失败。
- cognitive/evidence 字段必须先宽容通过协议反序列化，再由 runtime 按 `cognitive_schema_version` 判断是否采信；未知版本的 cognitive payload 是破坏性降级为空状态，不得被当作当前事实读取。
- viewer 对缺字段显示为空状态。
- 新增字段前必须先通过 generated schema freshness test。
- snapshot 带 `cognitive_schema_version`，缺失时视为 legacy snapshot。

### Tool schema 兼容

- 旧 `start_task/create_node/finish_node` 调用继续可用。
- 首轮只新增 MVP action，不一次性添加完整 cognitive 字段。
- 新 action 先 optional + prompt 要求使用；对应 E2/E3 稳定后再升级为 warning 或 barrier。
- 如果 `taskspace_control` schema 过大，优先拆 `taskspace_state` 工具，而不是继续扩充同一工具。
- 预实验已经确认当前 `taskspace_control` 只暴露 `start_task/route_task/create_node/bind_node/finish_node/block_node`。正式加 cognitive MVP action 时，必须保留旧 action 的行为，并把“当前缺字段”测试改写为“新增字段存在、旧字段兼容、promotion/collapse 仍不进入 MVP action”的测试。
- `record_output_contract`、`record_fact_source`、`mark_result_validity` 等新 action 的 schema 必须有最小必填字段和明确 ID：缺 `output_contract_id`、`fact_source_id`、`result_id`、`validity`、`evidence_refs` 时不能静默退化成 result body 文本。

### Runtime 兼容

- standard mode 完全不受影响。
- experiment mode 先观测，再逐步引入 barrier。
- Sentinel 第一版只记录 warning，再升级为 hard barrier。
- runtime 只消费结构化 trace event tag，不解析 shell 字符串和 preview。

## 风险与对策

| 风险 | 对策 |
|---|---|
| schema 过大导致模型负担上升 | 字段自然语言化，分阶段 optional，引入 compact prompt |
| runtime 过度 gate 导致任务无法继续 | sentinel 先 warning 后 hard barrier；所有 barrier 必须有明确清除动作 |
| 主 agent 机械填字段 | E2/E3 audit 观察 `claims_evidence_present` 和 `new_fact_per_node` |
| viewer 复杂度上升 | graph 只展示结构，cognitive state 放详情/侧栏 |
| generated data 规则误伤 legitimate fixture generation | `GeneratedForTestOnly` 允许用于自测，但禁止作为 final fact |
| result validity 被模型滥标 accepted | accepted 必须有 claims/evidence；下游依赖未审查 result 进入 audit |
| migration 破坏旧 rollout replay | restore default + replay repair tests |
| direct trace 变成第二套状态源 | trace 只 append-only；promotion 后唯一权威状态是 `TaskState.cognitive_state` |
| sentinel 变成隐藏语义判断器 | 只消费结构化 event tag；不解析命令字符串 |
| schema/generated 类型漂移 | schema freshness 作为 Phase 0 blocker |
| MVP 继续膨胀 | 首轮只做 output/provenance/result-evidence 三闭环 |
| contract-sketch 测试被误当成生产覆盖 | 测试名和报告必须显式标注 `contract_sketch`；E2/E3 结论只采信生产路径测试、真实 CLI/E2E 和 audit artifact |
| tool schema gap 被 prompt 掩盖 | 先用真实 `taskspace_control` schema 测试固定当前 action 集；正式实现后用正反向 schema 测试证明新 action/field 可用 |
| snapshot 字段新增破坏旧会话 | cognitive 字段必须 default；legacy restore、未知版本、viewer 空态测试是 Phase 3 blocker |

## 推荐实施顺序

```text
Phase 0: 文档和审查
Phase 0.1: preflight contract-sketch + real taskspace_control gap guard
Phase 0.5: protocol/generated schema freshness
Phase 1: append-only structured trace event
Phase 2: MVP sentinel warning
Phase 3: MVP data model + versioned snapshot
Phase 4: MVP control actions + validity guard
Phase 5: prompt/context for MVP protocol
Phase 6: viewer cognitive side panel
Phase 7: benchmark/audit MVP hard gates
Phase 8 (v1.1 / non-MVP): sentinel hard barrier only after warning path is clean
```

不要先做完整 heavy planning mode。先把 E3 暴露的输出契约、数据来源、结果采信和错误假设 containment 做实。

## 测试矩阵

| 层级 | 测试 |
|---|---|
| Preflight guard | `cognitive_preflight_contract_sketch_audit_*` 只检查契约草图；真实 `ActionMapSnapshotResult` JSON join key；真实 `taskspace_control` 当前 tool schema gap |
| Unit | data model default/restore、enum parse、validity transition、sentinel trigger |
| Integration | `taskspace_control` 新旧 schema、runtime gate、snapshot export/import、schema freshness |
| CLI smoke | `whale exec --taskspace` 自然 prompt、`task-show` viewer URL |
| Negative | 缺 output contract、GeneratedForTestOnly 写入 facts、Accepted 缺 evidence、Invalid result 进 facts、Questioned/Invalid result 进入 final artifact dependency |
| Audit fixture | uncleared sentinel 影响最终产物、合法/非法 sentinel clear action、why-chain 缺 artifact_hash、promotion_not_in_mvp 字段缺失、run gate expected/observed 缺失 |
| Replay | `hello-world` BOM、`heterogeneous-dates` UTF-16、`jsonl-aggregator` 自造数据 |
| E2 | existing action-map regression、natural-user、growth-health、natural-multi-agent |
| E3 small | `hello-world`、`heterogeneous-dates`、`jsonl-aggregator`、`multi-file-order-pipeline` |
| Browser | Playwright 验证 viewer auto-refresh 不破坏展开/选择，graph 可拖拽缩放 |
| v1.1 lifecycle | sentinel barrier raised/cleared、promotion promoted/collapsed/aborted，全部可按 ID 查询；Phase 8 开始前必须先有 promotion replay |

预实验已经覆盖的只是 `Preflight guard` 中的第一批检查。它们不能替代 Unit/Integration/Audit fixture：正式字段、正式 action、正式 snapshot 和正式 viewer 出现后，必须把 contract-sketch 规则迁移为生产路径测试。

正式实现阶段必须补齐的测试债务：

- cognitive state 真实 schema 的序列化 / 反序列化正例。
- legacy snapshot 缺 cognitive 字段时的 restore/default/viewer 空态测试。
- `cognitive_schema_version` 存在、缺失、未知版本的兼容测试。
- `taskspace_control` cognitive MVP action/field 的正向 schema 测试与旧 action 兼容测试。
- output contract / fact provenance / result validity / sentinel clearing / final artifact dependency 的 runtime transition 测试。
- audit artifact why-chain 测试：final artifact -> output contract -> result -> claim -> evidence -> validator/fact source 必须可机械 join。
- viewer / replay 测试：cognitive state 不仅存储在内部，还能被 `/task-show` 和 E2/E3 artifact 观察到。

## 第一轮完成定义

第一轮不是证明 TaskSpace 全面优于 standard，而是证明问题状态管理机制闭环可用：

- Direct trace 能记录 audit 和未来 promotion 可复用的结构化事实；MVP 不触发 promotion/collapse。
- Output contract 能阻止编码契约遗漏。
- Data provenance 能阻止自造数据污染 final facts。
- Result 能以 claims/evidence/validity 形式沉淀。
- Viewer 和 audit 能通过 join key 解释 result 为什么 accepted/questioned/invalid，并能追溯 final artifact 的 artifact hash、output contract、result、claim、evidence、validator 和 fact source。
- Schema freshness 防止 Rust/TS/JSON schema 漂移。
- E3 关键负例不再以同样形态失败。
- MVP 报告明确写 `promotion_not_in_mvp=true`；promotion/collapse 不作为首轮成功声明。
- 预实验中的 contract-sketch 测试仍保留为 guard，但所有 MVP hard gate 都必须至少有一条生产路径测试证明，不允许只靠 test-only helper 证明。
