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

Agent 先完成 `fix` 并执行全量测试，随后在一个 `update_and_finish` 中尝试同时把 `fix` 与仍处于 `waiting` 的
`verify` 标为 completed。Runtime 以 `TransitionInvalid` 原子拒绝，未执行任何 Map 或 Tool 动作。Agent 下一请求先完成
`fix`，使 `verify` 变为可执行，再完成 `verify` 与 Finish。

这说明硬边界正确且 Agent 可恢复，但也留下两项证据：

1. I03 仍存在一次可恢复的动作组织错误，继续保持 `verifying`；
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
