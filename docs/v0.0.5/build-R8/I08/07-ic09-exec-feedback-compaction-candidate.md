# IC-09 Exec 反馈机械收敛候选

- Status: static-complete / live-validation-pending
- Date: 2026-08-17
- Issue: R8-I08
- Variable: 仅收敛 Agent-visible `taskspace_exec` 成功反馈
- Cache candidate fingerprint: `cf157ba701ea8e88c80eeb8396f7b1cc8d9062b9c565e86b7add3595b89cb41b`

## 1. 变更边界

保留原生 Tool 结果、错误、节点归属、当前节点状态、实际状态变化和 Waiting 依赖说明。只删除没有 Agent 操作消费者的机械字段：

- 固定 `kind`、固定 `status`；
- 已由原生 Function output `call_id` 表达的 `outer_call_id`；
- 仅供 Runtime 持久化和日志使用的 `map_id`、`map_revision_at_dispatch`；
- 可由 outer call 与数组顺序派生的 `call_index`、`action_id`；
- 非 read 序列的空 `reads`、纯 Map 序列的空 `client_results` 和其他空反馈数组；
- 未变化时重复的 before/after 状态。当前状态始终保留，只有实际变化时才返回 `previous_state`。

Runtime 内部 identity、revision、action settlement 和日志字段不变。Deferred Tool 恢复继续依赖原生 `call_id` 与
`taskspace_exec` call/output 配对，不再依赖反馈中重复的固定 `kind`。

## 2. 免费验证

| 验证 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo test -p codex-core taskspace_exec --locked` | 72 passed |
| 反馈 schema 与序列化一致性 | passed；handler 测试直接验证被删除字段不再出现 |
| Deferred Tool 恢复 | passed |
| 缓存敏感面门禁 | 免费 final-wire passed；候选变化可比较，发布基线继续阻断 |

## 3. 静态收益

对 IC-06 真实 trace 的 6 个成功 Exec output 只执行上述机械字段变换：

| 指标 | 变更前 | 候选 | 差值 |
|---|---:|---:|---:|
| 成功 Exec output JSON | 7,957 B | 5,550 B | -2,407 B / -30.25% |
| 内部原生 Tool output | 3,613 B | 3,613 B | 0 |

这是历史 trace 的确定性结构反算，不是 Provider token 或行为收益。由于早期 output 会进入多个后续请求，真实 input 收益必须由
同 sample、同 commit 的 Standard/map-request 双臂确认；不得用 `bytes/4` 代替。

## 4. 真实验收

使用计划中已批准的额外修复复验额度：`single-file-fast-fix`，Standard 1 + map-request 1，零自动重试。两臂必须业务和隐藏
oracle 通过；TaskSpace 必须无反馈解析、Deferred Tool、状态理解或 Waiting 回归。报告 request、input、cache、output、time、cost
和逐请求 Exec output 面积。任何异常立即停止，不自动补跑。
