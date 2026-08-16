# IC-09 Exec 反馈机械收敛候选

- Status: complete / live-validated
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

使用已批准的额外修复复验额度运行 `single-file-fast-fix`，Standard 1 + map-request 1，零自动重试。证据：
`target/r8-i08/ic09-feedback-compaction/single-file-fast-fix/20260817-061016-681`。

| 指标 | Standard | map-request | TaskSpace / Standard |
|---|---:|---:|---:|
| 业务、公开验证、隐藏 oracle | passed | passed | 等价 |
| Provider requests | 5 | 7 | 1.40x |
| Input tokens | 61,368 | 106,500 | 1.74x |
| Cached input | 59,136 | 99,968 | 1.69x |
| Uncached input | 2,232 | 6,532 | 2.93x |
| Output tokens | 1,224 | 2,288 | 1.87x |
| Request 2+ cache hit | 95.63% | 93.13% | -2.50pp |
| Agent wall time | 13.38s | 20.12s | 1.50x |

TaskSpace 共 6 次 Exec、5 个 client actions，Map 为 5 nodes / 4 edges，全部闭合。没有 Waiting、JSON、协议拒绝、Tool 失败、
显式 `read_map` 或状态理解异常。6 个成功 Exec output 合计 6,537 B；旧 trace 排除一次 Waiting reject 后的 6 个对应成功 output
合计 7,957 B，减少 1,420 B（17.85%）。该在线值低于静态 30.25%，主要因为本轮 Map 多一个节点且具体 Tool 输出不同；它证明反馈
载体确实缩小，不把随机动作路径差异归因给字段删除。

两臂合计 12 requests、167,868 input、3,512 output，按冻结价格估算 CNY 0.01897008。当前结论只关闭“成功反馈中的机械冗余”；
TaskSpace 固定 Tool 合同仍为 26,688 B/request，额外请求和更大的历史仍使总 input 明显高于 Standard，R8-I08 保持 open。

五对 repeat=5 稳定性结果见 [`08-ic09-feedback-compaction-repeat5-result.md`](08-ic09-feedback-compaction-repeat5-result.md)：10/10
业务与 oracle 通过，成功 Exec output 平均体量较变更前五轮下降 27.65%；两次派生状态误写和一次 JSON syntax reject 独立归入
I04/I03，不构成反馈字段删除回归。
