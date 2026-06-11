# 10. Graph Health 与 Viewer v2 设计

## 1. 目标

Graph Health 的目标是回答：

```text
图是否在帮助 agent 收敛？
还是只是在制造活动量？
```

Viewer v2 的目标是让人类和 agent 快速恢复当前问题状态，而不是手动读完整 transcript。

## 2. graph-health.json

每个 TaskSpace run 输出：

```json
{
  "schema_version": "taskspace-graph-health-v1",
  "node_count": 0,
  "edge_count": 0,
  "result_count": 0,
  "decision_count": 0,
  "unreviewed_result_ratio": 0.0,
  "result_adoption_rate": 0.0,
  "decision_density": 0.0,
  "blocked_node_ratio": 0.0,
  "open_question_closure_rate": 0.0,
  "subagent_decision_yield": 0.0,
  "thin_mode_violation": false,
  "warnings": []
}
```

## 3. 指标定义

| 指标 | 公式 | 用途 |
|---|---|---|
| decision_density | decision_count / node_count | 衡量 node 是否转化为决策 |
| result_adoption_rate | accepted_adopted_results / accepted_results | 衡量 accepted result 是否被使用 |
| unreviewed_result_ratio | unreviewed_results / total_results | 衡量 result 噪声 |
| blocked_node_ratio | blocked_nodes / total_nodes | 衡量图收敛问题 |
| node_inflation_ratio | node_count / max(1, decision_count) | 衡量图膨胀 |
| open_question_closure_rate | closed_questions / total_questions | 衡量问题状态推进 |
| subagent_decision_yield | decisions_supported_by_subagent_results / spawn_count | 衡量 subagent ROI |
| validation_rework_count | repeated validate cycles without new decision | 衡量验证循环 |

## 4. Warning taxonomy

```text
high_unreviewed_result_ratio
low_decision_density
high_blocked_node_ratio
node_inflation_high
subagent_no_adoption
thin_mode_violation
validation_loop
synthesis_not_ready
stale_ready_node
decision_tainted_by_questioned_result
```

## 5. Viewer v2 结构

Viewer 不应只展示 node graph。建议分区：

```text
1. Task header
2. ProblemStateLedger
3. Active/blocked node graph
4. Decisions and evidence refs
5. Result validity/adoption summary
6. Subagent ROI summary
7. Graph health warnings
8. Audit readiness
9. Next best action
```

## 6. Viewer 示例

```text
Task: Recover accuracy logs
Mode recommendation: thin

Objective:
  Generate recovered output files and results.json, then pass public validator.

Success Criteria:
  [satisfied] sc-1: Expected output artifacts exist.
  [open]      sc-2: Public validator exits 0.

Open Questions:
  q-1 [closed] Required output file set confirmed by validator.

Hypotheses:
  h-1 [supported] Output reconstruction is determined by parsed judge logs.

Decisions:
  d-1 [patch] Generate artifacts directly from parsed logs.
       depends_on: result-7, result-8

Graph Health:
  nodes=4 edges=3 results=18
  decision_density=0.50
  unreviewed_result_ratio=0.38
  warnings=[]

Next Best Action:
  Run public validator once and update sc-2.
```

## 7. Audit readiness display

Viewer should show:

```text
validator evidence present: yes/no
cleanup artifact present: yes/no
diff present: yes/no
graph health present: yes/no
failure taxonomy present: yes/no
included_in_utility: true/false/inconclusive
```

## 8. 验收

```text
每个 TaskSpace run 输出 graph-health.json。
/task-show 展示 ProblemStateLedger 和 graph health warnings。
viewer 能从 final graph 快速判断卡点。
```
