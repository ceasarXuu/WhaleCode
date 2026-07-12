# R5-J6.7.6 收益门禁与收口结果

- Date: 2026-07-12
- Phase: R5-J6.7.6
- Status: Engineering evidence complete / superseded as final gate by J6.7.7
- Scope: `count-call-stack`、`subscription-billing-repair`
- Next gate: 完成 `30-r5-j6-7-phase7-context-residue-plan.md` 后执行最终授权审查

## 1. 结论

J6.7 的工程和 Docker 收益门禁已经完成，两个 Standard/R5 paired sample 均正确通过。TaskSpace
任务上下文现在只有 canonical Event Store 一份事实源，provider 按原始事件顺序机械线性化；Map、
control、result 和 projection 只保存状态或 event ref，不再复制任务正文。

已证实的收益：

1. canonical payload/call/output record 精确重复为 0，orphan call/output 为 0；
2. Map 语义 retention/salience 为 100%，protected miss、semantic replacement 和 compaction loss 为 0；
3. 两个样本的 request 2+ cache 均比同轮 Standard 高约 2.1 个百分点，没有 warm-cache 负收益；
4. active 非消息固定区较 J6.6 下降约 3.7%，较 J6.6 follow-up 再下降约 2.0%；
5. complex sample 的 request/input/wall 放大分别收敛到 1.17x/1.27x/1.12x，且 uncached input 为
   Standard 的 0.86x。

不能过度声明的部分：两组各只有 1 次，Agent 动作路径存在明显方差。focused sample 本轮 R5 为
11 requests，高于 J6.7.5 的 10 和 J6.7.3 的 9；complex sample 本轮 R5 为 14，高于 J6.7.3 的 10。
因此结构性去重和缓存收益成立，但不能由这些跨轮样本断言总请求数已经稳定下降。

## 2. 有效证据

| Sample | Evidence | Eligibility |
|---|---|---|
| focused | `target/r5-j6-7-6-live/count-call-stack/20260712-124928-300` | valid paired diagnostic, 1 repeat |
| complex | `target/r5-j6-7-6-live/subscription-billing-repair/20260712-124928-323` | valid paired diagnostic, 1 repeat |
| R4 | 当前可执行快照不可用 | 只保留历史结论，不补造 request/token/cache |

两次 `20260712-124727-*` 运行在 provider 调用前被 binary freshness preflight 拒绝，属于
`invalid_harness`，不进入收益统计。重建 locked Whale 并写入 binary attestation 后，两组正式运行通过。

## 3. 结果、动作与成本

| Sample | Mode | Result | Requests | Runtime tools | Controls | Input | Cached | Uncached | Output | Wall |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| focused | Standard | solved | 7 | 13 | 0 | 50,000 | 47,360 | 2,640 | 1,150 | 14.04s |
| focused | R5 | solved | 11 | 16 | 2 | 91,297 | 88,064 | 3,233 | 2,070 | 23.67s |
| complex | Standard | solved | 12 | 19 | 0 | 120,518 | 113,664 | 6,854 | 5,066 | 49.83s |
| complex | R5 | solved | 14 | 22 | 6 | 153,604 | 147,712 | 5,892 | 6,173 | 55.69s |

| Sample | Request ratio | Tool ratio | Input ratio | Uncached ratio | Wall ratio | Req 2+ cache delta |
|---|---:|---:|---:|---:|---:|---:|
| focused | 1.57x | 1.23x | 1.83x | 1.22x | 1.69x | +2.10pp |
| complex | 1.17x | 1.16x | 1.27x | 0.86x | 1.12x | +2.09pp |

两侧 public/hidden validator 均为 0，Agent completion 均为 complete，没有 runtime interruption 或
validator environment mismatch。

## 4. Provider 结构分账

下表从最终 Chat wire 机械分为 message bytes 与 non-message bytes。non-message 包含 tools、tool choice、
model 和其他请求字段，不对其中内容做语义估算。

| Sample | Mode | Avg payload | Avg messages | Avg non-message | Request 1 non-message | Request 2+ non-message |
|---|---|---:|---:|---:|---:|---:|
| focused | Standard | 29,757 B | 8,070 B | 21,686 B | 21,676 B | 21,688 B |
| focused | R5 | 33,183 B | 12,221 B | 20,963 B | 13,973 B | 21,662 B |
| complex | Standard | 40,423 B | 18,730 B | 21,694 B | 21,676 B | 21,695 B |
| complex | R5 | 43,908 B | 23,338 B | 20,570 B | 13,973 B | 21,077 B |

focused R5 active request 2+ 的 21,662 B 相比 J6.6 的 22,496 B 下降 834 B/request（3.7%），
相比 J6.6 follow-up 的 22,107 B 下降 445 B/request（2.0%）。当前 R5 与 Standard 的固定区已经接近，
本轮 input 差距主要来自请求数及随自然历史增长的 message 区，而不是 TaskSpace 再平行携带一套正文。

## 5. Canonical 与反馈门禁

| Gate | Focused R5 | Complex R5 | Result |
|---|---:|---:|---|
| exact payload duplicate | 0 | 0 | pass |
| duplicate call/output record | 0 / 0 | 0 / 0 | pass |
| orphan call/output | 0 / 0 | 0 / 0 | pass |
| active projection max | 1 | 1 | pass |
| runtime boundary forbidden marker | 0 | 0 | pass |
| retention / salience | 100% / 100% | 100% / 100% | pass |
| protected miss / semantic replacement | 0 / 0 | 0 / 0 | pass |
| final task / open nodes | completed / 0 | completed / 0 | pass |

complex 两侧各有 1 个相同 output body hash，但 duplicate call/output record 均为 0。这表示不同真实工具
调用产生了相同正文，不是同一反馈被 canonical carrier 重复写入。TaskSpace 该重复正文为 219 B，
Standard 为 12 B；原始反馈保留，不由 Runtime 按语义删除。

所有 Map result 的 `sourceEventRef` 都是可直接解析的 `task-event-*`，不再带自定义前缀；terminal
完成后 task `activeMapId` 为空，全部 node/result 闭合。nested init action 继续形成独立 canonical
call/output pair，outer control 只返回 refs。

## 6. Complex trace 解释

complex R5 有 1 次 control protocol failure：首次 `initialize_then_actions` 参数存在尾随 `}`，工具返回
`invalid_arguments: trailing characters at line 1 column 981`。失败正文完整进入 canonical context，Agent
下一请求修正参数并成功初始化。这是模型生成的 JSON 语法错误及正常自恢复，不是 feedback 丢失或
Runtime 语义干预。

三个非终态 `finish_nodes` 都在同一 typed call 中携带 `next_node_id`，不是 standalone finish；terminal
使用 `finish_then_end`，extra final request 为 0。complex trace 还出现同一响应并行 4 个 `apply_patch`，
该行为属于已规划但尚未实施的 J7 singular patch carrier，不在 J6.7 中增加临时限制。

## 7. 历史对照边界

| Evidence | Standard req/input | R5 req/input | R5 active fixed | Req 2+ cache |
|---|---|---|---:|---:|
| J6.6 | 6 / 43,666 | 9 / 75,316 | 22,496 B | 87.30%（active warm 97.27%） |
| J6.6 follow-up | 8 / 57,857 | 11 / 90,412 | 22,107 B | active warm 97.08% |
| J6.7.6 focused | 7 / 50,000 | 11 / 91,297 | 21,662 B | 96.37% |

固定结构可以跨轮按最终 wire 机械比较；总 request/input 不能，因为 Standard 自身历史上为 5 到 9
requests，本轮 Agent 还选择了不同的发现、patch 和验证路径。J6.7.6 不把跨轮 token 差写成因果收益。

## 8. Phase Gate

| Exit item | Status |
|---|---|
| focused + complex correctness | passed |
| canonical duplication / orphan | passed / 0 |
| event codec round-trip | 100%（J6.7.1） |
| fixed/control-history duplicate reduction | passed |
| warm-cache regression不超过2pp | passed；两组均为正向约2.1pp |
| critical/high adversarial findings | superseded by J6.7.7-G final review |

J6.7.0-J6.7.5 已完成，J6.7.6 的工程与 live evidence 已完成。后续字段lineage审计发现：

- `finish_then_end.final_candidate`与assistant final正文完全相同；
- bootstrap outer `actions[]`与expanded native call参数重复；
- 595 B空Map developer message在初始化后继续作为旧epoch hard state出现；
- projection存在无界nodes/edges/goals风险；
- full snapshot占rollout约95%。

因此本报告不再作为J6.7最终关闭门。J6.7已重开J6.7.7，详细计划见
`30-r5-j6-7-phase7-context-residue-plan.md`；J7继续保持锁定。
