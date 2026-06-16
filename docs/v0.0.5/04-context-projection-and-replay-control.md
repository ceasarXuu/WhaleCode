# 04. Context Projection and Replay Control

## 1. 背景

v0.0.4 的第二个成本乘数是每次模型请求的输入变大。更重要的是，TaskSpace 状态并没有替代标准 history，而是叠加到了标准 history 上。

v0.0.5 的目标不是删掉 TaskSpace 状态，而是把完整状态拆成：

```text
runtime/audit 可见的完整状态
模型每轮可见的最小充分状态
```

## 2. 核心原则

```text
完整 map 是数据库，不是 prompt。
完整 trace 是审计证据，不是每轮推理材料。
原始大输出是 artifact，不是 history。
模型每轮只需要 active working set。
```

## 3. Context ProjectionV1

每轮模型请求前，runtime 根据当前 task state 生成 projection。

### Projection 分区

```text
1. Active Objective
2. Active Success Criteria
3. Current Node / Current Phase
4. Blocking Questions / Current Risks
5. Adopted Decisions
6. Top Relevant Facts
7. Latest Validator State
8. Relevant Result Summaries
9. Next Valid Actions
10. Hidden but available evidence refs
```

### 默认不进入 projection

```text
- completed stale nodes
- unreviewed raw result bodies
- rejected subagent outputs
- old shell stdout/stderr
- full graph edge list
- full success criteria history
- all prior taskspace_control calls
- full subagent transcript
- full validator logs
```

这些仍保存在 artifact/audit/map 中，但不进入模型 active context。

## 4. Static / Dynamic context split

TaskSpace context 分为：

| 类型 | 处理 |
|---|---|
| Static protocol | 只在进入 TaskSpace 或重大模式切换时注入；优先 prompt-cache；不每轮重述 |
| Dynamic state | 每轮生成短 projection |
| Evidence detail | 引用化，按需展开 |
| Debug/audit detail | 不进入模型；只进入 artifact/viewer/report |

v0.0.4 中 `build_developer_context` 会构造模型可见 TaskSpace protocol、task inventory、active task path、node list、current node contract、collaboration guidance。v0.0.5 应拆解为 static protocol + dynamic projection。

## 5. OutputReferenceV1

大工具输出必须引用化。规则：

| 输出大小 | 默认处理 |
|---:|---|
| <= 8KB | 可直接返回模型 |
| 8KB - 50KB | 摘要 + head/tail + artifact ref |
| 50KB - 150KB | 默认 artifact ref + summary；模型需显式 request_slice |
| >150KB | 禁止直接进入 history；必须 artifact ref + sampling/slicing |

### 返回结构

```json
{
  "output_ref": "artifact://tool-output/result-42.txt",
  "sha256": "...",
  "bytes": 169047,
  "summary": "Access log with ~N lines. Contains HTTP method, path, status, and timestamp fields.",
  "head": "first 20 lines...",
  "tail": "last 20 lines...",
  "suggested_slices": [
    {"name": "status_distribution", "command": "..."},
    {"name": "sample_errors", "command": "..."}
  ],
  "raw_output_elided": true
}
```

## 6. Slice-on-demand

模型可以请求：

```text
- line range
- grep pattern
- head/tail
- statistical summary
- structured parse
```

但不能默认把全文塞回 context。

## 7. History elision 前置能力

v0.0.5 不直接替换标准上下文，但要开始做 shadow elision：

```text
每次 projection 生成时，标记哪些旧 messages 可以由 map state 替代。
```

输出指标：

```text
history_tokens_retained
history_tokens_shadow_elidable
projection_tokens
context_replacement_potential = shadow_elidable / projection_tokens
```

这个指标为 v0.0.6/0.0.7 真正替换标准上下文做准备。

## 8. Projection size budget

每轮 projection 目标：

```text
active objective: <= 300 tokens
criteria summary: <= 500 tokens
current node/phase: <= 500 tokens
facts/decisions: <= 1500 tokens
result summaries: <= 1500 tokens
next valid actions: <= 500 tokens
warnings/blockers: <= 500 tokens
```

默认 projection 总大小目标：

```text
<= 5k tokens for thin
<= 8k tokens for standard
<= 12k tokens for deep
```

如果超过预算，runtime 必须进行 compaction 或只保留 high-salience items。

## 9. Replay control

新增 replay guard：

```text
large_output_replay_count
replayed_tool_output_bytes
replayed_taskspace_control_history_tokens
replayed_graph_snapshot_tokens
```

v0.0.5 验收：

```text
large_output_replay_count = 0
full raw output >50KB 不得出现在下一轮模型 prompt
completed old taskspace_control history 不得全文进入 projection
```

## 10. 验收指标

| 指标 | v0.0.5 目标 |
|---|---:|
| avg_input_per_request_ratio | <= 1.25x Standard |
| max_input_per_request | 比 v0.0.4 top outlier 下降 >= 70% |
| large_output_replay_count | 0 |
| projection_tokens p95 | <= profile budget |
| raw tool output >50KB in prompt | 0 |
| history_shadow_elidable measured | 100% TaskSpace runs |

## 11. 设计风险

| 风险 | 缓解 |
|---|---|
| 摘要丢失关键信息 | artifact ref + slice-on-demand + hash 保证可回读 |
| projection 太短导致模型失忆 | salience scoring + active blocker/decision 强制保留 |
| 大输出引用化影响日志类任务 | 提供 structured summary / grep / slice tools |
| hidden history elision 与标准上下文冲突 | v0.0.5 只做 shadow，不实际替换 |
