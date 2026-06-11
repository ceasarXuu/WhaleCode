# 05. taskspace_control Schema v2 设计

## 1. 设计目标

0.0.3 的 `taskspace_control` 已能管理 task/node/lease/result，但 action 仍偏执行结构。v2 的目标是把问题状态、证据、假设、决策、风险纳入工具 contract。

## 2. Action 分组

| 分组 | Action |
|---|---|
| Task bootstrap | `start_task`, `route_task`, `record_success_criteria` |
| Problem-state ledger | `record_open_question`, `close_open_question`, `record_hypothesis`, `update_hypothesis`, `record_decision`, `record_risk`, `record_next_best_action` |
| Node control | `create_node`, `bind_node`, `finish_node`, `block_node` |
| Evidence/result | `record_fact_source`, `record_fact`, `mark_result_validity`, `adopt_result` |
| Subagent | `record_subagent_plan` |
| Audit/failure | `classify_failure` |

## 3. 修改现有 action

### 3.1 start_task

0.0.3：

```text
required: task_title, node_title, node_context_summary
optional: task_objective, node_kind, bind_current
```

0.0.4：

```text
required:
  task_title
  task_objective
  initial_success_criteria
  node_kind
  node_title
  node_context_summary
optional:
  bind_current
  initial_open_questions
  initial_risks
```

### 3.2 create_node

新增字段：

```text
expected_artifact
closes_questions
tests_hypotheses
depends_on_results
supports_criteria
risk_flags
mode_hint
```

### 3.3 finish_node

新增字段：

```text
produced_result_refs
closed_questions
updated_hypotheses
created_decisions
updated_criteria
remaining_open_questions
next_best_action
```

### 3.4 mark_result_validity

增强字段：

```text
adoption_target: none | fact | hypothesis | decision | criterion | validation
adoption_refs:
  fact_ids
  hypothesis_ids
  decision_ids
  criterion_ids
```

## 4. 新增 action 设计

### 4.1 record_success_criteria

用途：记录任务完成标准。

```json
{
  "action": "record_success_criteria",
  "criteria": [
    {
      "id": "sc-1",
      "kind": "validator",
      "description": "Public validator exits with code 0",
      "status": "open",
      "evidence_refs": []
    }
  ]
}
```

### 4.2 record_open_question

用途：显式记录当前缺口。

```json
{
  "action": "record_open_question",
  "question_id": "q-1",
  "question": "Which files are required by the expected output contract?",
  "reason": "Needed before patching generated artifacts",
  "blocking": true,
  "opened_by_node_id": "node-1"
}
```

### 4.3 close_open_question

```json
{
  "action": "close_open_question",
  "question_id": "q-1",
  "resolution": "Validator expects six JSONL files and results.json",
  "closed_by_result_id": "result-8",
  "evidence_refs": [{"result_id": "result-8"}]
}
```

### 4.4 record_hypothesis

```json
{
  "action": "record_hypothesis",
  "hypothesis_id": "h-1",
  "statement": "The failure is caused by path-dependent output placement",
  "confidence": "medium",
  "evidence_refs": [{"result_id": "result-5"}],
  "falsification_check": "Run validator after writing outputs in expected directory"
}
```

### 4.5 update_hypothesis

```json
{
  "action": "update_hypothesis",
  "hypothesis_id": "h-1",
  "status": "supported",
  "evidence_refs": [{"result_id": "result-12"}],
  "reason": "Validator failure message matched missing output path"
}
```

### 4.6 record_decision

```json
{
  "action": "record_decision",
  "decision_id": "d-1",
  "decision_kind": "patch",
  "decision": "Generate recovered output files directly from parsed logs",
  "rationale": "All blocking schema questions are closed and required artifacts are known",
  "depends_on_results": ["result-8", "result-12"],
  "depends_on_facts": ["fact-1"],
  "resolves_questions": ["q-1"],
  "supports_criteria": ["sc-1"]
}
```

### 4.7 record_next_best_action

```json
{
  "action": "record_next_best_action",
  "node_id": "node-4",
  "action_summary": "Patch output generation and run public validator once",
  "reason": "Patch decision is recorded and blocking questions are closed",
  "expected_artifact": "Generated output files and validator evidence"
}
```

### 4.8 adopt_result

用于把 accepted result 正式绑定到 ledger 对象。

```json
{
  "action": "adopt_result",
  "result_id": "result-8",
  "adoption_state": "accepted_adopted",
  "adopted_by": {
    "facts": ["fact-1"],
    "decisions": ["d-1"],
    "criteria": ["sc-1"]
  }
}
```

### 4.9 record_subagent_plan

```json
{
  "action": "record_subagent_plan",
  "parent_node_id": "node-3",
  "why_parallelizable": "Parser and validator-schema investigation are independent evidence tracks",
  "expected_artifact": "Schema summary with concrete file refs",
  "acceptance_check": "Main agent will accept only if output cites files or validator lines",
  "max_scope": "read-only inspection, no edits",
  "supports_questions": ["q-2"]
}
```

### 4.10 classify_failure

```json
{
  "action": "classify_failure",
  "failure_classes": ["taskspace_overhead_timeout", "validator_slow_or_flaky"],
  "reason": "TaskSpace public validator exited 124 while standard passed; graph health shows no large node expansion in this pair",
  "evidence_refs": [{"artifact_ref": "taskspace.validator.stderr.txt"}]
}
```

## 5. 兼容策略

- v1 action 保留；v2 action 加 schema version。
- 旧 `output_contract` 可以映射为 `success_criteria`，但不自动视为 satisfied。
- 旧 `facts` 保留，但没有 decision refs 时 adoption state 仍为 unknown/legacy。
- viewer 对 legacy trace 标记 `schema_incomplete`。

## 6. 错误消息原则

错误应告诉 agent 缺什么，而不是只说 invalid：

```text
TaskSpace final_synthesis blocked: q-2 is still blocking/open. Close or defer the question with evidence before final synthesis.
```

```text
TaskSpace record_decision blocked: depends_on_results contains result-14 marked invalid.
```
