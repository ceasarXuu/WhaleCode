# 03. Protocol Compaction

## 1. 背景

v0.0.4 的最大成本乘数是模型请求轮数。根因不是 `taskspace_control` 这个概念错误，而是它的粒度太细：大量 bookkeeping 被拆成模型可见工具调用。

v0.0.5 要把 TaskSpace 状态推进从：

```text
一件小事一个工具调用
```

改成：

```text
一个阶段一次批量提交
```

## 2. 设计目标

```text
taskspace_control call count <= v0.0.4 baseline 的 35%
model_request_count_ratio <= 2.5x Standard
gate retry request 显著下降
finish/validity/adoption/decision 不再分散成多轮调用
```

## 3. 新动作：`state_commit`

`state_commit` 是 v0.0.5 的核心协议压缩动作。

它一次提交以下变更：

```text
- node transition
- result validity/adoption
- known facts update
- hypothesis update
- decision creation/update
- success criteria status update
- open question close/defer
- next node / next action
- blocker resolution
```

### 例子

```json
{
  "action": "state_commit",
  "active_node_id": "node-7",
  "node_update": {
    "status": "completed",
    "summary": "Parsed validator expected output and generated local checker."
  },
  "result_updates": [
    {
      "result_id": "result-18",
      "validity": "accepted",
      "adoption": "fact",
      "summary": "Validator expects first line to contain stack trace count, not weighted total."
    },
    {
      "result_id": "result-19",
      "validity": "rejected",
      "reason": "Subagent counted weighted frames, not stack traces."
    }
  ],
  "facts": [
    {
      "fact_id": "fact-9",
      "statement": "Expected output uses stack trace count 646, not weighted frame total.",
      "evidence_refs": ["result-18"]
    }
  ],
  "decisions": [
    {
      "decision_id": "decision-4",
      "kind": "verification",
      "decision": "Regenerate output.txt using validator-compatible stack-trace count.",
      "depends_on": ["fact-9"],
      "supports_criteria": ["criterion-output-format"]
    }
  ],
  "next_action": {
    "kind": "tool",
    "summary": "Run local output checker before public validation."
  }
}
```

## 4. 兼容旧动作

v0.0.5 不立刻删除 v0.0.4 actions，而是分三类处理：

| 旧 action | v0.0.5 处理 |
|---|---|
| `record_success_criteria` | 可在 `start_task` 或 `state_commit` 内批量提交 |
| `finish_node` | 收敛到 `state_commit.node_update` |
| `mark_result_validity` | 收敛到 `state_commit.result_updates` |
| `adopt_result` | 收敛到 `state_commit.result_updates.adoption` |
| `record_fact` | 收敛到 `state_commit.facts` |
| `record_decision` | 收敛到 `state_commit.decisions` |
| `block_node` | 收敛到 `state_commit.blockers` |
| `create_node` / `bind_node` | 保留，但尽量由 runtime next-valid-action 自动建议 |

## 5. Runtime 自动 bookkeeping

以下状态可以由 runtime 自动维护，不应要求模型逐项调用：

```text
- finished node 的 result_refs 绑定
- validator output 与 active validate node 绑定
- edit action 与 patch node 绑定
- tool output artifact ref 绑定
- success criteria 与 validator pass/fail 的基础关联
- stale node age / stale result age
- graph health warning 计算
```

模型只在语义判断处介入：

```text
这个结果是否可信？
这个事实是否重要？
这个 decision 为什么成立？
这个失败是否推翻假设？
```

## 6. Gate 从 reject/retry 改为 next-valid-action

v0.0.4 中，gate 常表现为：

```text
动作不合法 -> 拒绝 -> 模型再猜下一步
```

v0.0.5 gate 输出必须包含：

```json
{
  "allowed": false,
  "reason": "final_synthesis_not_ready",
  "blocking_items": ["criterion-output-format has no evidence"],
  "next_valid_actions": [
    {
      "action": "state_commit",
      "template": "adopt validator result as evidence or waive criterion"
    },
    {
      "action": "create_node",
      "kind": "validate",
      "template": "run local output checker"
    }
  ]
}
```

这样减少模型通过多轮试错学习 TaskSpace 协议。

## 7. State commit 的粒度

一个 `state_commit` 对应一个“认知阶段结束”，而不是一个自然语言段落。

建议触发点：

```text
- 完成一次重要工具调查后
- 形成 patch decision 前
- validator 失败后需要更新假设时
- subagent 返回后进行批量采纳/废弃时
- 进入 final synthesis 前
```

不建议触发点：

```text
- 每次看到一条小事实
- 每个工具输出后立即 record
- 每个 result 单独 validity 标记
- 每个 node 状态单独 finish/bind
```

## 8. 验收指标

| 指标 | v0.0.4 baseline | v0.0.5 目标 |
|---|---:|---:|
| `taskspace_control` calls / 15 runs | 850 | <= 300 |
| `finish_node` 独立调用 | 209 | <= 50 或被 state_commit 替代 |
| `mark_result_validity` 独立调用 | 149 | <= 50 或被 state_commit 替代 |
| `record_success_criteria` 独立调用 | 114 | <= 30 或 start_task 批量化 |
| model request ratio | 9.31x | <= 2.5x |
| gate retry count | 新增统计 | 比 v0.0.4 下降 >= 70% |

## 9. 实施顺序

```text
Phase 1.1: 新增 state_commit schema 和 handler
Phase 1.2: 保持旧 action 兼容，但报告 legacy-action usage
Phase 1.3: prompt 改为优先 state_commit
Phase 1.4: runtime 自动绑定 routine result/edit/validator evidence
Phase 1.5: gate 返回 next_valid_actions
Phase 1.6: E3 小样本对比 taskspace_control call count
```

## 10. 设计风险

| 风险 | 缓解 |
|---|---|
| state_commit 太大，模型难填 | 提供模板和允许 partial commit |
| 批量提交导致错误一起进入状态 | runtime 校验 dependency refs，commit 可局部接受/拒绝 |
| 旧 prompt 仍使用旧 action | legacy action 计数报警，逐步 soft-deprecate |
| 过度自动 bookkeeping 误归因 | 所有 auto-link 产出 trace event，便于 audit |
