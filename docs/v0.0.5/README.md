# TaskSpace v0.0.5 工程设计文档包

- 版本：v0.0.5 continuation
- 日期：2026-06-19
- 状态：继续开发中，不能关闭版本
- 主题：Protocol Compaction、Map Self-Management、主动成本控制与正式 E3 治理

## 当前状态

v0.0.5 尚未收口，当前状态为继续开发。

2026-06-19 的 `terminal-bench_E3-P0_3_2` 诊断变体显示，TaskSpace 在 P0 comparable 样本上的成本和正确率仍未达到 v0.0.5 目标：TaskSpace 成功数 `3/5`，Standard 成功数 `4/5`，TaskSpace agent wall time 为 `3.66x`，token 为 `11.39x`。因此，早先的收口总结只能作为历史阶段性记录，不能作为当前版本关闭依据。

## 当前规范入口

当前开发、验收和 E3 准入必须以以下文件为入口：

```text
17-unfinished-work-inventory.md
18-unfinished-work-engineering-design.md
```

其中 `18-unfinished-work-engineering-design.md` 是 v0.0.5 未完成项继续开发的 canonical execution entry。若它与 `10-implementation-plan.md`、`13-design-corrections-and-engineering-contract.md`、`09-e3-validation-plan.md`、`checklists/acceptance-checklist.md` 或 `TaskSpace-v0.0.5-Design-All-in-One.md` 在 Phase 6 样本安排、release taxonomy、report-only routing、formal E3 准入、成本门槛或收口规则上冲突，以 `18-unfinished-work-engineering-design.md` 为准。

`10-implementation-plan.md` 保留为历史 corrected plan 和早期阶段背景，不得单独作为当前 Phase 6 或 release closeout 执行依据。

## 禁止误用

- 代码实际完成、非 agent gates 通过、code-complete marker 有效、用户 approval marker 绑定当前 sample set 之前，禁止运行真实 E3。
- `_1_1`、`_3_1`、`_3_2` 等低成本变体只能是 `diagnostic-only` 或 `E3-candidate`，不能支撑 `release_pass`。
- `terminal-bench_E3-v004-clean_3_5` 只能用于与 v0.0.4 clean 15-run 做同口径正确率回归对比，不能替代 v0.0.5 P0 release proof。
- v0.0.5 正式 P0 收口证明必须使用 `terminal-bench_E3-P0_3_5`，且必须满足实验制度和 release/start gate。

## 文档目录

| 文件 | 用途 |
|---|---|
| `00-executive-summary.md` | 设计摘要、目标、非目标、版本边界 |
| `01-evidence-and-root-cause.md` | v0.0.4 根因输入与设计约束 |
| `02-system-design-overview.md` | v0.0.5 总体架构变化 |
| `03-protocol-compaction.md` | 批量状态提交、减少 `taskspace_control` 轮次 |
| `04-context-projection-and-replay-control.md` | 上下文投影、history 替代前置、大输出引用化 |
| `05-map-self-management.md` | retention / compaction / salience / projection / GC |
| `06-routing-thin-and-verification-first.md` | thin path、task-shape routing、verification-first |
| `07-decision-adoption-and-result-lifecycle.md` | result adoption 收敛机制 |
| `08-observability-and-budget-metrics.md` | token/time/request 观测与 2x 验收指标 |
| `09-e3-validation-plan.md` | 历史 v0.0.5 E3 验证矩阵与发布门槛，当前执行前必须对照 `18` |
| `10-implementation-plan.md` | 历史 corrected implementation plan，当前 Phase 6 已被 `18` supersede |
| `11-issue-backlog.md` | 可拆 issue backlog |
| `12-risks-and-open-questions.md` | 风险、取舍和后续讨论项 |
| `13-design-corrections-and-engineering-contract.md` | 设计修正合同，与更早概念文档冲突时优先 |
| `14-implementation-gap-audit.md` | 静态实现缺口审查，已被 2026-06-19 诊断结果 supersede |
| `15-closeout-summary.md` | 历史阶段性收口总结，不再作为当前关闭依据 |
| `16-terminal-bench_E3-P0_3_2-variant-run.md` | 低成本诊断变体运行记录 |
| `17-unfinished-work-inventory.md` | 当前权威未完成工作盘点与继续开发顺序 |
| `18-unfinished-work-engineering-design.md` | 当前 canonical execution entry：未完成项工程设计、代码落点、阶段门禁和验证方案 |
| `TaskSpace-v0.0.5-Design-All-in-One.md` | 合并版设计文档，对外分发前需要重新生成 |
| `schemas/*.json` | 核心结构示例 schema |
| `examples/*.md` | 关键流程示例 |
| `checklists/*.md` | 历史实施与验收 checklist，当前验收以 `18` 和实验制度为准 |

## 阅读建议

开始 v0.0.5 后续开发前：

```text
17-unfinished-work-inventory.md
18-unfinished-work-engineering-design.md
docs/experiments/taskspace-evidence-levels-and-samples.md
```

拆分工程实现时：

```text
18-unfinished-work-engineering-design.md
03-protocol-compaction.md
04-context-projection-and-replay-control.md
05-map-self-management.md
11-issue-backlog.md
```

运行任何 E3 或 E3 变体前：

```text
18-unfinished-work-engineering-design.md
docs/experiments/taskspace-evidence-levels-and-samples.md
08-observability-and-budget-metrics.md
09-e3-validation-plan.md
checklists/acceptance-checklist.md
```

如果这些文件之间出现冲突，先按 `18-unfinished-work-engineering-design.md` 和 `docs/experiments/taskspace-evidence-levels-and-samples.md` 执行，并把冲突记录为文档修复项。
