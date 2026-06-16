# 00. TaskSpace v0.0.5 Executive Summary

## 1. 版本定位

v0.0.5 不是继续堆 TaskSpace 结构，也不是简单加 budget hard stop。

v0.0.5 的定位是：

```text
TaskSpace v0.0.5 = Protocol Compaction + Context Replay Control + Map Self-Management Foundation
```

它要解决两个问题：

1. **运行成本问题**：v0.0.4 的 TaskSpace 模式相比 Standard 模式出现约 5x agent time 和约 20x direct input+output token；根因已经基本确认，是模型请求轮次约 9.31x 与单请求上下文约 2.16x 的乘法效应。
2. **map 管理能力问题**：当前 map 已经具备 objective / criteria / facts / decisions / adoption / graph health 等结构，但更像模型可见的结构化日志，还没有成为能够替代标准线性上下文的语义工作记忆。

## 2. 两个大目标

### 目标 A：上下文长度与轮次治理

阶段性目标：

```text
TaskSpace 收敛后的 agent walltime <= Standard 的 2x
TaskSpace 收敛后的 direct input+output tokens <= Standard 的 2x
```

这里的“收敛后”指 v0.0.5 新协议、新上下文投影、新大输出引用化、新 thin routing 启用后的 TaskSpace profile，不是 v0.0.4 的 legacy full protocol。

拆解指标：

```text
model_request_count_ratio <= 2.5x
avg_input_per_request_ratio <= 1.25x
uncached_input_ratio <= 2x
output_token_ratio <= 2x
taskspace_control_call_count <= 0.35 * v0.0.4 baseline
large_output_replay_count = 0
```

为什么先看 request count：v0.0.4 的最大乘数是模型请求轮次，而不是单次请求变慢。修复优先级必须从减少模型可见协议轮次开始。

### 目标 B：完善 TaskSpace map 自我管理能力

v0.0.5 不直接替换标准上下文。它要完成替换前置能力：

```text
map 可以保留语义状态；
artifact 可以保留原始证据；
runtime 可以压缩、投影、裁剪 map；
模型只读取当前最小充分状态。
```

本版要建立五类能力：

```text
Retention: 信息保留等级
Compaction: 多条 observation/result 压缩成 fact/decision
Salience: 按当前任务重要性排序
Projection: 每轮只投影 active working set
Garbage Collection: stale/unreviewed/blocked/no-yield 信息出 active context
```

## 3. 非目标

v0.0.5 不做：

| 非目标 | 原因 |
|---|---|
| 不直接用 map 替换标准上下文 | 当前 map 还未证明语义替代率和管理能力足够 |
| 不继续扩 benchmark 样本 | 当前瓶颈是协议成本与 map 管理，不是样本数 |
| 不继续增加 subagent 类型 | subagent ROI 未成立，先修 adoption 和 routing |
| 不做 full automatic planner | 会继续放大模型可见 orchestration loop |
| 不把 budget 作为唯一治理手段 | budget 是保险丝，不是根因修复 |
| 不把 graph health 只保留为报告项 | v0.0.5 必须让 graph health 影响收敛和上下文投影 |

## 4. 设计主线

v0.0.5 的设计主线是：

```text
减少模型可见协议轮次
  -> 批量 state_commit
  -> runtime 自动处理 routine bookkeeping
  -> gate 返回 next-valid-action 而非只拒绝

减少每轮上下文负担
  -> dynamic context projection
  -> static protocol cache / elision
  -> 大输出引用化
  -> active working set only

增强 map 自我管理
  -> retention class
  -> compaction pipeline
  -> salience score
  -> context projection
  -> GC / archive-to-audit

保持语义价值
  -> decision adoption chain
  -> result lifecycle
  -> thin / verification-first routing
  -> audit evidence 不丢失，但不进入每轮 prompt
```

## 5. 成功判定

v0.0.5 成功不是“TaskSpace 大幅超过 Standard”。它的阶段成功标准是：

```text
1. TaskSpace 成本收敛到 2x 以内，或接近 2x 且主要 outlier 可解释；
2. TaskSpace solved 不低于 Standard，且不低于 v0.0.4 的 raw 8/15 太多；
3. map active context 不随任务线性膨胀；
4. result / node / subagent debt 能被压缩、归档或废弃；
5. 大工具输出不再污染后续多轮模型上下文；
6. E3 报告能同时展示 request-count、avg-input/request、state_commit 数、projection size 和 semantic compaction 指标。
```

## 6. 版本名称建议

```text
TaskSpace v0.0.5 — Protocol Compaction & Map Self-Management
```
