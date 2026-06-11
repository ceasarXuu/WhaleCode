# 13. Migration 与实施计划

## 1. Schema versioning

新增：

```text
taskspace_schema_version = taskspace-v2
problem_state_ledger_version = problem-state-ledger-v1
graph_health_version = graph-health-v1
audit_manifest_version = taskspace-e3-audit-v1
```

0.0.3 trace 不迁移为 v2，只在 viewer 中作为 legacy 展示。

## 2. 数据迁移策略

| 旧字段 | 新字段 | 策略 |
|---|---|---|
| task.objective | ledger.objective | 新 run 直接写入；旧 run legacy display |
| outputContracts | success_criteria | 新 run 建议替换；旧 run可显示为 legacy output contract |
| facts | known_facts | 新 run 保留并加强 evidence refs |
| result validity | result adoption | 新增 adoption refs；旧 result adoption unknown |
| node kind | canonical node kind | 运行时映射 |

## 3. 实施阶段

### Stage 1：Audit / GraphHealth 先落地

目标：不改变 agent 行为，先把证据链补齐。

交付：

```text
audit.yaml
graph-health.json
failure taxonomy classifier
aggregate update
```

### Stage 2：ProblemStateLedger 与 schema v2

目标：改变 agent 必填状态，但只加入少量 hard gate。

交付：

```text
ProblemStateLedger
record_success_criteria
record_open_question
record_decision
record_next_best_action
start_task schema update
```

### Stage 3：ResultAdoption 与 typed node contract

目标：让 result/decision/node 进入引用链。

交付：

```text
adopt_result
result dependency refs
invalid/questioned gates
canonical node kinds
kind-specific finish requirements
```

### Stage 4：Subagent / Thin mode / Viewer v2

目标：可观测协作收益和低摩擦模式。

交付：

```text
record_subagent_plan
subagent ROI metrics
thin/standard/deep classifier report-only
viewer v2
```

## 4. 开发顺序

```text
1. CleanE3AuditManifest
2. FailureTaxonomyV1
3. GraphHealthReportOnly
4. ProblemStateLedgerV1
5. taskspace_control schema v2
6. ResultAdoptionV1
7. TypedNodeKindContractV1
8. SubagentContractV1
9. ThinModeClassifierReportOnly
10. ViewerV2
```

## 5. 回滚策略

| 改动 | 回滚方式 |
|---|---|
| taskspace_control v2 | 支持 schema_version，回退 v1 action |
| hard gate | feature flag 关闭为 warning |
| graph health | 纯 report-only，可安全保留 |
| audit manifest | 不影响 agent execution，可保留 |
| viewer v2 | 保留 legacy viewer |

## 6. Feature flags

```text
taskspace.problem_ledger.enabled
taskspace.schema_v2.enabled
taskspace.gate.final_synthesis.enabled
taskspace.gate.invalid_result.enabled
taskspace.graph_health.enabled
taskspace.audit_manifest.enabled
taskspace.thin_mode.report_only
```

## 7. 最小可交付 0.0.4

如果时间受限，保留：

```text
CleanE3AuditManifest
FailureTaxonomyV1
GraphHealthReportOnly
ProblemStateLedgerV1 minimal
ResultAdoptionV1 minimal final gate
```

推迟：

```text
SubagentContractV1
ThinModeClassifierReportOnly
ViewerV2 full drill-down
graph prune/merge hard actions
```
