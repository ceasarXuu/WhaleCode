# 08. Typed Nodes 与 Graph Convergence 设计

## 1. 背景

0.0.3 node kind 偏执行阶段，不能充分表达认知任务。`inspect_code_context` 过载导致 node 粒度不稳定：有时是读文件，有时是诊断，有时是设计，有时是 baseline validation。

0.0.4 的目标是：node 成为“认知状态转换单元”。

## 2. Canonical node kinds

| Kind | 定义 | 典型产出 |
|---|---|---|
| discover | 查明代码结构、文件、接口、运行入口 | relevant files, known facts, open questions |
| diagnose | 定位 bug/root cause 或失败原因 | hypotheses, evidence refs, falsification checks |
| design | 选择修改方案 | decisions, tradeoffs, risks |
| patch | 修改代码/生成 artifact | changed artifacts, patch rationale |
| validate | 运行测试、validator、smoke check | command, exit code, stdout/stderr refs, criterion updates |
| synthesize | 汇总完成状态 | satisfied criteria, accepted decisions, remaining risks |

## 3. 旧 kind 映射

| 0.0.3 kind | 0.0.4 canonical |
|---|---|
| inspect_code_context | discover / diagnose / design |
| implement_solution | patch |
| smoke_test | validate |
| regression_test | validate |
| final_synthesis | synthesize |

0.0.4 可先保留旧 kind，但内部显示 canonical kind。

## 4. Definition of Done

### discover

必须至少产出一类：

```text
relevant_files
known_facts
open_questions
```

### diagnose

必须产出：

```text
hypothesis 或 rejected_hypothesis
evidence_refs
falsification_check 或 next validation action
```

### design

必须产出：

```text
decision
rationale
tradeoff/risk
supporting refs
```

### patch

必须产出：

```text
changed_artifacts
patch_rationale
expected_behavior
```

如果无修改，必须说明 no-edit rationale，并关联 design decision。

### validate

必须产出：

```text
command
exit_code/failure_reason
stdout/stderr/artifact refs
criterion status update
```

### synthesize

必须产出：

```text
satisfied criteria
accepted decisions
remaining risks
excluded/questioned evidence summary
```

## 5. Node 粒度准则

一个好 node 应满足至少一个条件：

```text
关闭一个 open question；
验证/推翻一个 hypothesis；
产生一个 design/patch/validation decision；
交付一个明确 artifact；
消除一个 blocker。
```

反模式：

```text
Read one known file
Continue investigation
Ask subagent to look around
Try something
Fix more issues
```

## 6. Graph convergence

0.0.4 先不实现硬 prune/merge，但要报告：

```text
node_inflation_ratio
stale_ready_nodes
blocked_node_ratio
leaf_nodes_without_result
nodes_without_decision_or_question_effect
```

## 7. Node budgets

report-only budget：

| Mode | Node budget hint | Subagent budget hint |
|---|---:|---:|
| thin | 1-4 | 0 |
| standard | 4-12 | 0-3 |
| deep | 12+ | 3+ with ROI tracking |

超过 budget 不阻断，但输出 warning。

## 8. Reborn 与事实迁移

0.0.4 不建议重做完整 `/task-reborn`，但要定义原则：

```text
accepted facts 可以迁移；
questioned/invalid result 不自动迁移；
open blocking questions 必须重新确认；
decisions 迁移时保留 source refs 和 risk note。
```

## 9. 验收

```text
每个 node 有 canonical kind。
finish_node 根据 kind 检查最小 output。
graph-health.json 包含 node convergence warnings。
viewer 能显示 node kind、expected artifact、closed questions、created decisions。
```
