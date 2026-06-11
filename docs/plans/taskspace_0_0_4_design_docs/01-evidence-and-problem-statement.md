# 01. 0.0.3 证据与问题定义

## 1. 证据摘要

0.0.3 已完成工程链路证明，但未完成 utility 证明。关键结论：

```text
TaskSpace 0.0.3 能跑，但没有证明 agent 跑得更好。
```

E3 diagnostic 结果：

| 方向 | 数量 |
|---|---:|
| TaskSpace better | 0 |
| Standard better | 3 |
| Both success | 5 |
| Both failed | 7 |
| clean utility pairs | 0 |

## 2. 0.0.3 样本级证据

下表基于 0.0.3 evidence pack 中的 pair index 与 TaskSpace graph dump 统计。注意：这里的 result_total 是 graph/detail 侧记录的 node result 数，包括 main tool call、result、blocker 等，比高层 run summary 里的聚合 result 口径更细。

| Sample | Standard success | TaskSpace success | TS nodes | TS edges | TS detailed results | Unreviewed results | Accepted results | Direction summary |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `processing-pipeline` | 3/5 | 2/5 | 59 | 103 | 357 | 297 | 50 | both_success=2, standard_better=1, both_failed=2 |
| `multi-source-data-merger` | 0/5 | 0/5 | 56 | 60 | 214 | 157 | 35 | both_failed=5 |
| `recover-accuracy-log` | 5/5 | 3/5 | 46 | 43 | 175 | 131 | 35 | both_success=3, standard_better=2 |

## 3. 关键观察

### 3.1 success criteria 未成为 contract

外部 graph dump 中三个主要样本的 `success_criteria_total` 均为 0，而 output contract、fact source、fact 已经有一定记录。这说明 0.0.3 的 cognitive state 已经开始记录事实和输出契约，但没有把“完成标准”提升为 first-class contract。

影响：

```text
final synthesis 无法机械判断是否满足任务；
validator/audit 难以知道哪些 artifact 是必须的；
agent 容易把“做了一些事”误当成“任务完成”。
```

### 3.2 result validity 有动作，但 adoption 不足

三个样本中 unreviewed result 占比很高：

| Sample | Unreviewed / Results | 粗略占比 |
|---|---:|---:|
| processing-pipeline | 297 / 357 | 83.2% |
| multi-source-data-merger | 157 / 214 | 73.4% |
| recover-accuracy-log | 131 / 175 | 74.9% |

这不要求所有 result 都必须被 review。问题在于：0.0.3 缺少结构化字段说明“哪些 result 被采信为 decision 的依据”。因此无法判断大量 result 是有效背景，还是纯噪声。

### 3.3 node 仍偏动作化

0.0.3 node kind 主要是：

```text
inspect_code_context
implement_solution
smoke_test
regression_test
final_synthesis
```

这些 kind 能表达执行阶段，但不能表达问题状态转换。例如 `inspect_code_context` 同时承载 discover、diagnose、design、baseline validation、subagent integration，导致节点语义过宽。

### 3.4 recover-accuracy-log 暴露低摩擦问题

`recover-accuracy-log` 中 Standard 5/5，TaskSpace 3/5。两个 TaskSpace 失败 pair 的形态不同：

| Pair | Direction | Nodes | Edges | Results | Failure classes |
|---|---|---:|---:|---:|---|
| pair-003 | standard_better | 16 | 16 | 53 | taskspace_overhead_timeout; validator_slow_or_flaky; node_overfragmentation; subagent_noise_or_unused |
| pair-004 | standard_better | 2 | 1 | 17 | taskspace_overhead_timeout; validator_slow_or_flaky |

这说明 timeout 不能统一归因为“图过大”。0.0.4 需要 failure taxonomy 区分 graph overhead、validator 噪声、patch 错误、验证循环等不同根因。

### 3.5 processing-pipeline 暴露图活跃但决策密度不足

`processing-pipeline` 有较高 graph activity：59 nodes、103 edges、357 detailed results，TaskSpace 2/5，Standard 3/5。失败分类多次出现 `subagent_noise_or_unused`、`agent_patch_wrong`、`node_overfragmentation`。

这说明：

```text
更多节点 / 更多边 / 更多 result / 更多 subagent 并不自动转化为更好 patch。
```

0.0.4 应衡量 decision density 和 result adoption，而不是只统计结构活动量。

## 4. 问题归因

| 问题 | 0.0.3 表现 | 0.0.4 对策 |
|---|---|---|
| 完成标准缺失 | successCriteria 未被稳定使用 | ProblemStateLedgerV1 + start_task gate |
| result 未采信 | unreviewed result 比例高，决策引用缺失 | ResultAdoptionV1 + dependency refs |
| node 语义不清 | inspect_code_context 过载 | TypedNodeKindContractV1 |
| graph 成本不可解释 | nodes/results 膨胀但 utility 未升 | GraphHealthReportOnly |
| subagent ROI 不明 | spawn/result 是否改变 decision 不可查 | SubagentContractV1 |
| timeout 归因混杂 | validator / TaskSpace overhead / agent error 混在一起 | FailureTaxonomyV1 |
| clean utility 缺失 | valid_utility_pairs=0 | CleanE3AuditManifest |

## 5. 0.0.4 设计输入

0.0.4 设计应围绕以下问题建立 contract：

```text
1. 当前任务完成标准是什么？
2. 当前已验证事实是什么？
3. 当前尚未回答的问题是什么？
4. 当前假设及其证据是什么？
5. 当前 patch/design/validation decision 依赖哪些 result？
6. 哪些 result 被 accepted/questioned/invalid？
7. 哪些 node 已经 stale 或未贡献 decision？
8. 当前 run 是否具备 clean audit inclusion 条件？
```
