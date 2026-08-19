# Base 3.0.6 跨样本 repeat=3 结果

- Date: 2026-08-20
- Issue: R8-I03；旁路观察 I04、I07
- Ledger: `WAR-20260820-023538-R8-I03-CROSS-SAMPLE-R3`
- Scenario: `release-dispatch-repair × map-request × repeat=3`
- Model: `deepseek-v4-flash`
- Budget: CNY 0.20；实际估算 CNY 0.08457704

## 结果

| Run | 业务/公开/Oracle/Map | 请求 | input | cached | output | 顶层 client 逃逸 | Exec 拒绝 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | 全部通过 | 10 | 174,104 | 151,936 | 5,469 | 0 | 2 |
| 2 | 全部通过 | 8 | 135,701 | 120,192 | 4,882 | 0 | 0 |
| 3 | 全部通过 | 8 | 133,221 | 122,624 | 3,853 | 0 | 0 |
| 总计 | 3/3 | 26 | 443,026 | 394,752 | 14,204 | **0** | 2 |

三轮 Provider wire 均为 TaskSpace Base `3.0.6`，hash 为 `8ce811...a449d`，且
`matches_current_contract=true`。23 次顶层 Function Call 全部为 `taskspace_exec`，没有 `exec_command` 或其他 client Tool
逃逸。Request 2+ 加权缓存命中率为 89.57%。

## I03 判断

Base `3.0.6` 在第二个自然复杂样本上获得 3/3 正向证据；连同 `subscription-billing-repair` 的 5/5，当前累计两个
复杂样本 8/8、顶层 client Tool 逃逸 0。可以收敛“Base 显式内外层作用域能抑制顶层提升”这一子问题，不再增加同义
提示文字，也不改写原生 Tool identity。

I03 整体仍保持 `verifying`。Run 1 先后出现一次缺少顶层 `type` 和一次 JSON syntax 错误，Runtime 均零副作用拒绝，Agent
随后纠正并完成任务。它们不是顶层逃逸，但仍属于 Exec envelope 生成稳定性问题。

后续提交 `3eeaeac3c` 已离线修复第二个错误：仅当缺失一个 action 闭合符、freeform `apply_patch` 被唯一误包为
`input.cmd`、Patch 标记完整且归一化结果通过当前 Catalog 完整解码时，才在历史落账前机械替换。非 Patch、歧义候选和
缺失 `type` 不自愈。该分支已有确定性回归但尚未自然在线命中，因此不改变 I03 的 `verifying` 状态。

## I04 与 I07

三张 Map 都是 5 节点、4 边的线性链。没有自然形成 fork/join，也没有在同一批次完成刚解锁的父子 Work 节点，因此 I04
仍缺目标行为证据。

三次 Agent、公开验证、隐藏 Oracle、Map store 和 Provider usage 原始证据均完整，但 benchmark 后处理依次暴露了旧
`results/agentThreads/events`、单值 `.Count`、可选 timing 字段和单臂 oracle probe 假设。相关工程缺口已修复，专项 graph、
metrics、cost、Exec observer 和 performance 回归通过；由于没有在修复后重新消耗 API 运行完整 finalization，I07 重新标为
`verifying`，等待后续自然运行顺带闭环。完整历史 `test-harness.ps1` 还会在后段
`audit-report.ps1:209` 的单值 `.Count` fixture 处失败；该缺口不影响本轮原始运行事实，但仍归 I07 工程收尾。

完整机器证据：
[`../../../../benchmarks/taskspace/r8/evidence/WAR-20260820-023538-R8-I03-CROSS-SAMPLE-R3.json`](../../../../benchmarks/taskspace/r8/evidence/WAR-20260820-023538-R8-I03-CROSS-SAMPLE-R3.json)。
