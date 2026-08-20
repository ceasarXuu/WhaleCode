# R8 I04 自然 Fork/Join 生产验收结果

- Date: 2026-08-18
- Subject commit: `413f940d7e846ffd029619fe38ba014fcf38736b`
- Run ledger: `WAR-20260818-223200-R8-I04-DAG-R1`
- Sample: `release-dispatch-repair`
- Status: business passed / fork-join not observed / I04 remains verifying

## 1. 正确性与成本

| 模式 | 业务/公开测试/隐藏 Oracle | 请求 | Input | Cached | Uncached | Output | Agent wall | Tool calls |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | PASS | 9 | 127,531 | 123,776 | 3,755 | 3,375 | 27.934s | 8 |
| map-request | PASS | 11 | 206,060 | 184,960 | 21,100 | 6,492 | 50.371s | 10 |
| **合计** | **2/2** | **20** | **333,591** | **308,736** | **24,855** | **9,867** | **78.305s** | **18** |

两臂都只修改 `inventory.py` 与 `shipping.py`，公开测试与隐藏 Oracle 全部通过。按冻结价格估算总费用为
`CNY 0.05076372`。

## 2. 实际 Map

TaskSpace 最终形成 `root -> explore -> fix -> verify -> finish` 的五节点线性链：4 条边、最大深度 4、最大入度和
出度均为 1。Root/Finish 唯一，所有节点均位于 Root 到 Finish 的路径上，最终没有开放叶节点。

样本客观提供 inventory 与 shipping 两个独立修复域，但 Agent 将两个修改合并在同一个 `fix` 节点和一次 patch 中，
没有建立两个分支及共同 join。业务完成不算失败，但该结果不能关闭 I04 的 fork/join 验收。

## 3. 协议行为

Agent 先完成 `fix` 并执行全量测试，随后在一个 `update_and_finish` 中按顺序声明 `fix -> completed`、
`verify -> completed` 与 Finish。旧 Runtime 只在整批 patch 结束后派生状态，因此校验第二项时仍把 `verify` 视为
`waiting`，以 `TransitionInvalid` 原子拒绝。Agent 下一请求拆开同一逻辑后成功。

该 trace 后续经产品复核确认：既然 `node_patches[]` 已表达顺序，Runtime 应在每个 patch 后机械派生状态，并允许后续 patch
操作刚解锁的节点；这不需要 Runtime 判断验证是否充分。当前修复将该拒绝从“Agent 状态错误”重分类为批内状态事务能力缺口。

这说明硬边界正确且 Agent 可恢复，但也留下两项证据：

1. I03 不再把该次拒绝计为 Agent 动作组织错误，但仍需在新实现生产 trace 中验证连续动作收益；
2. 当前 observer 报告 control/preflight/protocol/state failure 均为 0，与 rollout 中的 canonical reject 不一致，
   因此 I07 从 `closed` 恢复为 `verifying`。

首次请求在读取代码前就建立了通用线性链，可能降低后续自然拆分 Map 的倾向，但单次 trace 不足以证明这是 fork/join
缺失的根因。本轮不据此修改提示词、Runtime 或 Map 规则。

## 4. 证据

- Pair report: `target/whale-agent-runs/WAR-20260818-223200-R8-EXPERIMENT-R1B/release-dispatch-repair/r-001/pair-001/pair-report.md`
- TaskSpace graph: `target/whale-agent-runs/WAR-20260818-223200-R8-EXPERIMENT-R1B/release-dispatch-repair/r-001/pair-001/right/artifacts/graph-health.json`
- TaskSpace rollout: `target/whale-agent-runs/WAR-20260818-223200-R8-EXPERIMENT-R1B/release-dispatch-repair/r-001/pair-001/right/artifacts/rollout.jsonl`
- Global ledger: `benchmarks/whale-agent-run-ledger.json`

直接 runner 的第一次预检没有自动加载 `.env.local`，在 Provider 前以零请求终止；加载环境后按相同批准范围执行，
没有重试任何已开始的 Agent run。
