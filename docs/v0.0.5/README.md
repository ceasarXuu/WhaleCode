# TaskSpace v0.0.5 工程设计文档包

版本：v0.0.5 design draft  
日期：2026-06-16  
主题：Protocol Compaction & Map Self-Management

## 设计目标

v0.0.5 围绕两个大目标展开：

1. **上下文长度与轮次治理**：将 TaskSpace 模式收敛后的耗时与 token 成本控制到 Standard 模式的 **2x 以内**，作为阶段性可接受范围。
2. **TaskSpace map 自我管理能力**：让 map 从“模型可见结构化日志”进一步逼近“runtime-managed semantic working memory”。v0.0.5 不直接替换标准上下文，而是为后续 0.0.6 / 0.0.7 的上下文替代做工程前置。

## 文档目录

| 文件 | 用途 |
|---|---|
| `00-executive-summary.md` | 设计摘要、目标、非目标、版本边界 |
| `01-evidence-and-root-cause.md` | v0.0.4 根因输入与设计约束 |
| `02-system-design-overview.md` | v0.0.5 总体架构变化 |
| `03-protocol-compaction.md` | 批量状态提交、减少 taskspace_control 轮次 |
| `04-context-projection-and-replay-control.md` | 上下文投影、history 替代前置、大输出引用化 |
| `05-map-self-management.md` | retention / compaction / salience / projection / GC |
| `06-routing-thin-and-verification-first.md` | thin path、task-shape routing、verification-first |
| `07-decision-adoption-and-result-lifecycle.md` | result adoption 收敛机制 |
| `08-observability-and-budget-metrics.md` | token/time/request 观测与 2x 验收指标 |
| `09-e3-validation-plan.md` | v0.0.5 E3 验证矩阵与发布门槛 |
| `10-implementation-plan.md` | 修正后的可执行工程计划、phase gate、验证和回滚 |
| `11-issue-backlog.md` | 可直接拆 issue 的 backlog |
| `12-risks-and-open-questions.md` | 风险、取舍和后续讨论项 |
| `13-design-corrections-and-engineering-contract.md` | 设计修正合同；与早期概念文档冲突时以此为准 |
| `TaskSpace-v0.0.5-Design-All-in-One.md` | 合并版设计文档 |
| `schemas/*.json` | 核心结构示例 schema |
| `examples/*.md` | 关键流程示例 |
| `checklists/*.md` | 实施与验收 checklist |

## 修正说明

2026-06-17 后的工程执行以以下文件为准：

```text
13-design-corrections-and-engineering-contract.md
10-implementation-plan.md
checklists/acceptance-checklist.md
```

如果这些文件与早期概念文档或 `TaskSpace-v0.0.5-Design-All-in-One.md` 存在冲突，以修正合同和可执行计划为准。All-in-One 在对外分发前需要重新生成。

## 阅读建议

先读：

```text
00-executive-summary.md
01-evidence-and-root-cause.md
02-system-design-overview.md
13-design-corrections-and-engineering-contract.md
```

工程拆分时读：

```text
03-protocol-compaction.md
04-context-projection-and-replay-control.md
05-map-self-management.md
13-design-corrections-and-engineering-contract.md
10-implementation-plan.md
11-issue-backlog.md
```

跑 E3 前读：

```text
08-observability-and-budget-metrics.md
09-e3-validation-plan.md
checklists/acceptance-checklist.md
```
