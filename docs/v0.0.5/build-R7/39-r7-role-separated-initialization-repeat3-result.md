# R7 角色分区初始化 repeat-3 结果

> 状态：R-20 已关闭；R-10 重新打开；R-19 不晋升
>
> 候选：`3e827065c912d9c332a9e52a796ccd7c2207106c`
>
> 基线：`b6bf532bf8b6d92d076b30d842e54c4f565fcfee`

## 1. 结论

第一版 `nodes + role ids` 紧凑 wire 的根因已修复。新 wire 重新区分 `root`、`initial_work`、
`additional_work` 和 `finish_id`，18 个 TaskSpace run 共 19 次初始化尝试、18 次提交、1 次普通图
端点拼写失败，未再出现 Finish/Work 角色重叠。固定 Tool section 从 `55,578` 降到 `46,926`
bytes/request，下降 `15.57%`。

候选不能整体晋升。`map-append` 有 1/6 run 的业务验证通过，但 Agent 未闭合 Map，CLI
`interrupted`。该 run 先产生 3 次单独 `complete_then_continue`，又选择未 ready 的后继节点，最后
`finish_map` 被硬状态机正确拒绝。

## 2. 四臂结果

| Arm | 成功 | Requests 总计 | Input 总计 | Req2+ cache | Wall 总计 |
|---|---:|---:|---:|---:|---:|
| Standard | 6/6 | 59 | 906,157 | 96.81% | 184.13s |
| map-always | 6/6 | 67 | 1,641,182 | 44.05% | 265.63s |
| map-append | 5/6 | 77 | 2,355,341* | 95.78%* | 295.63s |
| map-request | 6/6 | 87 | 2,049,363 | 94.19% | 293.96s |

\* interrupted run 的正式 observer 保持 token unavailable；表中数值由 20 个
`provider_request_budget status=response_completed` 事件重建，仅用于诊断，不改变其 incomplete 状态。

## 3. 与旧基线比较

| TaskSpace 汇总 | 旧基线 | 角色分区候选 | 变化 |
|---|---:|---:|---:|
| 业务/协议闭合 | 18/18 | 17/18 | -1 |
| Requests | 260 | 231 | -11.15% |
| Input tokens | 7,402,939 | 6,045,886* | -18.33% |
| 初始化 attempts / failures | 20 / 2 | 19 / 1 | 改善 |
| Protocol failures | 25 | 28 | 未解决 |
| State failures | 16 | 19 | 未解决 |

成本收益成立，但不能抵消闭合回归。

## 4. R-10 重新打开

旧基线本身已有 25 次 protocol failure，说明 R-10 过去只是“Runtime 能拒绝单独 boundary”，并没有实现
“Agent 无法生成单独 boundary”。当前 `taskspace_control` schema 只约束一个 control 调用的参数，
无法要求同一 response 还存在普通 Tool sibling；`after_boundary` 又位于另一份普通 Tool schema 中。
因此当前方案本质上仍是后置惩罚。

下一候选必须在 L4 结构上让非终态 boundary 与真实后继动作不可分离，同时保持：

- 初始化角色分区和固定 capability epoch；
- 普通 Tool 顶层业务参数及 Patch 原文；
- Agent 选择 revision、当前节点、后继节点和真实动作；
- Runtime 只做机械事务、顺序和硬状态校验；
- Standard 隔离与三种 projection 共用基建；
- 不恢复完整 lifecycle 联合、通用 nested dispatcher 或 Runtime 自动补动作。

该变更会调整 boundary action 的唯一承载位置，属于重大 Tool 合同决策，未在本轮直接实施。
