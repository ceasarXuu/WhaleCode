# TaskSpace Base 显式 client Tool 作用域 repeat=5 结果

- Status: completed-candidate-retained
- Date: 2026-08-19
- Ledger: `WAR-20260819-223533-R8-BASE-CLIENT-SCOPE-R5`
- Subject commit: `5d2516eb1`
- Budget: 用户批准不超过 CNY 1.00；实际估算 CNY 0.20924088

## 唯一变量

TaskSpace Base `3.0.5` 原本已有宏观规则：TaskSpace execution capability 是 Map 与 client Tool 的唯一顶层入口，
不得单独顶层调用 client Tool。历史 B0 和 H-009 都携带该规则却仍然逃逸，因此本轮没有重复添加同义句，而是将它替换为
显式层级合同：点名顶层 `taskspace_exec`，并规定包括 `exec_command` 在内的 client Tool 只能位于其 `tools` 数组。

TaskSpace Base 升级为 `3.0.6`，Standard、Tool schema、TaskSpace Exec description、Runtime、Map/DAG、反馈和拒绝均不变。
五轮 Provider trace 均报告版本 `3.0.6`、hash `8ce811...449d`、`matches_current_contract=true`，排除了候选未加载。

## 结果

| 指标 | 历史 B0 | 内层 description H-009 | Base H-010 |
|---|---:|---:|---:|
| 顶层逃逸 runs | 3/5 | 4/5 | **0/5** |
| 顶层逃逸 calls | 6 | 6 | **0** |
| 所有非法动作 | 14 | 8 | **4** |
| 业务 / 公开 / hidden oracle | 5/5 | 5/5 | 5/5 |
| Map 完整闭合 | 5/5 | 5/5 | 5/5 |
| Provider requests | 56 | 50 | 48 |
| Input / cached / uncached | 1,026,629 / 942,336 / 84,293 | 870,780 / 759,808 / 110,972 | 883,878 / 751,744 / 132,134 |
| Output | 33,797 | 28,113 | 31,036 |
| Request 2+ cache hit | 未固化 | 90.85% | 90.35% |
| Agent wall time | 267.908s | 230.645s | 243.439s |
| 估算费用 | CNY 0.17073372 | CNY 0.18239416 | CNY 0.20924088 |

本轮剩余四次拒绝为两次未转义换行 JSON syntax 和两次对 Waiting `verify` 的提前执行；均由 Runtime 零副作用拒绝并在
下一请求恢复。它们不是顶层逃逸，也没有抵消总体非法动作下降，但仍属于 I03/I04 的独立遗留，不能因本轮主指标通过而忽略。

## 结论

H-010 在当前复杂样本 repeat=5 上获得支持：显式 Base 层级合同比内层 Tool description 更有效，Base `3.0.6` 保留。
该结果证明的是一个样本上的方向性和候选可晋升性，不代表所有 client Tool、样本或 projection 已稳定，因此 I03 继续
`verifying`。后续优先复用自然复杂验收观察跨样本复发，不立即继续增加提示文字。

完整证据：`benchmarks/taskspace/r8/evidence/WAR-20260819-223533-R8-BASE-CLIENT-SCOPE-R5.json`。
