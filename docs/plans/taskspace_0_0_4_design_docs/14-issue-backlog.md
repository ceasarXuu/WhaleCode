# 14. 0.0.4 Issue Backlog

## P0 Issues

### TS-004-01 CleanE3AuditManifest

目标：每个 E3 pair 输出可机械解释的 audit manifest。

交付：

```text
audit.yaml schema
audit emitter
aggregate inclusion/exclusion logic
manual review status fields
```

验收：

```text
每个 completed pair 有 audit.yaml。
aggregate 能统计 valid_utility_pairs / inconclusive / excluded_by_reason。
```

---

### TS-004-02 FailureTaxonomyV1

目标：每个 failed/timeout pair 有 failure classification。

交付：

```text
failure taxonomy enum
automatic classifier
pair report integration
aggregate summary
```

验收：

```text
failed/timeout pair failure_classification 不为空且不全是 unknown。
```

---

### TS-004-03 GraphHealthReportOnly

目标：每个 TaskSpace run 输出 graph-health.json。

交付：

```text
decision_density
result_adoption_rate
unreviewed_result_ratio
blocked_node_ratio
subagent_decision_yield
thin_mode_violation
warnings
```

验收：

```text
graph-health.json 出现在每个 TaskSpace pair package。
```

---

### TS-004-04 ProblemStateLedgerV1

目标：TaskState 持有权威问题状态账本。

交付：

```text
ProblemStateLedger model
success criteria
open questions
hypotheses
decisions
risks
next best action
trace events
viewer display basic
```

验收：

```text
start_task 后 ledger.objective 和 success_criteria 非空。
```

---

### TS-004-05 taskspace_control schema v2

目标：新增问题状态 action。

交付：

```text
record_success_criteria
record_open_question
close_open_question
record_hypothesis
update_hypothesis
record_decision
record_next_best_action
schema versioning
```

验收：

```text
agent 能通过 taskspace_control v2 更新 ledger。
```

---

### TS-004-06 ResultAdoptionV1

目标：result validity 进入 dependency/adoption 链。

交付：

```text
adoption state
adopt_result action
result -> fact/hypothesis/decision/criterion refs
invalid final gate
questioned patch gate
```

验收：

```text
record_decision 引用 invalid result 时被阻断。
final_synthesis 引用 invalid result 时被阻断。
```

---

### TS-004-07 TypedNodeKindContractV1

目标：node 有 canonical kind 和 definition of done。

交付：

```text
discover/diagnose/design/patch/validate/synthesize
legacy kind mapping
kind-specific finish requirements
viewer display
```

验收：

```text
validate node finish 没有 command/validator evidence 时被阻断。
```

## P1 Issues

### TS-004-08 SubagentContractV1

交付：

```text
record_subagent_plan
spawn justification
expected artifact
acceptance check
subagent result contract
ROI metrics
```

验收：

```text
spawn 前必须有 subagent plan；graph health 输出 subagent_decision_yield。
```

---

### TS-004-09 ThinModeClassifierReportOnly

交付：

```text
complexity classifier
recommended_mode
thin/standard/deep report
thin mode violation warning
```

验收：

```text
recover-accuracy-log 输出 recommended_mode=thin 或解释为什么不是。
```

---

### TS-004-10 ViewerV2

交付：

```text
ProblemStateLedger display
Decision evidence refs
Result adoption summary
Graph health warnings
Audit readiness panel
```

验收：

```text
/task-show 能看到 objective、criteria、questions、decisions、graph health。
```

## P2 Issues

```text
TS-004-11 Graph prune/merge report-to-action design
TS-004-12 Reborn fact migration policy
TS-004-13 Automatic mode switching experiment
TS-004-14 Expanded benchmark suite design
```
