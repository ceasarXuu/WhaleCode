# R7 FLA-2 阻塞修复结果

- 日期：2026-07-21
- 状态：`repair_smoke_verified / pending_adversarial_reacceptance`
- 产品代码提交：`f3d07b824`
- 观察器修复提交：`30be4585e`
- 二进制 SHA256：`ed5ca605d86911f13c7ec16196046e7f556a9f2ae00a6fb4e4ff34e1f4673e1d`
- 机器结果：[`five-layer-fla2-blocker-repair-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla2-blocker-repair-result.json)
- COE：[`2026-07-20-21-24-r7-fla2-control-path-observability.md`](../../../coe/2026-07-20-21-24-r7-fla2-control-path-observability.md)

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

简单与复杂样本共 32 个 TaskSpace provider request：

| 合同项 | 结果 |
|---|---:|
| system message 恰为 2 条 | 32/32 |
| L1、L2、production manifest 精确匹配 | 32/32 |
| Map handle 恰为 1 个、role=user、位于 request tail | 32/32 |
| Control result 为 V2 | 14/14 |
| Initialize commit 包含 `node_bound` | 2/2 |
| 旧 nested transition 调用 | 0 |
| 非法 lifecycle 参数 | 0 |

遥测与 raw trace 对账：14 次 control 中 7 次提交、7 次 preflight reject；`committed_control_count`、
`graph_revision_commit_count` 和 `state_commit_count` 均为 7。另有 1 次初始化前普通工具 gate failure，独立统计。

## 3. Docker 配对结果

每个样本运行 1 个 pair，因此这里只是修复冒烟，不是效用结论。

| 样本 | 模式 | 结果 | Request | 工具 | Input | Cached | Uncached | Output | Request 2+ cache | Wall |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | solved | 6 | 10 | 67,766 | 65,664 | 2,102 | 1,284 | 96.57% | 14.08s |
| simple | TaskSpace | solved | 20 | 19 | 319,844 | 286,336 | 33,508 | 5,244 | 92.49% | 61.25s |
| complex | Standard | solved | 13 | 24 | 217,302 | 211,328 | 5,974 | 7,075 | 97.15% | 60.98s |
| complex | TaskSpace | solved | 12 | 16 | 187,252 | 167,936 | 19,316 | 5,826 | 89.18% | 57.42s |

复杂样本中 TaskSpace 少 1 个 request、少 8 个工具调用，并一次完成四文件 patch；简单样本则被 3 次
missing-sibling 和 4 次错误 patch 放大到 20 个 request。两者共同说明本轮机制修复有效，但单次 Agent 路径波动
仍足以主导成本，不能用均值替代 trace 分析。

## 4. Map 与反馈

| 样本 | 节点 | 边 | 开放叶 | Control 提交/失败 | `read_map` | redundant bind |
|---|---:|---:|---:|---:|---:|---:|
| simple | 5 | 4 | 0 | 4/3 | 0 | 0 |
| complex | 4 | 3 | 0 | 3/4 | 0 | 0 |

两次初始化成功后 Agent 都直接在已绑定节点下工作，没有再 bind 或为确认绑定而读取 Map。该结果证明反馈机制已补全；
但只有两个样本，不能把行为下降归因强度表述为统计结论。

## 5. 未解决问题

`H-003` 仍成立：`required_next_call` 已在 L2、action 描述和字段描述中明确为“声明，不会执行或调度”，但 JSON
Schema 只能约束一个 control 参数对象，不能结构性要求 provider response 再生成一个 top-level sibling。简单样本
出现 3 次、复杂样本出现 4 次 standalone control，均在事实型 V2 拒绝后自行纠正。

这不是反馈丢失，也没有证据支持继续增加 L2 提示或 Runtime 语义纠正。后续若处理，应作为独立 Tool 交互形状实验，
明确评估结构表达、请求成本与 Runtime 边界，不能混入本次 blocker 修复结论。

## 6. 结论

B1、B2、旧 L4 discriminator、V2 binding 事实和观测少报均已通过代码测试与真实请求双重验证。FLA-2 从
`acceptance_blocked` 前进到 `repair_smoke_verified`；正式恢复 `active_verified` 仍需要独立对抗性复审，且不得把
尚未关闭的 H-003 隐去。
