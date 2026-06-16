# 05. Map Self-Management

## 1. 目标

v0.0.5 的 map 不直接替换标准上下文，但要具备替换所需的管理能力。

目标不是让 map 更大，而是让 map 能回答：

```text
什么信息应该继续 active？
什么信息应该压缩？
什么信息应该归档？
什么信息应该只保留 audit 引用？
什么信息可以安全从模型上下文里消失？
```

## 2. 当前缺口

v0.0.4 的 map 已经能记录：

```text
objective
success criteria
facts
decisions
result validity/adoption
graph health
```

但还缺：

```text
retention class
compaction operators
salience score
projection policy
GC / archival lifecycle
semantic replacement metric
```

因此当前 map 更像结构化日志，而不是上下文管理系统。

## 3. Retention Class

每个 map item 都应有 retention class。

```text
Active: 当前推理必须看到
Retained: 重要，但当前不必每轮看到
Archived: 已压缩/过期，可按需展开
AuditOnly: 只用于审计，不进入模型上下文
Discarded: 明确废弃，不再使用
```

### 默认分类

| 对象 | 默认 retention |
|---|---|
| objective | Active |
| active success criteria | Active |
| satisfied criteria | Retained |
| accepted decision | Active/Retained，取决于是否相关当前 node |
| rejected hypothesis | Archived |
| raw tool output | AuditOnly |
| large stdout/stderr | AuditOnly |
| stale blocked node | Archived |
| unreviewed result older than N steps | Retained -> Archived |
| rejected subagent result | Archived/AuditOnly |

## 4. Compaction Operators

map 必须有显式压缩算子。

### Result Collapse

```text
多个 raw results -> 一个 accepted fact / rejected finding
```

示例：

```text
result-12: grep output
result-13: validator failure text
result-14: local checker output
=> fact-5: expected first line is stack trace count, not weighted frame total
```

### Node Collapse

```text
多个 completed nodes -> 一个 phase summary
```

示例：

```text
nodes: inspect logs, parse format, run checker
=> phase-summary: output format understood; parser strategy chosen
```

### Failure Collapse

```text
多个失败尝试 -> 一个 rejected hypothesis
```

示例：

```text
hypothesis: weighted total should be reported
status: rejected
reason: validator expects count 646
```

### Subagent Collapse

```text
多个 subagent raw outputs -> accepted/rejected evidence summary
```

### Validation Collapse

```text
多次 validator stdout/stderr -> latest validation state + failure class
```

## 5. Salience Score

每个 map item 计算 salience，用于决定 projection。

建议因素：

```text
+ 当前 node 直接依赖
+ 支撑 active success criterion
+ 关闭 blocking open question
+ 支撑 patch decision
+ 最近 validator failure
+ 推翻旧假设
+ 用户明确约束
- 已被 rejected / superseded
- 无 decision adoption
- stale age 高
- audit-only
```

输出：

```json
{
  "item_id": "fact-9",
  "salience": 0.92,
  "reasons": ["supports_current_decision", "validator_failure_related"]
}
```

## 6. Projection Policy

projection 不是完整 map，而是从 map 投影出来的 active working set。

默认规则：

```text
1. objective 必保留
2. active criteria 必保留
3. current node 必保留
4. current blockers 必保留
5. top K accepted decisions 按 salience 保留
6. top K facts 按 salience 保留
7. latest validator state 必保留
8. unreviewed results 只保留数量和 top risky items，不保留全文
9. stale/archive/audit-only 不进入 projection
```

## 7. Garbage Collection

GC 不一定删除，可以是状态转移：

```text
Active -> Retained
Retained -> Archived
Archived -> AuditOnly
Rejected -> Discarded/AuditOnly
```

触发条件：

```text
- node completed and phase summary exists
- result adopted/rejected/deferred
- result age > N steps and no dependency
- blocked node has no path after synthesis checkpoint
- subagent no-yield after review
- decision superseded by newer decision
```

## 8. Map Self-Management Checkpoint

每个阶段结束时 runtime 自动运行：

```text
1. classify new results
2. suggest compaction candidates
3. identify stale nodes
4. update salience scores
5. produce next projection
6. record GC trace event
```

模型只需要对语义不确定的项做判断。

## 9. Shadow Replacement Metrics

v0.0.5 不实际替换标准上下文，但要测 map 替代潜力：

```text
semantic_replacement_rate:
  final decisions explainable from map / all final decisions

history_shadow_elidable_tokens:
  old history tokens covered by map facts/decisions/summaries

active_context_convergence:
  projection size does not grow linearly with elapsed turns
```

## 10. 验收指标

| 指标 | 目标 |
|---|---:|
| 100% map items have retention class | 是 |
| 100% TaskSpace runs produce projection | 是 |
| unreviewed result active count | 比 v0.0.4 下降 >= 60% |
| stale blocked nodes in final projection | 0 |
| semantic replacement rate | >= 70% |
| projection size growth | sublinear / bounded |
| archived raw output with artifact refs | 100% large outputs |

## 11. 后续版本接口

v0.0.5 完成后，v0.0.6/0.0.7 可以开始试验：

```text
older history actual elision
map-backed conversation compaction
standard context replacement in controlled profiles
```

但 v0.0.5 只做 shadow metrics，不直接切换。
