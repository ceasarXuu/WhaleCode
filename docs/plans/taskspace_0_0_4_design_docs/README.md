# TaskSpace 0.0.4 设计文档包

日期：2026-06-11  
状态：v0.1 design draft  
输入材料：TaskSpace 0.0.3 架构总结、外部审查材料包、E3 pair evidence pack、runtime/schema/prompt 摘录、graph dump、aggregate report。

## 版本命题

TaskSpace 0.0.4 不定义为“更强 planner”，而定义为：

```text
Problem-State Ledger + Result Adoption + Graph Health + Clean E3 Audit
```

0.0.3 已经证明 TaskSpace runtime 能真实接入 Whale 执行路径；0.0.4 的目标是证明 TaskSpace 不只是能记录结构化活动，而是能把结构化活动转化为可审计的问题状态、决策依据和评估证据。

## 文档目录

| 文档 | 用途 |
|---|---|
| `00-design-overview.md` | 总体设计结论、原则、范围、P0/P1/P2 优先级 |
| `01-evidence-and-problem-statement.md` | 0.0.3 证据、问题归因、0.0.4 设计输入 |
| `02-prd.md` | 产品需求、用户/角色、功能需求、验收标准 |
| `03-system-architecture.md` | 0.0.4 模块架构与职责边界 |
| `04-problem-state-ledger.md` | ProblemStateLedger 数据模型、生命周期、更新协议 |
| `05-taskspace-control-schema-v2.md` | `taskspace_control` v2 action 设计 |
| `06-runtime-gates-and-state-machine.md` | runtime gate、状态机、硬阻断与软告警 |
| `07-result-adoption-and-dependency.md` | result validity、adoption、dependency graph、taint 传播 |
| `08-typed-nodes-and-graph-convergence.md` | typed node contract、node 粒度、图收敛策略 |
| `09-subagent-contract-and-roi.md` | subagent spawn contract、result contract、ROI 指标 |
| `10-graph-health-and-viewer.md` | graph health 指标与 viewer v2 设计 |
| `11-clean-e3-audit-and-failure-taxonomy.md` | audit manifest、failure taxonomy、aggregate 规则 |
| `12-benchmark-and-release-plan.md` | benchmark 分层、0.0.4 复跑矩阵、release gate |
| `13-migration-and-implementation-plan.md` | 兼容、迁移、实施顺序、回滚策略 |
| `14-issue-backlog.md` | 可直接转 issue 的 backlog |
| `15-acceptance-checklist.md` | 合入、E3、发布前 checklist |
| `TaskSpace-0.0.4-Design-Spec-All-in-One.md` | 上述核心文档合并版 |
| `schemas/` | 设计用 schema 草案 |
| `examples/` | 典型样本下的运行方式示例 |

## 建议阅读顺序

1. 先读 `00-design-overview.md` 和 `01-evidence-and-problem-statement.md`。
2. 需要做产品范围决策时读 `02-prd.md`。
3. 需要拆工程任务时读 `03` 到 `11`。
4. 进入开发排期时读 `13-migration-and-implementation-plan.md` 和 `14-issue-backlog.md`。

## 最高优先级结论

```text
P0:
  CleanE3AuditManifest
  FailureTaxonomyV1
  GraphHealthReportOnly
  ProblemStateLedgerV1
  ResultAdoptionV1
  TypedNodeKindContractV1

P1:
  SubagentContractV1
  ThinModeClassifierReportOnly
  ViewerV2

P2:
  Graph prune / merge hard actions
  automatic mode switching
  benchmark expansion
```
