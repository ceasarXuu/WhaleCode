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
