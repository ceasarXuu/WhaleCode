# TaskSpace Full Benchmark Run 记录

日期：2026-06-03

## 结论摘要

本轮跑了两类评估：

1. Terminal-Bench 外部真实样本：4 个样本，每个 5 次 `standard` / `taskspace` paired run，共 20 个 pair。
2. 内置 E2 matrix：3 个自建场景，每个 3 次 paired run，共 9 个 pair。

结论：

- 外部 Terminal-Bench：当前 TaskSpace 没有表现出收益。20 个 pair 中 14 个达到 E3 可审计证据，6 个双方失败或不可比较；E3 pair 里 `taskspace_better = 0`，`standard_better = 5`，`no_clear_delta = 9`。
- 内置 E2 matrix：机制层通过，`e2_evidence_readiness = True`，`e2_clean_readiness = True`；但 `e2_utility_clean_readiness = False`，主要因为 L2 场景有 2 个 pair 出现 TaskSpace 成本偏高。
- TaskSpace 路径确实创建了 map/node，说明运行约束和观测链路在工作；但在这些偏短、偏文件处理的任务中，map/node 生长没有转化成更高成功率，反而在 `jsonl-aggregator` 上出现明显过度生长和成本放大。

E3 负收益后的系统性复盘与下一阶段重构基线见：
[TaskSpace E3 负收益后问题状态与模型管理重构基线](../plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md)。

该复盘把当前问题重新定义为：TaskSpace 已经完成“agent 必须绑定 task/map/node 行动”，但尚未完成“主 agent 以问题状态与模型管理者身份，维护事实、假设、证据、契约、决策和不确定性”。因此后续优化重点不再是继续堆叠 gate，而是把 map 从行动台账升级为问题状态模型，把主 agent 从一线执行者升级为认知控制器和验收责任人。

## 环境

- repo：`D:\whalecode-alpha`
- branch：`whalecode-alpha`
- whale：`whale 0.1.0`
- whale sha256：`24F0BFE16185473BC9EE3D3AD8F22E3D11AF1CE061537DC8B30196F5DF7E19BF`
- Docker Server：`29.1.3`
- model：`deepseek-v4-flash`
- sandbox：`full-auto`
- reasoning effort：`max`
- Terminal-Bench source revision：`1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`

## 无效首跑

首跑目录：

`D:\whalecode-alpha\target\taskspace-full-benchmark-20260603-182319`

该 run 无效。原因是我把 run root 命名为 `taskspace-full-benchmark-*`，触发了 benchmark harness 的 neutral cwd 校验。该校验会拒绝路径中出现 `taskspace`、`map`、`node`、`subagent` 等内部概念，防止路径污染 agent 输入。

典型错误：

```text
Non-neutral cwd for left: ...\target\taskspace-full-benchmark-...\pair-001\left\repo
```

纠正方式：有效 run 使用不含内部概念的路径 `target\benchfull-*` 和 `target\benchmx-*`。

## 有效外部 E3 Run

Run root：

`D:\whalecode-alpha\target\benchfull-20260603-182527`

执行范围：

| sample | source task dir | repeats | pair 数 |
|---|---|---:|---:|
| `hello-world` | `%TEMP%\whale-real-external-benchmarks\terminal-bench\original-tasks\hello-world` | 5 | 5 |
| `heterogeneous-dates` | `%TEMP%\whale-real-external-benchmarks\terminal-bench\original-tasks\heterogeneous-dates` | 5 | 5 |
| `jsonl-aggregator` | `%TEMP%\whale-real-external-benchmarks\terminal-bench\original-tasks\jsonl-aggregator` | 5 | 5 |
| `log-summary` | `%TEMP%\whale-real-external-benchmarks\terminal-bench\original-tasks\log-summary` | 5 | 5 |

关键产物：

- 执行日志：`D:\whalecode-alpha\target\benchfull-20260603-182527\execution-process.log`
- audit/finalize 补跑日志：`D:\whalecode-alpha\target\benchfull-20260603-182527\audit-finalize-process.log`
- `hello-world` aggregate：`D:\whalecode-alpha\target\benchfull-20260603-182527\runs\terminal_bench__hello-world\20260603-182531-589\aggregate-report.md`
- `heterogeneous-dates` aggregate：`D:\whalecode-alpha\target\benchfull-20260603-182527\runs\terminal_bench__heterogeneous-dates\20260603-190939-938\aggregate-report.md`
- `jsonl-aggregator` aggregate：`D:\whalecode-alpha\target\benchfull-20260603-182527\runs\terminal_bench__jsonl-aggregator\20260603-200843-895\aggregate-report.md`
- `log-summary` aggregate：`D:\whalecode-alpha\target\benchfull-20260603-182527\runs\terminal_bench__log-summary\20260603-211545-277\aggregate-report.md`

### 外部 E3 结果

| sample | all pairs | valid E3 pairs | excluded | review decisions |
|---|---:|---:|---:|---|
| `hello-world` | 5 | 4 | 1 | `include_standard_better=1; include_no_clear_delta=3` |
| `heterogeneous-dates` | 5 | 3 | 2 | `include_standard_better=1; include_no_clear_delta=2` |
| `jsonl-aggregator` | 5 | 5 | 0 | `include_standard_better=2; include_no_clear_delta=3` |
| `log-summary` | 5 | 2 | 3 | `include_standard_better=1; include_no_clear_delta=1` |

汇总：

| 指标 | 数值 |
|---|---:|
| 外部 pair 总数 | 20 |
| valid E3 pairs | 14 |
| excluded / E1 pairs | 6 |
| `include_taskspace_better` | 0 |
| `include_standard_better` | 5 |
| `include_no_clear_delta` | 9 |

### 成功率和成本

| sample | mode | runs | success | avg tools | avg nodes | avg wall seconds |
|---|---|---:|---:|---:|---:|---:|
| `hello-world` | standard | 5 | 4 | 1.2 | 0 | 12.26 |
| `hello-world` | taskspace | 5 | 3 | 1.8 | 1.0 | 23.09 |
| `heterogeneous-dates` | standard | 5 | 3 | 7.4 | 0 | 19.36 |
| `heterogeneous-dates` | taskspace | 5 | 2 | 10.0 | 3.6 | 63.73 |
| `jsonl-aggregator` | standard | 5 | 5 | 11.4 | 0 | 50.89 |
| `jsonl-aggregator` | taskspace | 5 | 3 | 40.2 | 13.2 | 337.27 |
| `log-summary` | standard | 5 | 2 | 4.0 | 0 | 26.66 |
| `log-summary` | taskspace | 5 | 1 | 3.8 | 2.2 | 22.15 |

观察：

- `jsonl-aggregator` 是最关键负例：TaskSpace 平均 node 数 13.2，平均 wall time 337.27 秒，成功率 3/5；standard 成功率 5/5，平均 wall time 50.89 秒。
- `hello-world` 这类极短任务中 TaskSpace 仍会创建 1 个 node，成本高于 standard，并出现 1 次 taskspace validator 失败。
- `log-summary` 两边都不稳定，excluded pair 达到 3/5，说明任务或模型执行本身也存在波动；但在可比较 pair 中仍没有看到 TaskSpace 优势。

## 内置 E2 Matrix

Run root：

`D:\whalecode-alpha\target\benchmx-20260603-232852`

报告：

`D:\whalecode-alpha\target\benchmx-20260603-232852\e2-matrix-report.md`

结果：

```text
e2_evidence_readiness: True
e2_clean_readiness: True
e2_utility_clean_readiness: False
```

| scenario | level | valid pairs | utility warning pairs | outcomes |
|---|---|---:|---:|---|
| `single-file-fast-fix` | L1 | 3 | 0 | `both_success_cost_within_budget=3` |
| `multi-file-order-pipeline` | L2 | 3 | 2 | `both_success_cost_within_budget=1; both_success_taskspace_cost_higher=2` |
| `subscription-billing-repair` | L3 | 3 | 0 | `both_success_cost_within_budget=3` |

解读：

- E2 机制层是健康的：没有 evidence blocking gap，也没有 mechanism warning gap。
- E2 效用层不是 clean：`multi-file-order-pipeline` 有 2 个 pair 出现 TaskSpace 成本偏高。
- 这和外部 benchmark 的结论方向一致：TaskSpace 的基础机制能跑，但调度/拆解策略还没有稳定转化为效用。

## 执行过程问题

1. 无效首跑暴露了执行脚本使用者容易踩的路径命名问题。run root 不得包含 `taskspace`、`map`、`node`、`subagent` 等词。
2. 外部完整 run 耗时超过 5 小时，超过单次 shell 工具超时后，外层 wrapper 被截断，但子 run 已经落盘完成。后续通过 artifact 目录补写 audit review 并 finalize。
3. 第一次自动写 audit 使用 `Start-Process -ArgumentList` 传含空格字符串，PowerShell 拆词导致失败；后续改用直接调用脚本完成。
4. `validation_exit=124` 出现在若干 pair，但 `exec_timed_out=False`，说明超时主要发生在 validator 阶段，不是 agent exec 阶段。

## 当前水平判断

当前 TaskSpace 已经达到“机制可运行、可观测、可审计”的状态，但还没有达到“外部 benchmark 上有稳定产品收益”的状态。

从这轮数据看，优先修复方向不是继续扩 benchmark 数量，而是改进 TaskSpace 的调度策略：

- 简短任务必须更克制，避免为了建图而建图。
- 中等数据处理任务需要防止 map/node 过度生长，尤其是 `jsonl-aggregator` 这种已经暴露出高成本低收益的样本。
- 主 agent 的 task/map 驱动力需要从“完成形式上的绑定”升级为“用 map 降低重复探索和错误路径”，否则 node 数增长只会变成成本放大器。
- validator 超时和双方失败样本需要单独归因，避免把环境/验证波动误判成 TaskSpace 或 standard 的能力差异。

下一轮建议以 `jsonl-aggregator` 和 `multi-file-order-pipeline` 为主要回归样本，因为它们分别暴露了外部真实任务上的过度生长，以及内置 E2 上的效用成本警告。
