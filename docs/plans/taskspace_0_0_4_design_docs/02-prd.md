# 02. TaskSpace 0.0.4 PRD

## 1. 背景

TaskSpace 0.0.3 完成了 runtime 可运行性验证，但 E3 diagnostic 未证明 utility 正收益。0.0.4 的产品任务不是增加更多执行结构，而是让现有结构成为 agent 可依赖的问题状态管理层。

## 2. 目标用户与角色

| 角色 | 需求 |
|---|---|
| Main agent | 在多步骤工程任务中维护目标、事实、假设、决策、风险，不被线性历史淹没 |
| Runtime | 强制结构合法，维护状态机、result 引用和 audit artifact |
| Subagent | 在限定上下文下生产可采信证据，而不是泛泛建议 |
| Benchmark maintainer | 能机械判断 pair included/excluded，区分 agent failure 和环境/validator failure |
| Human reviewer | 能通过 viewer 快速恢复任务状态和失败原因 |
| Product owner | 能判断 TaskSpace 是否具备进入默认模式或复杂任务模式的证据 |

## 3. 用户故事

### US-1：主 agent 启动任务时明确完成标准

作为 main agent，我必须在 start_task 后记录 objective 和 success criteria，以便后续 patch、validation、final synthesis 都有明确完成标准。

验收：普通工具调用前若 active task 没有 success criteria，runtime 阻断或强告警。

### US-2：主 agent 把调查结果转为问题状态

作为 main agent，我需要把调查结果记录为 known fact、hypothesis、open question 或 decision，而不是只写在 result_summary 中。

验收：finish_node 可以关联 closed_questions、updated_hypotheses、created_decisions。

### US-3：主 agent 不能依赖 invalid result

作为 runtime，我必须阻止 invalid result 进入 final synthesis 或 patch decision。

验收：record_decision 若 depends_on_results 包含 invalid result，返回结构化错误。

### US-4：人类 reviewer 能快速判断图是否健康

作为 reviewer，我需要看到 decision density、unreviewed result ratio、subagent yield、thin mode violation，而不是手动读完整 transcript。

验收：每个 TaskSpace run 输出 `graph-health.json`，viewer 显示 graph health warnings。

### US-5：benchmark 能输出 clean aggregate

作为 benchmark maintainer，我需要每个 pair 都有 audit manifest，能判断 inclusion/exclusion/inconclusive。

验收：0.0.4 E3 aggregate 中 `valid_utility_pairs` 不再因 audit missing 全部为 0；若仍为 0，必须有机械解释。

## 4. 功能需求

### FR-1 ProblemStateLedger

系统必须在 TaskState 中持有权威 ProblemStateLedger，包括：

```text
objective
success_criteria
known_facts
open_questions
hypotheses
decisions
risks
blockers
next_best_action
```

### FR-2 taskspace_control schema v2

新增 action：

```text
record_success_criteria
record_open_question
close_open_question
record_hypothesis
update_hypothesis
record_decision
record_next_best_action
record_risk
classify_failure
record_subagent_plan
```

### FR-3 ResultAdoption

NodeResult 必须支持 adoption 状态：

```text
unreviewed
accepted_unused
accepted_adopted
questioned
invalid
```

### FR-4 TypedNodeKindContract

新增 canonical node kinds：

```text
discover
diagnose
design
patch
validate
synthesize
```

并定义每类 node 的 required output。

### FR-5 GraphHealthReport

每个 run 输出：

```text
node_count
edge_count
result_count
unreviewed_result_ratio
result_adoption_rate
decision_density
blocked_node_ratio
subagent_decision_yield
thin_mode_violation
validation_loop_count
```

### FR-6 CleanE3AuditManifest

每个 pair 输出 audit manifest，包含 standard/taskspace artifact、validator evidence、cleanup evidence、failure taxonomy、inclusion decision。

## 5. 非功能需求

| 需求 | 说明 |
|---|---|
| Backward compatibility | 0.0.3 trace 作为 historical evidence 保留；0.0.4 新 schema versioned |
| Low friction | 低复杂度任务不应强制 deep graph；先输出 report-only classifier |
| Deterministic audit | aggregate 不能依赖散落人工解释 |
| Minimal semantic runtime | runtime 不判断语义真假，只维护显式引用与状态 |
| Fail closed | remote asset 不可证明等价时继续 fail-closed |
| Cleanup preserved | 0.0.3 Docker cleanup 基线不得回退 |

## 6. 验收标准

### Must

```text
每个 TaskSpace run 有非空 success criteria。
每个 TaskSpace run 有 graph-health.json。
每个 pair 有 audit manifest。
每个 failed/timeout pair 有 non-unknown failure taxonomy。
invalid result 不能进入 final synthesis。
blocking open question 未关闭时不能 final synthesis。
```

### Should

```text
recover-accuracy-log 能输出 thin mode recommendation。
processing-pipeline 的 subagent result adoption 可观测。
result_adoption_rate 和 unreviewed_result_ratio 可在 aggregate 中展示。
```

### Could

```text
部分 warning 转 hard gate。
viewer 支持 decision dependency drill-down。
```

## 7. 成功/失败判定

0.0.4 成功不是“TaskSpace 全面胜过 Standard”，而是：

```text
TaskSpace utility 能进入 clean audit 证据链；
TaskSpace graph 能解释自己的存在；
TaskSpace result 能解释自己如何支持 decision；
TaskSpace failure 能被分类和复盘。
```
