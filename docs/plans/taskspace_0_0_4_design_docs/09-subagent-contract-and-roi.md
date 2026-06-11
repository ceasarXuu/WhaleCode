# 09. Subagent Contract 与 ROI 设计

## 1. 背景

0.0.3 已将 subagent 绑定到 node lease，这是正确边界。但 E3 结果显示，subagent result 不一定转化为主 agent decision。0.0.4 不增加更多 role，而是要求 subagent 有明确 expected artifact 和 adoption 路径。

## 2. Spawn 原则

允许 spawn 的条件：

```text
1. 子任务可并行，且与主路径存在清晰边界；
2. 子任务能生产明确 artifact；
3. 子任务能验证或推翻特定 hypothesis；
4. 子任务能减少主 agent 上下文压力；
5. 子任务输出有 acceptance check。
```

不建议 spawn：

```text
简单局部 bug；
已知 patch 路径；
只是让另一个 agent “看看”；
validator 已接近 timeout；
当前 task 没有 success criteria；
recommended mode = thin。
```

## 3. record_subagent_plan

spawn 前必须记录：

```text
parent_node_id
why_parallelizable
expected_artifact
acceptance_check
max_scope
supports_questions
tests_hypotheses
depends_on_results
```

## 4. Subagent result contract

subagent 输出应结构化：

```yaml
artifact_type: evidence_summary | patch_candidate | validation_result | risk_review
claims:
  - id:
    statement:
    evidence_refs:
confidence: low | medium | high
limits:
  - what was not checked
recommended_next_action:
changed_artifacts:
validator_refs:
```

## 5. Main agent adoption

主 agent 必须对 subagent result 做三步：

```text
1. mark_result_validity
2. adopt_result 或标记 unused/questioned/invalid
3. 若采用，record_decision / fact / hypothesis 引用该 result
```

## 6. ROI 指标

| 指标 | 公式/含义 |
|---|---|
| spawn_count | subagent spawn 次数 |
| subagent_result_count | subagent result 数 |
| accepted_subagent_results | accepted 的 subagent result |
| adopted_subagent_results | 进入 decision/fact/hypothesis 的 subagent result |
| decisions_supported_by_subagent_results | 被 subagent result 支撑的 decision 数 |
| patches_changed_due_to_subagent_results | subagent result 改变 patch 的次数 |
| subagent_decision_yield | decisions_supported_by_subagent_results / spawn_count |

## 7. Gate 与 warning

Hard gate：

```text
spawn 必须绑定 ready node；
spawn 前必须存在 record_subagent_plan；
spawn node 不得已有 active lease。
```

Soft warning：

```text
expected_artifact 为空；
acceptance_check 为空；
recommended thin 但 spawn_count > 0；
spawn_count > 0 且 adopted_subagent_results = 0。
```

## 8. 示例

```yaml
parent_node_id: node-3
why_parallelizable: Parser behavior and validator output schema are independent tracks.
expected_artifact: Concrete schema summary with file refs and failing validator message refs.
acceptance_check: Accept only if claims cite source files or validator stderr.
max_scope: read-only, no edits.
supports_questions: [q-2]
```

## 9. 验收

```text
每次 spawn 有 plan。
每个 subagent result 有 validity/adoption 状态。
graph-health.json 输出 subagent_decision_yield。
processing-pipeline 这类多 spawn 样本可判断 subagent 是收益还是噪声。
```
