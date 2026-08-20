# exec_command 内层作用域描述 repeat=5 结果

- Status: completed-candidate-rejected
- Date: 2026-08-19
- Ledger: `WAR-20260819-220821-R8-EXEC-SCOPE-DESCRIPTION-R5`
- Subject commit: `bf0b7cc8b`
- Budget: 用户批准不超过 CNY 1.00；实际估算 CNY 0.18239416

## 唯一变量

仅在 TaskSpace Exec 内部 catalog 的 `exec_command` description 中追加：该 action 只能位于
`taskspace_exec.tools`，不得作为顶层 Function Tool call。Standard、其他内部 Tool、Base instructions、Map/DAG、
Runtime 拒绝和反馈均保持不变。

Provider wire 证明候选实际生效：五轮的 client catalog 均为 16,164 bytes、同一 hash；历史 B0 为 16,024 bytes，
正好增加 140 bytes。当前二进制也包含目标合同文本，因此本轮不是“改动没有进入请求”。

## 结果

| 指标 | 历史 B0 | 本轮 H-009 | 判断 |
|---|---:|---:|---|
| 逃逸 runs | 3/5 | 4/5 | 未改善 |
| 逃逸 calls | 6 | 6 | 未改善 |
| 每轮逃逸 | `[1,0,2,0,3]` | `[3,0,1,1,1]` | 分布波动，不构成收益 |
| 业务 / 公开 / hidden oracle | 5/5 | 5/5 | 无正确性回归 |
| Map 完整闭合 | 5/5 | 5/5 | 无 Map 回归 |
| Provider requests | 56 | 50 | 仅观测，不归因于候选 |
| Input / cached / uncached | 1,026,629 / 942,336 / 84,293 | 870,780 / 759,808 / 110,972 | 仅观测，不归因于候选 |
| Output | 33,797 | 28,113 | 仅观测，不归因于候选 |
| Request 2+ cache hit | 未在旧证据固化 | 90.85% | 无 shape transition |
| Agent wall time | 267.908s | 230.645s | 并行批次中的单轮总和 |
| 估算费用 | CNY 0.17073372 | CNY 0.18239416 | 低于授权上限 |

六次逃逸全部与首个 `initialize_and_work` 位于同一响应，仍早于任何 Tool feedback。Run 2 另有一次错误
`finish_map` 类型造成 contract reject，Run 5 另有一次空 Map 更新造成 `NoEffectMapUpdate`；两者均零副作用恢复，
且没有证据表明由 140 bytes 描述导致，不作为候选因果结论。

## 结论

H-009 被证伪：局部 description 的约束强度不足以改变首响应顶层 Function 选择。候选不晋升并回退，I03 继续
`verifying`。后续不得在该候选上继续堆叠文字；应转向能够结构性限制 Provider 首响应 Function 名的机制，或重新评估
当前 outer/inner Function 表达模型。

完整证据：`benchmarks/taskspace/r8/evidence/WAR-20260819-220821-R8-EXEC-SCOPE-DESCRIPTION-R5.json`。
