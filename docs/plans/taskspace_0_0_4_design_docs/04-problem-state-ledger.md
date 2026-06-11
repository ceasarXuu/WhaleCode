# 04. ProblemStateLedgerV1 详细设计

## 1. 目标

ProblemStateLedgerV1 的目标是把 TaskSpace 的“问题状态”从自然语言 result body 中抽出来，成为 TaskState 的 first-class runtime contract。

它要回答：

```text
当前目标是什么？
完成标准是什么？
已验证事实是什么？
尚未回答的问题是什么？
当前假设是什么？
已经做了哪些决策？
还存在什么风险？
下一步最小高价值行动是什么？
```

## 2. 数据模型

```text
ProblemStateLedger
├── objective
├── success_criteria[]
├── known_facts[]
├── open_questions[]
├── hypotheses[]
├── decisions[]
├── risks[]
├── blockers[]
├── next_best_action
└── updated_at_ms
```

## 3. 字段定义

### 3.1 objective

任务目标，必须由 start_task 初始化。objective 不应只是用户原话，而应是 agent 归纳后的工程目标。

示例：

```text
Recover accuracy logs by generating expected output files and results.json from available logs, then pass the public validator.
```

### 3.2 success_criteria

完成标准。每条标准必须包含：

```text
id
description
kind
status
evidence_refs
```

建议 kind：

```text
artifact
behavior
test
validator
compatibility
performance
user_visible_output
```

状态：

```text
open
satisfied
questioned
waived
```

### 3.3 known_facts

已验证事实，必须有 evidence refs。不得记录“我猜测”式事实；猜测应进入 hypothesis。

### 3.4 open_questions

未解决问题。每个问题要标注是否 blocking。

示例：

```text
q-1: Which files are required by the validator output contract?
blocking: true
```

### 3.5 hypotheses

未完全验证但可推进的问题模型。

字段：

```text
id
statement
confidence
status
evidence_refs
falsification_check
```

状态：

```text
proposed
supported
rejected
superseded
```

### 3.6 decisions

决策是 0.0.4 的关键对象。每个 patch/design/validation/synthesis decision 必须引用 supporting evidence。

字段：

```text
id
decision_kind
decision
rationale
depends_on_results
depends_on_facts
resolves_questions
supports_criteria
risks
```

### 3.7 risks

风险用于记录仍可接受但未完全消除的不确定性。final synthesis 必须展示 remaining risks。

### 3.8 next_best_action

下一步行动不是自由文本计划，而是当前问题状态下的最小高价值行动。

字段：

```text
node_id
action_summary
reason
expected_artifact
blocked_by
```

## 4. 生命周期

### 4.1 初始化

`start_task` 必须提供：

```text
objective
initial_success_criteria
first_node
```

如果没有 initial_success_criteria，runtime 应阻断普通工具调用，要求先补 `record_success_criteria`。

### 4.2 调查阶段

discover / diagnose node 完成后，应至少更新一类 ledger 对象：

```text
known_fact
open_question
hypothesis
risk
```

### 4.3 设计阶段

design node 完成后，必须产生 decision。decision 必须解释它依赖哪些 result/fact/hypothesis。

### 4.4 Patch 阶段

patch node 完成后，必须记录 changed artifacts，并关联 patch decision。

### 4.5 Validate 阶段

validate node 完成后，必须更新 success criteria status。

### 4.6 Synthesize 阶段

final synthesis 前必须满足：

```text
blocking open questions = 0
至少一个 validation criterion satisfied 或 waived
final decision 不依赖 invalid result
remaining risks 已记录
```

## 5. Gate 策略

| 场景 | Gate |
|---|---|
| task 没有 success criteria | 阻断普通工具调用 |
| patch decision 无 evidence refs | hard error 或强告警，建议 0.0.4 初期 hard error 仅限 invalid/questioned 引用 |
| final synthesis 前有 blocking open question | hard error |
| final synthesis 未引用 satisfied criteria | hard error |
| open questions 长期未变化 | graph health warning |

## 6. Viewer 展示

viewer 应优先展示 ledger，而不是只展示 graph：

```text
Objective
Success Criteria
Known Facts
Open Questions
Hypotheses
Decisions
Risks
Next Best Action
```

## 7. 示例：recover-accuracy-log

```yaml
objective: Recover accuracy log outputs and results.json from provided log files.
success_criteria:
  - id: sc-1
    kind: artifact
    description: All expected recovered output files are generated.
    status: open
  - id: sc-2
    kind: validator
    description: Public validator exits with code 0.
    status: open
open_questions:
  - id: q-1
    question: Which source logs determine run boundaries?
    blocking: true
hypotheses:
  - id: h-1
    statement: Accuracy can be reconstructed by grouping judge events by run id.
    confidence: medium
decisions:
  - id: d-1
    decision_kind: patch
    decision: Generate expected output artifacts directly from parsed log files.
    depends_on_results: [result-7, result-8]
```
