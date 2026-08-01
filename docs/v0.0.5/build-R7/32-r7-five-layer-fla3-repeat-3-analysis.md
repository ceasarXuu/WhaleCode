# R7 五层架构 FLA-3 三轮重复对比

- 日期：2026-07-21
- 状态：`completed`
- 源码提交：`9b5038522`
- 模型：`deepseek-v4-flash`，reasoning effort `max`
- 执行环境：Docker 硬边界，bundled Skills 启用，plugins 关闭
- Projection policy：`map-request`
- 机器结果：[`five-layer-fla3-repeat-3-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla3-repeat-3-result.json)

## 1. 有效性

`single-file-fast-fix` 与 `subscription-billing-repair` 各执行 3 个 Standard/TaskSpace pair，共 6 个 pair、12 个
side。12 个 side 均完整、可比较，且全部通过业务、公开和隐藏验证。左右臂按 repeat 交替，统计使用
`logical-mode-map.json`，不把固定 left/right 当成模式。

正式运行前有两组零请求预检失败：第一次因 FLA-3 测试提交使旧 binary attestation 过期；第二次因 key 只在
`.env.local` 中而未导出到 benchmark shell。二者均在 provider 请求前停止并标记为 `invalid_harness`，不进入下表。

## 2. 逐轮结果

| 样本 | Repeat | 模式 | 成功 | Request | 普通 Tool | Control/失败 | Input | Uncached | Output | Cache hit | R2+ hit | Wall |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Simple | 1 | Standard | 是 | 5 | 4 | 0/0 | 58,969 | 1,241 | 1,128 | 97.90% | 97.60% | 13.54s |
| Simple | 1 | TaskSpace | 是 | 12 | 11 | 8/3 | 194,450 | 14,226 | 4,014 | 92.68% | 92.21% | 41.35s |
| Simple | 2 | Standard | 是 | 7 | 6 | 0/0 | 83,064 | 1,400 | 1,203 | 98.31% | 98.18% | 15.50s |
| Simple | 2 | TaskSpace | 是 | 12 | 9 | 9/4 | 189,374 | 11,070 | 3,306 | 94.15% | 93.77% | 33.49s |
| Simple | 3 | Standard | 是 | 7 | 9 | 0/0 | 85,885 | 1,917 | 1,651 | 97.77% | 97.56% | 16.79s |
| Simple | 3 | TaskSpace | 是 | 9 | 6 | 7/3 | 132,276 | 9,780 | 3,055 | 92.61% | 91.88% | 25.89s |
| Complex | 1 | Standard | 是 | 11 | 17 | 0/0 | 167,352 | 4,664 | 5,288 | 97.21% | 97.03% | 48.31s |
| Complex | 1 | TaskSpace | 是 | 22 | 29 | 8/3 | 464,418 | 31,394 | 8,714 | 93.24% | 93.08% | 82.84s |
| Complex | 2 | Standard | 是 | 19 | 25 | 0/0 | 354,651 | 6,747 | 7,821 | 98.10% | 98.04% | 67.54s |
| Complex | 2 | TaskSpace | 是 | 21 | 21 | 14/8 | 443,316 | 26,548 | 9,580 | 94.01% | 93.87% | 88.29s |
| Complex | 3 | Standard | 是 | 24 | 38 | 0/0 | 512,438 | 11,446 | 9,647 | 97.77% | 97.72% | 93.59s |
| Complex | 3 | TaskSpace | 是 | 18 | 17 | 14/8 | 339,686 | 26,342 | 9,684 | 92.25% | 91.99% | 88.45s |

## 3. 总和、均值与中位数

| 样本 | 模式 | Request 总/均/中 | Input 总/均/中 | Uncached 总/均/中 | Output 总/均/中 | Wall 总/均/中 |
|---|---|---|---|---|---|---|
| Simple | Standard | 19 / 6.33 / 7 | 227,918 / 75,973 / 83,064 | 4,558 / 1,519 / 1,400 | 3,982 / 1,327 / 1,203 | 45.82s / 15.27s / 15.50s |
| Simple | TaskSpace | 33 / 11.00 / 12 | 516,100 / 172,033 / 189,374 | 35,076 / 11,692 / 11,070 | 10,375 / 3,458 / 3,306 | 100.74s / 33.58s / 33.49s |
| Complex | Standard | 54 / 18.00 / 19 | 1,034,441 / 344,814 / 354,651 | 22,857 / 7,619 / 6,747 | 22,756 / 7,585 / 7,821 | 209.44s / 69.81s / 67.54s |
| Complex | TaskSpace | 61 / 20.33 / 21 | 1,247,420 / 415,807 / 443,316 | 84,284 / 28,095 / 26,548 | 27,978 / 9,326 / 9,580 | 259.57s / 86.52s / 88.29s |

| 样本 | TaskSpace / Standard Request | Input | Uncached | Output | Wall | R2+ cache delta |
|---|---:|---:|---:|---:|---:|---:|
| Simple | 1.74x | 2.26x | 7.70x | 2.61x | 2.20x | -5.09pp |
| Complex | 1.13x | 1.21x | 3.69x | 1.23x | 1.24x | -4.67pp |
| 合计 | 1.29x | 1.40x | 4.35x | 1.43x | 1.41x | 约 -4.60pp |

两个样本合计，Standard/TaskSpace 的 request 为 `73/94`，input 为 `1,262,359/1,763,520`，wall time 为
`255.26s/360.31s`。加权 cache hit 为 `97.83%/93.23%`。TaskSpace 缓存并未失效，但 uncached input 仍是
Standard 的 4.35 倍。

## 4. Map 与 Skill

| 样本 | Repeat | 节点/边 | Open leaf | Root | 语义保留 |
|---|---:|---:|---:|---|---:|
| Simple | 1/2/3 | 5/4，5/4，5/4 | 0/0/0 | completed | 100% |
| Complex | 1/2/3 | 5/4，5/4，7/7 | 0/0/0 | completed | 100% |

6 个 TaskSpace run 均创建并闭合有效 Map，没有孤立 open leaf。`taskspace-advanced` catalog 在 6/6 中可见，正文在
0/6 中被加载；因此 FLA-3 的能力载体和隔离仍成立，但自然选择率和高级 Skill 效用仍未得到正向证据。

## 5. 新暴露的问题

TaskSpace 共调用 control 60 次，其中失败 29 次：

| 错误 | 次数 | 含义 |
|---|---:|---|
| `TASKSPACE_REQUIRED_SIBLING_MISSING` | 21 | lifecycle 动作没有在同一 response 携带约定的 ordinary sibling call |
| `TASKSPACE_LIFECYCLE_INVARIANT` | 4 | 提交的节点迁移不符合当前生命周期 |
| `TASKSPACE_GRAPH_INVARIANT` | 2 | 初始化或图变更不符合当前依赖图 |
| `TASKSPACE_INVALID_ARGUMENT` | 1 | 初始化参数不符合 schema |
| `TASKSPACE_PROTOCOL_FAILURE` | 1 | `read_map` 协议调用不完整 |

此外 Simple 有 2 次在 Map 初始化前调用普通工具，被 `no_task_path` 硬约束拒绝。失败结果均如实进入上下文，Agent
最终都能恢复；但 21/29 的主导失败说明“control + 后续动作”的调用合同仍没有稳定转化为 Agent 的一次 response
行为，这是当前最明确的请求与 uncached input 放大来源之一。

## 6. 结论

1. 正确性结论稳定：Standard 与 TaskSpace 均为 6/6 成功，没有 FLA-3 引入的业务回归。
2. 原单轮 Simple 的 16-request TaskSpace 路径不是稳定中心，新三轮为 9/12/12，中位数 12；但相对 Standard
   中位数 7，流程成本仍明显偏高。
3. Complex 的 request 仅放大 13%，且第三轮 TaskSpace 为 18、Standard 为 24；TaskSpace 并非结构性地必然产生
   更多请求。其 input 和 wall 仍分别放大 21% 和 24%。
4. 当前最稳定的负向信号不是 FLA-3 Skill 正文，而是 L4 lifecycle carrier 的实际使用：Agent 频繁单独提交 control，
   再经历拒绝和恢复。该问题应作为后续 L4 正式阶段的优先输入，不应通过 Runtime 替 Agent补动作或改写语义。
5. 本轮只覆盖两个样本，足以替代单次 smoke 作为阶段重复证据，但不能外推为广泛效用结论。
