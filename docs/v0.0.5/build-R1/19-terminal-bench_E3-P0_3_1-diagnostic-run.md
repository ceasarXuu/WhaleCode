# v0.0.5 terminal-bench_E3-P0_3_1 诊断运行记录

## 结论

本次执行的是 `terminal-bench_E3-P0_3_1`，即 `terminal-bench_E3-P0_3_5` 的低成本诊断变体：3 个 P0 可比样本，每个样本 1 个 Standard/TaskSpace pair。

这不是正式 E3，不能用于声明 v0.0.5 的正式 Terminal-Bench 正确率，也不能用于发布通过判断。

关键结果：

- reported_evidence_level: `diagnostic-only`
- requested_evidence_target: `E3`
- not_release_proof: `true`
- formal_sample_set_id: `terminal-bench_E3-P0_3_5`
- 实际执行：3 个 pair
- score_valid: `false`
- engineering_clean: `false`
- Standard raw success: 2/3
- TaskSpace raw success: 0/3
- Standard agent wall time: 483,015 ms
- TaskSpace agent wall time: 1,752,094 ms
- Standard total tokens: 1,480,481
- TaskSpace total tokens: 24,118,773
- TaskSpace / Standard time: 3.63x
- TaskSpace / Standard tokens: 16.29x

本轮诊断没有显示 v0.0.5 TaskSpace 在 Terminal-Bench P0 上的正确率或成本优势。相反，在 3 个 raw pair 中 TaskSpace 全部失败，时间和 token 成本仍显著高于 Standard。

## 运行配置

| 项目 | 值 |
|---|---|
| 实验名称 | `terminal-bench_E3-P0_3_1` |
| 版本语境 | v0.0.5 |
| benchmark | Terminal-Bench |
| 子集 | E3-P0 comparable |
| samples | 3 |
| repeats | 1 |
| source version | `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b` |
| source root | `C:\w\terminal-bench-1a6ffa9674b571da0ed040c470cb40c4d85f9b9b\original-tasks` |
| run root | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4` |
| runner | `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` |
| diagnostic flag | `-AllowDiagnosticNonTargetResult` |
| scoring flag | `-ScoringMode` |
| model | `deepseek-v4-flash` |
| sandbox | `full-auto` |
| whale | `whale 0.1.0` |
| repo commit | `25fe8a9eeafacd286cdf791f35e412681f65621f` |

正式 `run-taskspace-e3-suite.ps1` 入口要求 `Repeats >= 5`。因此本次 `_3_1` 没有走正式 E3 suite 入口，而是逐样本走 external benchmark runner 的诊断路径。

## 每个 pair 结果

| sample | pair | Standard success | Standard agent ms | Standard tokens | Standard requests | TaskSpace success | TaskSpace agent ms | TaskSpace tokens | TaskSpace rollout requests | TS/Std time | TS/Std token |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `processing-pipeline` | 001 | true | 284,673 | 998,960 | 1 | false | 289,977 | 2,390,331 | 66 | 1.02x | 2.39x |
| `multi-source-data-merger` | 001 | false | 110,102 | 256,494 | 1 | false | 787,066 | 12,344,100 | 189 | 7.15x | 48.13x |
| `recover-accuracy-log` | 001 | true | 88,240 | 225,027 | 1 | false | 675,051 | 9,384,342 | 180 | 7.65x | 41.70x |

## 汇总表

| mode | executed runs | raw success | raw success rate | agent wall ms | agent wall min | total tokens | model requests | vs Standard time | vs Standard token |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3 | 2 | 66.7% | 483,015 | 8.05 | 1,480,481 | 3 | 1.00x | 1.00x |
| TaskSpace | 3 | 0 | 0.0% | 1,752,094 | 29.20 | 24,118,773 | 435 rollout requests | 3.63x | 16.29x |

说明：TaskSpace 的顶层 `whale exec` 也是每个 pair 1 次，但真实成本应按 `rollout_trace.model_request_count` 看内部模型请求。本轮 3 个 TaskSpace pair 的 rollout requests 合计为 435 次。

## 样本状态

| sample | runner exit | run_validity | score_valid | 说明 |
|---|---:|---|---|---|
| `processing-pipeline` | 0 | valid | false | Standard raw success，TaskSpace raw failure；因 `repeats=1` 和审计未闭环，score disabled。 |
| `multi-source-data-merger` | 3 | invalid_harness | false | pair 后触发 `score_validity` abort；原因包括 docker build environment failure、validator fidelity/eligibility、tests marker、public validation timeout。 |
| `recover-accuracy-log` | 0 | valid | false | Standard raw success，TaskSpace raw failure；因 `repeats=1` 和审计未闭环，score disabled。 |

`multi-source-data-merger` abort artifact:

```text
D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260619-191222-602\pair-001\pair-abort.json
```

abort reasons:

- `docker_build_environment_failure`
- `e3_external_validator_fidelity_unproven`
- `e3_external_validator_not_e3_eligible`
- `no_tests_started_marker`
- `public_validation_timeout`

## 与上次 terminal-bench_E3-P0_3_2 诊断对比

| 指标 | `_3_2` 诊断变体 | `_3_1` 本次诊断 | 变化 |
|---|---:|---:|---|
| completed pairs | 5 | 3 | 更少 |
| Standard raw success | 4/5 | 2/3 | 不可直接统计等价，但仍有成功样本 |
| TaskSpace raw success | 3/5 | 0/3 | 明显更差 |
| Standard agent wall ms | 833,017 | 483,015 | 本次样本/重复更少 |
| TaskSpace agent wall ms | 3,048,411 | 1,752,094 | 本次样本/重复更少 |
| TS/Std time | 3.66x | 3.63x | 基本相同 |
| Standard tokens | 3,106,038 | 1,480,481 | 本次样本/重复更少 |
| TaskSpace tokens | 35,376,825 | 24,118,773 | 本次样本/重复更少但仍极高 |
| TS/Std token | 11.39x | 16.29x | 更差 |

本次 `_3_1` 说明，v0.0.5 的成本控制改动尚未稳定兑现为 Terminal-Bench P0 raw 效果。请求次数确实比上次 `_3_2` 的若干样本有所下降，但 token 倍率仍然失控，且 raw 成功率退化。

## 工程归因

本轮可确认有效的部分：

- `state_commit` runtime 化、context projection、map management 已经有观测产物，且 TaskSpace side 没有再次出现最终 map 结构爆炸。
- `processing-pipeline` 的 TaskSpace rollout requests 为 66，明显低于上次 `_3_2` 中 142/143 的同样本级别；这说明部分预算/状态压缩建设对请求次数有一定抑制。
- start/release gate 口径正确地阻止了把本轮结果误报成正式 E3。

本轮仍无效或不足的部分：

- 成本控制没有达到产品目标。TaskSpace 总 token 仍是 Standard 的 16.29x，`multi-source-data-merger` 单 pair 达到 48.13x，`recover-accuracy-log` 达到 41.70x。
- 正确率没有守住。TaskSpace raw success 为 0/3，Standard 为 2/3。
- rollout request 数仍然偏高，3 个 TaskSpace pair 合计 435 次内部模型请求。
- 成本放大不只来自请求次数，也来自每次请求仍携带较大的 provider-visible context；例如 `recover-accuracy-log` 180 次 rollout requests 累积到 9,384,342 tokens。
- `multi-source-data-merger` 仍暴露 harness/validator 层工程不干净问题，导致 run_validity 为 `invalid_harness`。

## 证据等级说明

本轮只能作为低成本诊断证据，原因：

- `repeats=1`，低于正式 E3 的 `repeats>=5` 门槛。
- `score_valid=false`，score fields disabled。
- `multi-source-data-merger` 为 `invalid_harness`。
- 没有完成正式 E3 人工审计闭环。
- 本轮通过 external benchmark runner 逐样本运行，而不是正式 E3 suite 入口。

允许结论：

- v0.0.5 当前代码可以启动并完成 `terminal-bench_E3-P0_3_1` 诊断路径。
- 当前 TaskSpace 在该低成本诊断变体上 raw success 与成本表现不达标。
- 下一步不应进入正式 `terminal-bench_E3-P0_3_5` 收口，应先继续修复 TaskSpace 成本与 raw success。

禁止结论：

- 不能声明 v0.0.5 正式 E3 通过。
- 不能声明 v0.0.5 正确率未下降已经坐实。
- 不能把本轮 0/3 或 2/3 泛化为完整 Terminal-Bench 产品指标。

## Artifact 索引

| artifact | 路径 |
|---|---|
| driver summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\driver-summary.json` |
| parsed pair summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\pair-summary.json` |
| parsed metrics summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\metrics-summary.json` |
| processing pair report | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\processing-pipeline\runs\terminal_bench__processing-pipeline\20260619-185744-707\pair-001\pair-report.md` |
| merger abort | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260619-191222-602\pair-001\pair-abort.json` |
| recover pair report | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260619-193200-574\pair-001\pair-report.md` |

## 反思

1. `_3_1` 诊断足够暴露当前状态：TaskSpace 成本控制还没有达到 v0.0.5 目标，且 raw success 在本轮低样本上不可接受。
2. 不能因为非 agent gates 通过就进入正式 E3；正式 E3 前还需要先让低成本诊断至少不出现 0/3 raw success 和 16x token 倍率。
3. 下一步工程重点应从“可观测与防误报”转向“硬预算生效、请求策略收敛、provider-visible context 限额、失败样本专门修复”。
4. `multi-source-data-merger` 的 harness/validator 问题需要单独清理，否则正式 P0 样本覆盖会被中途破坏。
