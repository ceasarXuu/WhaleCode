# client work 恢复复验独立异常登记

- Date: 2026-08-16
- Source: [`66-client-work-restoration-repeat3-result.md`](66-client-work-restoration-repeat3-result.md)、
  [`68-client-work-restoration-repeat10-result.md`](68-client-work-restoration-repeat10-result.md)、
  [`69-owner-state-feedback-repeat5-result.md`](69-owner-state-feedback-repeat5-result.md)、
  [`70-affected-node-state-feedback-repeat5-result.md`](70-affected-node-state-feedback-repeat5-result.md)
- Scope: 与“首次工作型序列缺少 client work”修复无因果关系的异常

## 1. 登记规则

这些表现都发生在已成功初始化 Map 并执行首个 client work 之后。它们不能计为 client work 结构前置条件失败，也不因为共享
同一次运行而合并成一个根因。已有全局问题能够容纳时不新增顶层 I 编号，只增加稳定观测标识。

## 2. 当前清单

| Stable ID | 归属 | 表现 | repeat=3 | repeat=10 | owner-state R5 | affected-state R5 | 累计 | Runtime 行为 |
|---|---|---|---:|---:|---:|---:|---:|---|
| I03-ARG-SYNTAX | I03 | `taskspace_exec.arguments` 缺少合法 JSON 分隔 | 1 event / 1 run | 1 event / 1 run | 0 | 4 events / 1 run | 6 events / 3 runs / 23 | 准确返回 syntax；零副作用；最终修正 |
| I03-ARG-WRAPPER | I03 | 把完整 sequence 再包进顶层 `arguments` | 0 | 0 | 0 | 1 event / 1 run | 1 event / 1 run / 23 | 准确返回 top-level contract；零副作用；最终修正 |
| I03-TOP-LEVEL-CLIENT | I03 | 生成未声明顶层 client Tool，绕过 `taskspace_exec` | 0 | 0 | 1 event / 1 run | 0 | 1 event / 1 run / 23 | response contract 在执行前终止；无 client Tool 副作用 |
| I04-FRONTIER-EARLY | I04 | 父节点未完成时直接在 Waiting 子节点执行 Tool | 1 event / 1 run | 4 events / 4 runs | 2 events / 2 runs | 2 events / 2 runs | 9 events / 9 runs / 23 | 列出未完成直接父节点；零副作用；下一请求先完成父节点 |
| I04-REDUNDANT-INFLIGHT | I04 | 同批提交 Tool 时又显式把该 Ready 节点改为 `in_flight` | 2 events / 1 run | 2 events / 2 runs | 0 | 3 events / 3 runs | 7 events / 6 runs / 23 | `TransitionInvalid`；零副作用；移除冗余转换后继续 |
| I04-WAITING-COMPLETE | I04 | 同批完成父节点并把此前 Waiting 子节点直接标记 completed | 0 | 0 | 0 | 1 event / 1 run | 1 event / 1 run / 23 | `TransitionInvalid`；零副作用；先完成父节点再更新子节点 |

## 3. 证据身份

| Stable ID | Run | Outer call |
|---|---:|---|
| I03-ARG-SYNTAX | 1 | `call_00_Wubd3WWV0gTdlK1zSjQY2320` |
| I04-FRONTIER-EARLY | 1 | `call_00_viJ0tShXv8cb47Th5SnD1819` |
| I04-REDUNDANT-INFLIGHT | 3 | `call_00_yk1u7DEpkZ7ZZjufDMZb4196` |
| I04-REDUNDANT-INFLIGHT | 3 | `call_00_Jwkt9ZBOzoBfB5AXxQiP4801` |
| I04-REDUNDANT-INFLIGHT | repeat10 Run 3 | `call_00_CRYDgGzqGekYC6DDdKOE8741` |
| I04-REDUNDANT-INFLIGHT | repeat10 Run 4 | `call_00_MnN2p8w0hzOTfh1OPdzD1331` |
| I04-FRONTIER-EARLY | repeat10 Run 6 | `call_00_vh5aqLRmN3yFn3GuU28P6522` |
| I04-FRONTIER-EARLY | repeat10 Run 8 | `call_00_nNJ3JmxAOlyW0EDHRKYG6965` |
| I03-ARG-SYNTAX | repeat10 Run 9 | `call_00_tDnRCzXWb5HI59Rtvmrb3163` |
| I04-FRONTIER-EARLY | repeat10 Run 9 | `call_00_OXHGWKPQDEyWsI4dZ7T10833` |
| I04-FRONTIER-EARLY | repeat10 Run 10 | `call_00_WV4ZZJUO99L2MuKIZuOr2366` |
| I04-FRONTIER-EARLY | owner-state Run 1 | `call_00_poaU1mHXDwweh9urdfwX8395` |
| I04-FRONTIER-EARLY | owner-state Run 3 | `call_00_ARW8Ox8ImjsfftndjA9k9977` |
| I03-TOP-LEVEL-CLIENT | owner-state Run 5 | `call_00_cfrPxd7dktaa0GNpYQzu9616` |
| I04-REDUNDANT-INFLIGHT | affected-state Run 1 | `call_00_TMY5CzfmSF4f6szLTkM09970` |
| I04-WAITING-COMPLETE | affected-state Run 1 | `call_00_AoFz5m2llJXo6gHrgxLQ7800` |
| I03-ARG-SYNTAX | affected-state Run 3 | `call_00_OArvF7XeiE1V0wxGXhZJ3786`、`call_00_Fh15JlH8uayaP7K8JNZt2525`、`call_00_R81G9pGw2sSM4dh1CpME2014`、`call_00_ba487vAJjBkulsQVTwa31668` |
| I03-ARG-WRAPPER | affected-state Run 3 | `call_00_4TnZ18ihkICwwjtkc1Ma9277` |
| I04-FRONTIER-EARLY | affected-state Run 3 | `call_00_wXhQtX3eITvLAvmGwxec9963` |
| I04-REDUNDANT-INFLIGHT | affected-state Run 4 | `call_00_BzIZ5zCFv6mrzPeM9ck85354` |
| I04-FRONTIER-EARLY | affected-state Run 4 | `call_00_18fwKFHLnRohacwwBTHV4223` |
| I04-REDUNDANT-INFLIGHT | affected-state Run 5 | `call_00_ccDpnz3ZKbrxpBWl5pZB3435` |

原始 rollout：

- `target/r8-client-work-restoration/repeat3-1/single-file-fast-fix/20260816-210852-165/pair-001/right/artifacts/rollout.jsonl`
- `target/r8-client-work-restoration/repeat3-3/single-file-fast-fix/20260816-210852-146/pair-001/right/artifacts/rollout.jsonl`
- `target/r8-client-work-restoration/repeat10-{1..10}/single-file-fast-fix/*/pair-001/right/artifacts/rollout.jsonl`
- `target/r8-owner-state-feedback/repeat5-{1..5}/single-file-fast-fix/*/pair-001/right/artifacts/rollout.jsonl`
- `target/r8-feedback-candidate/repeat5-{1..5}/single-file-fast-fix/*/pair-001/right/artifacts/rollout.jsonl`

## 4. 边界判断

1. I03-ARG-SYNTAX 是 Agent 参数生成稳定性问题。当前反馈准确区分 syntax，没有 wrapper 注入或错误层级混淆，因此不是 I05
   反馈分类缺陷复发。
2. I04-FRONTIER-EARLY 与 I04-REDUNDANT-INFLIGHT 都涉及节点状态，但前者是选错可执行 frontier，后者是重复声明 Runtime 已按
   Tool 归属机械完成的启动转换；统计和后续根因分析不得合并。
3. 三类异常均被硬规则零副作用拦截，说明 Runtime 底线正确；这不等于 Agent 行为成本可接受。
4. 后续 repeat 扩大只统计复发率和上下文诱因，不因单次复发立即增加 Runtime 语义干预。
5. repeat=10 中首次合法初始化并执行 client work 为 10/10，顶层 client Tool 逃逸为 0/10；三类异常均发生在首个 work 之后，
   继续与本次结构恢复分开评价。
6. owner-state repeat=5 中只有 4 轮到达 patch-to-verify 边界，frontier 误选为 2/4；Run 5 的顶层 client Tool 逃逸提前终止，
   不进入 frontier 分母。累计表中的 `/18` 只表示总运行数，不能替代按实际到达边界计算的条件频率。
7. affected-state repeat=5 五轮均到达目标边界且逐字获得 canonical 状态和未完成父节点，frontier 误选仍为 2/5；它证明反馈
   缺口已修，但没有证明 Agent 的合法序列选择已收敛。
