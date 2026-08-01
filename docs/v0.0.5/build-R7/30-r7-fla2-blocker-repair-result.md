# R7 FLA-2 阻塞修复结果

- 日期：2026-07-21
- 状态：`active_verified / adversarially_reaccepted`
- 当前产品源码提交：`6ebe2c679`
- 观察器修复提交：`30be4585e`
- 证据新鲜度 gate 提交：`4baec0710`
- 二进制 SHA256：`d8e20fe3eaac8b8fc25982debd09e2de17ce75d5efe2d4eb564e873876910222`
- 机器结果：[`five-layer-fla2-blocker-repair-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla2-blocker-repair-result.json)
- COE：[`2026-07-20-21-24-r7-fla2-control-path-observability.md`](../../../coe/2026-07-20-21-24-r7-fla2-control-path-observability.md)
- 对抗审查：[`2026-07-20-r7-fla2-l1-l2-effectiveness-review.md`](../../../vs_review/2026-07-20-r7-fla2-l1-l2-effectiveness-review.md)

## 1. 修复范围

本轮关闭原 FLA-2 对抗审查的两个 blocker，并同步处理已确认的 Tool 与观测缺口：

1. `map-request` Map handle 不再作为第三条静态 system 消息；每次请求从 canonical 状态重新构造，并放在当前
   user tail。
2. `taskspace_control` 删除旧 `transition_node + transition` discriminator，生命周期操作改为直接 action。
3. 所有 control 成功和拒绝统一为 `TaskSpaceControlResultV2`；preflight 拒绝也返回 action、revision、
   `state_commit=false`、actual 和 expected。
4. 初始化成功结果除 `map_initialized` 外，明确返回 `node_bound` 事实。
5. 观察器分别统计 preflight、handler、ordinary gate、commit 和 graph revision，不再用旧 result schema 或虚构
   action 名少报。

这些修复没有让 Runtime 推断 Agent 意图、选择节点、补写参数或自动执行后续动作。

## 2. 请求级验收

当前 Base Tool wire 清理后的简单与复杂样本共 21 个 TaskSpace provider request：

| 合同项 | 结果 |
|---|---:|
| system message 恰为 2 条 | 21/21 |
| TaskSpace Base 2.0.1、L2 v2.1、manifest 1.0.2 精确匹配 | 21/21 |
| Map handle 恰为 1 个、role=user、位于 request tail | 21/21 |
| Standard TaskSpace 零注入 | 17/17 |
| Control result 为 V2 | 13/13 |
| Initialize commit 包含 `node_bound` | 2/2 |
| 旧 nested transition 调用 | 0 |
| 非法 lifecycle 参数 | 0 |

遥测与 raw trace 对账：13 次 control 中 8 次提交、5 次 preflight reject；`committed_control_count`、
`graph_revision_commit_count` 和 `state_commit_count` 均为 8，没有初始化前普通工具 gate failure。

证据新鲜度 gate 同时核对当前 Codex source commit、候选二进制 SHA、binary attestation、机器结果 identity、每个
run 的 binary health 和每条 `payload_captured` provider trace。旧 evidence 在该 gate 下稳定失败；当前 evidence
输出 `status=pass` 且 `findings=[]`。机械报告位于
`target/r7-five-layer/fla2-current-identity-reacceptance/evidence-freshness.json`。

## 3. Docker 配对结果

每个样本运行 1 个 pair，因此这里只是修复冒烟，不是效用结论。

| 样本 | 模式 | 结果 | Request | 工具 | Input | Cached | Uncached | Output | Request 2+ cache | Wall |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | solved | 6 | 8 | 65,013 | 63,488 | 1,525 | 1,098 | 97.37% | 13.60s |
| simple | TaskSpace | solved | 11 | 12 | 156,630 | 148,224 | 8,406 | 3,476 | 94.24% | 34.52s |
| complex | Standard | solved | 11 | 17 | 163,140 | 158,208 | 4,932 | 6,584 | 96.88% | 56.43s |
| complex | TaskSpace | solved | 10 | 16 | 157,404 | 137,856 | 19,548 | 6,090 | 86.78% | 48.77s |

复杂样本中 TaskSpace 少 1 个 request、少 1 个 Runtime tool，并一次完成四文件 patch；简单样本有 2 次
missing-sibling 和 1 次失败 patch，TaskSpace 比 Standard 多 5 个 request。两者继续说明单次 Agent 路径足以主导
成本，不能把一次 sample 的成本方向当作机制收益。

## 4. Map 与反馈

| 样本 | 节点 | 边 | 开放叶 | Control 提交/失败 | `read_map` | redundant bind |
|---|---:|---:|---:|---:|---:|---:|
| simple | 5 | 4 | 0 | 4/2 | 0 | 0 |
| complex | 5 | 4 | 0 | 4/3 | 0 | 0 |

两次初始化成功后 Agent 都直接在已绑定节点下工作，没有再 bind 或为确认绑定而读取 Map。该结果证明反馈机制已补全；
但只有两个样本，不能把行为下降归因强度表述为统计结论。

## 5. 未解决问题

`H-003` 仍成立：`required_next_call` 已在 L2、action 描述和字段描述中明确为“声明，不会执行或调度”，但 JSON
Schema 只能约束一个 control 参数对象，不能结构性要求 provider response 再生成一个 top-level sibling。简单样本
出现 2 次、复杂样本出现 3 次 standalone control，均在事实型 V2 拒绝后自行纠正。

这不是反馈丢失，也没有证据支持继续增加 L2 提示或 Runtime 语义纠正。后续若处理，应作为独立 Tool 交互形状实验，
明确评估结构表达、请求成本与 Runtime 边界，不能混入本次 blocker 修复结论。

## 6. 结论

B1、B2、旧 L4 discriminator、V2 binding 事实、观测少报、当前源码 identity 和 raw-count 新鲜度均已通过代码
测试、真实请求、机械 gate 和独立对抗性复验。Round 4 verdict 为 `pass_reacceptance`，FLA-2 已恢复
`active_verified`，可以作为 FLA-3 的已验证前置阶段。H-003 仍明确保持 open，不属于本次 L1/L2 验收关闭范围。
