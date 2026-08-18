# R8 I04 自然 Fork/Join 生产验收

- Created: 2026-08-18
- Status: live-completed / fork-join-not-observed
- Model: `deepseek-v4-flash`
- Candidate sample: `release-dispatch-repair`
- Initial live matrix: `standard + map-request`，各 `repeat=1`

## 1. 为什么不能继续使用旧样本

`multi-file-order-pipeline` 曾被作为 branch/join 观察样本，但历史真实运行持续形成线性链。解析和定价虽然位于不同文件，
却共享同一订单规则、一次补丁和一次集成验证；Agent 把它们合并为一个 Work 节点在产品上是合理行为。继续重复该样本
不能证明复杂 frontier，也不应通过提示词要求 Agent 人为拆图。

## 2. 新样本的客观结构

`release-dispatch-repair` 包含两个可以独立理解、修改和验证的修复域：

1. `inventory`：库存行规范化、数值边界和补货阈值；
2. `shipping`：区域基础价、按整公斤计费和配送服务附加费；
3. `dispatch`：只在两项结果都正确后通过最终汇总验证。

自然的理想工作图是 Root 经过调查后分成 inventory 与 shipping 两个 Work，再汇合到 dispatch/全量验证和唯一 Finish。
这只是样本提供的工作机会，不是 Runtime 规划结果，也不写进用户提示词。

## 3. 不诱导约束

- 用户提示不出现 TaskSpace、Map、节点、状态、并行、分叉、汇合或依赖图术语；
- Standard 与 TaskSpace 使用完全相同的 prompt、fixture、公开测试和隐藏 oracle；
- Runtime 不自动创建节点、补边、拆分任务或把线性图改成 DAG；
- Agent 选择线性图不算业务失败，但不能用来关闭 I04 的复杂 frontier 验收。

## 4. 离线门禁

| 门禁 | 结果 |
|---|---|
| Scenario manifest 可读取 | PASS |
| Prompt guard 无内部概念泄露 | PASS |
| 原始 fixture | `7 failed / 2 passed` |
| 仅修复 inventory + shipping 的参考解 | `9 passed` |
| 生成式隐藏 oracle 对参考解 | PASS |
| 生产 Runtime / Agent 协议变更 | 无 |

## 5. 真实验收

第一轮只运行 Standard 与 `map-request` 各一次。不得因未形成目标图而自动增加 repeat 或修改提示词。

共同正确性门禁：

- 公开测试与隐藏 oracle 均通过；
- 工作区修改仅解决样本缺陷；
- 没有 Tool 逃逸、未配对结果、Map 硬约束绕过或观测缺口。

TaskSpace 图观察：

- Root 与 Finish 唯一，所有节点都位于 Root 到 Finish 的路径上，最终显式闭合；
- 至少出现两个可独立推进的 Work 分支；
- 后续验证节点显式依赖两个分支，形成入度至少为 2 的 join；
- Tool 的 `node_id` 归属与实际 inventory、shipping、集成验证动作一致；
- waiting/ready/in_flight/completed 转换无非法尝试。

若共同正确性失败，按实际 trace 归因并停止。若任务正确但 Agent 仍选择线性图，记录为“能力未自然采用”，I04 保持
`verifying`，不归咎 Runtime，也不通过提示词或自动补图迎合验收目标。

## 6. 预算草案

| 项目 | 上限 |
|---|---:|
| Sample runs | 2 |
| Provider requests | 30（每 run 15） |
| Input tokens | 500,000（每 run 250,000） |
| Output tokens | 20,000（每 run 10,000） |
| 估算费用 | CNY 0.54 |
| 单 run 最长时间 | 600 秒 |
| Retry | 0 |

任一运行失败、业务失败、usage 缺失或预算观察超限立即停止。真实执行前必须登记全局 Whale Agent run ledger，并取得
用户对该专项预算的明确批准。

## 7. 执行结果

Standard 与 `map-request` 均通过公开测试和隐藏 Oracle；TaskSpace 最终形成五节点线性链，未自然形成 fork/join，
I04 保持 `verifying`。完整数据与 trace 分析见
[`01-fork-join-live-validation-result.md`](01-fork-join-live-validation-result.md)。
