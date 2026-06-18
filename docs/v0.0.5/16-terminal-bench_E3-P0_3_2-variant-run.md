# v0.0.5 terminal-bench_E3-P0_3_2 诊断变体运行记录

## 结论

本次执行的是 `terminal-bench_E3-P0_3_2`，即 `terminal-bench_E3-P0_3_5` 的低成本诊断变体，`repeats=2`。它不是正式 E3，不能用于声明 v0.0.5 的正式 Terminal-Bench E3 正确率。

核心结果：

- 计划范围：3 个 P0 样本，每个 2 个 repeat，理论 6 个 Standard/TaskSpace pair。
- 实际完成：5 个 pair。
- 中止样本：`multi-source-data-merger` 在 `pair-001` 后触发 `score_validity` abort，后续 repeat 被跳过。
- Standard 成功：4/5。
- TaskSpace 成功：3/5。
- Standard agent wall time 合计：833,017 ms。
- TaskSpace agent wall time 合计：3,048,411 ms。
- Standard token 合计：3,106,038。
- TaskSpace token 合计：35,376,825。

本轮低成本诊断没有显示 v0.0.5 TaskSpace 相比 Standard 的正确率优势；相反，在已执行 pair 上 TaskSpace 正确率更低、时间和 token 成本显著更高。

## 运行配置

| 项目 | 值 |
|---|---|
| 实验名称 | `terminal-bench_E3-P0_3_2` |
| 版本语境 | v0.0.5 |
| benchmark | Terminal-Bench |
| 子集 | E3-P0 comparable |
| samples | 3 |
| repeats | 2 |
| source version | `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b` |
| source root | `C:\w\terminal-bench-1a6ffa9674b571da0ed040c470cb40c4d85f9b9b\original-tasks` |
| run root | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant` |
| runner | `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` |
| diagnostic flag | `-AllowDiagnosticNonTargetResult` |
| scoring flag | `-ScoringMode` |
| model | `deepseek-v4-flash` |
| sandbox | `full-auto` |

正式 `run-taskspace-e3-*` 入口要求 `repeats >= 5`。因此本次没有走正式 E3 入口，而是走 external benchmark runner 的诊断路径。

## 执行结果总表

| sample | pair | Standard success | TaskSpace success | Standard agent ms | TaskSpace agent ms | Standard tokens | TaskSpace tokens | pair total ms | validation ms | bottleneck |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `processing-pipeline` | 001 | true | true | 199,823 | 715,257 | 687,253 | 7,415,722 | 1,151,962 | 184,127 | agent_bound |
| `processing-pipeline` | 002 | true | true | 421,056 | 642,131 | 1,932,370 | 7,154,561 | 1,296,085 | 194,330 | agent_bound |
| `multi-source-data-merger` | 001 | false | false | 93,651 | 878,250 | 178,289 | 11,516,871 | 1,247,650 | 241,590 | engineering_unclean_slow |
| `recover-accuracy-log` | 001 | true | false | 60,658 | 613,674 | 173,329 | 8,116,214 | 1,104,584 | 382,910 | validator_bound |
| `recover-accuracy-log` | 002 | true | true | 57,829 | 199,099 | 134,797 | 1,173,457 | 604,895 | 307,447 | validator_bound |

## 样本状态

| sample | requested pairs | completed pairs | exit | phase | run validity | 说明 |
|---|---:|---:|---:|---|---|---|
| `processing-pipeline` | 2 | 2 | 0 | audit_required | valid | 2 个 pair 均完成，但因为 repeats 不足和未完成人审，不进入正式 aggregate。 |
| `multi-source-data-merger` | 2 | 1 | 3 | invalid_harness | invalid_harness | pair-001 后触发 `score_validity` abort；原因包含 validator fidelity / eligibility、tests marker、public validation timeout。 |
| `recover-accuracy-log` | 2 | 2 | 0 | audit_required | valid | 2 个 pair 完成；1 个 TaskSpace 失败，1 个双方成功。 |

`multi-source-data-merger` abort artifact：

`D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260619-014722-005\pair-001\pair-abort.json`

abort reasons：

- `e3_external_validator_fidelity_unproven`
- `e3_external_validator_not_e3_eligible`
- `no_tests_started_marker`
- `public_validation_timeout`

## 成功率与成本汇总

| mode | executed runs | success | success rate | agent wall ms | agent wall min | total tokens | input tokens | output tokens |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 5 | 4 | 80.0% | 833,017 | 13.88 | 3,106,038 | 3,052,762 | 53,276 |
| TaskSpace | 5 | 3 | 60.0% | 3,048,411 | 50.81 | 35,376,825 | 35,139,365 | 237,460 |

TaskSpace 相对 Standard：

- 成功数少 1 个。
- agent wall time 约为 Standard 的 3.66 倍。
- token 约为 Standard 的 11.39 倍。

## 证据等级说明

本次结果只能作为诊断证据，原因：

- `repeats=2`，低于正式 E3 的 `repeats>=5` 门槛。
- `processing-pipeline` 和 `recover-accuracy-log` 的 pair 报告均有 `e3_repeats_lt_5` 和 `e3_human_review_not_completed`。
- aggregate 明确显示 `score_ready=False`、`score_valid=False`，score fields disabled。
- `multi-source-data-merger` 为 invalid_harness，不能纳入正确率结论。

因此，本轮只能回答“v0.0.5 在低成本 Terminal-Bench P0 诊断变体上是否通畅、成本表现如何”，不能回答“v0.0.5 正式 E3 正确率是否未下降”。

## Artifact 索引

| artifact | 路径 |
|---|---|
| driver summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\driver-summary.json` |
| parsed pair summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\pair-summary.json` |
| parsed metrics summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\metrics-summary.json` |
| processing aggregate | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\processing-pipeline\runs\terminal_bench__processing-pipeline\20260619-010610-484\aggregate-report.md` |
| merger abort | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260619-014722-005\pair-001\pair-abort.json` |
| recover aggregate | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_2-v005-20260619-variant\recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260619-020820-831\aggregate-report.md` |

## 反思

1. 低 repeat 诊断适合快速发现工程通路和成本异常，但不能替代正式 E3。
2. `multi-source-data-merger` 的 abort 说明正式跑前仍需要先处理 validator eligibility / lifecycle marker / timeout 类 harness 问题，否则会中途损失样本覆盖。
3. 本轮 TaskSpace token 放大非常明显，尤其 `multi-source-data-merger` 单 pair TaskSpace 达到 11,516,871 tokens，成本优化不应只看 wall time。
4. 未来文档和命令输出必须始终写清实验名称、sample 数、repeat 数、是否正式 E3，避免再次把诊断变体当作正式 E3 结论。
