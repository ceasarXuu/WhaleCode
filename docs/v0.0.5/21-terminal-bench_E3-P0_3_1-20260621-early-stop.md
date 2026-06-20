# v0.0.5 terminal-bench_E3-P0_3_1 诊断早停记录

## 结论

本次按 `terminal-bench_E3-P0_3_1` 低成本诊断口径启动，但只执行了第一个样本 `processing-pipeline`。执行中发现 TaskSpace 侧明确超时，因此按“中间发现问题立刻停下来不要继续浪费时间”的要求停止，未继续执行 `multi-source-data-merger` 和 `recover-accuracy-log`。

这不是正式 E3，也不是完整 `_3_1` 结果。

## 运行信息

| 项目 | 值 |
|---|---|
| 当前 commit | `f3d4d45e94` |
| 样本集目标 | `terminal-bench_E3-P0_3_1` |
| 实际执行样本 | `processing-pipeline` |
| repeats | 1 |
| source version | `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b` |
| source root | `C:\w\terminal-bench-1a6ffa9674b571da0ed040c470cb40c4d85f9b9b\original-tasks` |
| run root | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260621-diagnostic` |
| runner | `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` |
| evidence level | diagnostic-only / early-stop |

## 停止原因

`processing-pipeline` pair 报告显示：

| mode | business_success | exec_exit_code | exec_timed_out | wall_time_ms | outcome |
|---|---:|---:|---:|---:|---|
| Standard | true | 0 | false | 409,997 | solved |
| TaskSpace | false | 124 | true | 900,029 | agent_exec_timeout |

关键失败信息：

- `failure_taxonomy`: `taskspace_overhead_timeout, subagent_noise_or_unused, audit_unclean`
- TaskSpace 右侧 validation 未开始：`right_tests_started_seen=False`
- TaskSpace map/node 膨胀仍存在：`maps=1`、`nodes=15`、`spawn_agent_calls=2`、`subagent_results=10`、`open_leaf_nodes=3`
- TaskSpace 侧 `rollout_trace` 显示 `model_request_count=191`
- TaskSpace 侧 rollout tokens：`input_tokens=11,493,455`、`output_tokens=49,582`、`cached_input_tokens=9,212,032`

## 与上一轮同样本对比

上一轮 `2026-06-19` `_3_1` 中 `processing-pipeline` 的 TaskSpace 侧也是失败，但耗时为 289,977 ms，rollout requests 为 66。

本次同样本：

- TaskSpace wall time 从约 290s 上升到 900s，并触发 agent timeout。
- rollout requests 从 66 上升到 191。
- 成本和耗时没有显示修复改善，反而在该样本上明显恶化。

## Artifact

| artifact | 路径 |
|---|---|
| pair report | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260621-diagnostic\processing-pipeline\runs\terminal_bench__processing-pipeline\20260621-000213-625\pair-001\pair-report.md` |
| pair timing | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260621-diagnostic\processing-pipeline\runs\terminal_bench__processing-pipeline\20260621-000213-625\pair-001\pair-timing.json` |
| TaskSpace request summary | `D:\whalecode-alpha\target\terminal-bench_E3-P0_3_1-v005-20260621-diagnostic\processing-pipeline\runs\terminal_bench__processing-pipeline\20260621-000213-625\pair-001\right\artifacts\request-summary.json` |

## 判断

当前修复不足以支撑继续跑完整 `terminal-bench_E3-P0_3_1`，更不应进入正式 `terminal-bench_E3-P0_3_5`。下一步应先定位 `processing-pipeline` 上 TaskSpace 为什么从 66 requests 回退到 191 requests，并处理超时前的 map/node/spawn 扩张。
