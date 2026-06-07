# TaskSpace E3 Run Index

本文档记录每轮 E3 执行情况，避免不同轮次的范围、结论和 artifact 路径混淆。

## 记录规则

- `时间` 使用本机执行日期。
- `范围` 必须写清样本集合、每个样本 repeat 数、pair 总数。
- `结论` 只写简洁判断；详细分析放到独立复盘文档。
- `状态` 使用 `completed`、`partial`、`invalid`、`running`。
- `索引` 必须指向 run root、aggregate report 或复盘文档。
- 若 run root、scenario path、prompt 暴露了 `taskspace`、`map`、`node`、`subagent` 等内部概念，应标记为 `invalid`。

## 执行记录

| 时间 | 状态 | 范围 | 结果简洁 | 索引 |
|---|---|---|---|---|
| 2026-06-03 | completed | Terminal-Bench 4 samples x 5 repeats：`hello-world`、`heterogeneous-dates`、`jsonl-aggregator`、`log-summary`；20 pairs。另有内置 E2 matrix 3 samples x 3 repeats。 | TaskSpace 机制可运行，但外部 E3 无正收益证据；`jsonl-aggregator` 暴露 node 过度生长和成本放大。 | [2026-06-03 full benchmark](./2026-06-03-taskspace-full-benchmark-run.md)；run root: `D:\whalecode-alpha\target\benchfull-20260603-182527` |
| 2026-06-06 | partial | Terminal-Bench `hello-world` only x 5 repeats；5 pairs。 | 只能作为 `hello-world` 局部信号；不能与 2026-06-03 完整 E3 直接整体对比。暴露 PowerShell 编码、final gate 恢复循环、validator timeout 噪声。 | run root: `D:\whalecode-alpha\target\e3-full-20260606-014919` |
| 2026-06-06 | completed | 复跑与 2026-06-03 一致的 Terminal-Bench 4 samples x 5 repeats：`hello-world`、`heterogeneous-dates`、`jsonl-aggregator`、`log-summary`；20 pairs。 | 有效 E3 16/20，excluded/E1 4/20；`include_taskspace_better=5`、`include_standard_better=3`、`include_no_clear_delta=8`。`jsonl-aggregator` 仍是 Standard better 2、TaskSpace better 0；`log-summary` 出现 TaskSpace better 4。 | run root: `D:\whalecode-alpha\target\benchfull-20260606-035046`；aggregate：`hello-world` `D:\whalecode-alpha\target\benchfull-20260606-035046\runs\terminal_bench__hello-world\20260606-035051-020\aggregate-report.md`；`heterogeneous-dates` `D:\whalecode-alpha\target\benchfull-20260606-035046\runs\terminal_bench__heterogeneous-dates\20260606-044907-794\aggregate-report.md`；`jsonl-aggregator` `D:\whalecode-alpha\target\benchfull-20260606-035046\runs\terminal_bench__jsonl-aggregator\20260606-055232-352\aggregate-report.md`；`log-summary` `D:\whalecode-alpha\target\benchfull-20260606-035046\runs\terminal_bench__log-summary\20260606-065839-046\aggregate-report.md` |
| 2026-06-07 | partial | Terminal-Bench P0 candidate 4 samples x 5 repeats：`processing-pipeline`、`multi-source-data-merger`、`recover-accuracy-log`、`query-optimize`；计划 20 pairs，实际完成 `processing-pipeline` 5 pairs、`multi-source-data-merger` 5 pairs、`query-optimize` 1 partial pair；`recover-accuracy-log` 未进入 agent 执行。 | 这是 P0 适配性试跑，不是有效 E3 结论：`processing-pipeline` 5/5 生成 E3-candidate 但未做 human audit；`multi-source-data-merger` 5/5 均被排除，其中 2 个 E1、3 个 E2-candidate 但需人工审查；`recover-accuracy-log` 被 prompt guard 对任务领域词 `multi-agent` 误杀；`query-optimize` 标准侧 900s 无输出超时，TaskSpace 侧有输出但 metrics 提取因 `oewn.sqlite` 文件锁失败。 | run root: `D:\whalecode-alpha\target\benchp0-20260607-014707`；aggregate：`processing-pipeline` `D:\whalecode-alpha\target\benchp0-20260607-014707\runs\terminal_bench__processing-pipeline\20260607-014712-519\aggregate-report.md`；`multi-source-data-merger` `D:\whalecode-alpha\target\benchp0-20260607-014707\runs\terminal_bench__multi-source-data-merger\20260607-031023-681\aggregate-report.md`；failure logs：`sample-recover-accuracy-log.err.log`、`sample-query-optimize.err.log` |
| 2026-06-07 | partial | 修复 remote asset preflight 后重跑 Terminal-Bench P0 candidate 4 samples x 5 repeats：`processing-pipeline`、`multi-source-data-merger`、`recover-accuracy-log`、`query-optimize`；计划 20 pairs，实际完成 15 pairs，`query-optimize` 0 pairs。 | 工程环境可跑，fail-closed 生效，但不是 clean E3：0 pair 进入 utility aggregate；诊断性结果为 TaskSpace better 3、Standard better 4、both success 4、both failed/inconclusive 4；`query-optimize` 因 HuggingFace `oewn.sqlite` 远程资产未证明等价被阻止。 | [2026-06-07 P0 rerun](./2026-06-07-taskspace-p0-rerun-after-asset-preflight-fix.md)；run root: `D:\whalecode-alpha\target\benchp0-20260607-070527`；aggregate：`processing-pipeline` `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__processing-pipeline\20260607-070532-088\aggregate-report.md`；`multi-source-data-merger` `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__multi-source-data-merger\20260607-082405-434\aggregate-report.md`；`recover-accuracy-log` `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__recover-accuracy-log\20260607-100434-377\aggregate-report.md` |
