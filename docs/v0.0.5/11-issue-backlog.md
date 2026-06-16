# 11. v0.0.5 Issue Backlog

## EPIC A：Cost Instrumentation

### TS-005-A01：Token Summary Artifact

目标：pair/sample/suite 输出一等公民 token summary。

交付：

```text
token-summary.json
model_request_count
avg_input_per_request
max_input_per_request
taskspace_control_count
largest_tool_output_bytes
```

验收：所有 E3 side 都生成 token summary。

### TS-005-A02：Request Proxy Reconciliation

目标：自动复现 v0.0.4 的 request-count × input/request 分解。

验收：报告能输出 request_count_ratio 与 avg_input_per_request_ratio。

## EPIC B：Protocol Compaction

### TS-005-B01：StateCommitV1 Schema

目标：新增批量状态提交 schema。

验收：schema 支持 node/result/fact/decision/criteria/next_action 批量更新。

### TS-005-B02：StateCommit Handler

目标：runtime 支持局部接受/拒绝 state_commit。

验收：无效 refs 不污染状态，合法部分可提交。

### TS-005-B03：Legacy Action Soft Deprecation

目标：保留旧 action，但报告 legacy usage。

验收：E3 报告显示 legacy action count 和 state_commit adoption rate。

### TS-005-B04：Next Valid Action Gate

目标：gate 拒绝时返回合法下一步模板。

验收：gate retry count 下降。

## EPIC C：Context Projection

### TS-005-C01：ContextProjectionV1

目标：从完整 map 生成 active working set。

验收：每轮有 projection event，projection size 可测。

### TS-005-C02：Static/Dynamic Context Split

目标：TaskSpace protocol 不每轮完整重述。

验收：dynamic projection 可独立计量。

### TS-005-C03：Prompt Projection Budget

目标：thin/default/deep projection size 有预算。

验收：超预算触发 compaction，不直接注入全文。

## EPIC D：Output Referenceization

### TS-005-D01：Large Output Ref Policy

目标：大工具输出引用化。

验收：>50KB 输出不直接进入后续 prompt。

### TS-005-D02：Slice-on-demand Tooling

目标：模型可按行/模式/摘要请求 artifact slice。

验收：日志类任务仍能获取必要信息。

## EPIC E：Map Self-Management

### TS-005-E01：Retention Class

目标：map item 有 active/retained/archived/audit-only/discarded。

验收：100% map items 有 retention class。

### TS-005-E02：Compaction Operators

目标：实现 result/node/failure/validation/subagent collapse。

验收：compaction-events.jsonl 生成。

### TS-005-E03：Salience Scoring

目标：按当前决策重要性排序 map items。

验收：projection 使用 salience 选择 items。

### TS-005-E04：Map GC

目标：stale/unreviewed/blocked/no-yield 出 active context。

验收：final projection 不含 stale blocked nodes。

### TS-005-E05：Semantic Replacement Metrics

目标：测 map 替代标准 history 的潜力。

验收：semantic_replacement_rate 和 history_shadow_elidable_tokens 可计算。

## EPIC F：Routing

### TS-005-F01：TaskShapeRouterV1

目标：输出 thin/default/verification/subagent/deep mode。

验收：100% TaskSpace runs 有 routing-decision.json。

### TS-005-F02：Thin Path

目标：小任务低摩擦路径。

验收：thin task 默认不 spawn，state_commit_count <= 4 before first validation。

### TS-005-F03：Verification-first Path

目标：格式敏感任务先读 validator/expected format。

验收：count-call-stack 有 expected-format decision 和 local checker evidence。

## EPIC G：E3 Validation

### TS-005-G01：Cost Gate Report

目标：suite-cost-gate.json。

验收：2x cost pass/partial/fail 自动输出。

### TS-005-G02：Compact Profile E3

目标：跑 v005-compact profile。

验收：输出 sample/pair/suite 对照报告。
