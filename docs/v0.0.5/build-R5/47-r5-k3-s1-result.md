# R5-K3 S1 单策略结果

- 日期：2026-07-14
- 策略：`S1 = completed_inactive_leaf_batch_archive_projection`
- 候选代码：`69e8113`
- 候选 binary SHA-256：`711789a609c3f08363ed7f027f386693093d288316189b79c9cf0570a0c54473`
- 判定：`REVISE`
- 后续：暂停；不得进入 S2

> 该文档保留首次 live 校准未激活的历史结论。后续已用自然 Agent 轨迹构建 active-prefix 正式样本，S1 实际
> 激活后因工程成本负收益判定为 `REJECTED`。最终结果见
> `48-r5-k3-s1-natural-prefix-result.md`。

## 1. 结论

S1 的 production slice、可逆 archive、hash 校验、读取引用和 100/1,000/10,000 节点规模测试通过，canonical Map
没有变化。但预登记 live gate 没有通过：所有有效 live 运行的 `strategy_activation_count` 均为 0，无法计算
`activated_projection_bytes` 相对 B0/P0 的收益比，也没有资格执行正式 3 次矩阵。

这不是用一次失败样本否定压缩实现。当前阻塞是实验前提错误：projection 只在已有 compaction/resume epoch 中
构造，S1 又要求同一 active Map 内至少 3 个已完成、无边、无 lease 的节点。固定 token 阈值与节点生命周期无关，
不能稳定命中两者交集；已 terminal 的 Map 在 resume 后也不再是 active projection 的对象。

## 2. 已通过证据

| 维度 | 结果 |
|---|---|
| runtime S1 lifecycle | `25/25` 通过 |
| projection/archive codec | `3/3` 通过 |
| output ref读取与hash校验 | `8/8` 通过 |
| rollout reconstruction | `30/30` 通过 |
| long replay | `2/2` 通过 |
| initial context相关过滤集 | `16/16` 通过 |
| observer synthetic fixture | 通过 |
| production scale round-trip | 100 / 1,000 / 10,000节点全部通过 |
| canonical Map | archive前后node、edge、result与payload hash保持一致 |
| provider-visible无激活路径 | 与B0保持原projection格式；不输出archive section |

S1 仅修改 provider projection。archive 是由 canonical Map 派生的内存内容寻址载荷；缺失、损坏或hash mismatch
显式失败，不做partial view或静默回退。Runtime不生成自然语言摘要，不修改Agent状态，不新增tool action/schema。

## 3. Live校准结果

| 校准 | 结果 | S1激活 | 判定 |
|---|---|---:|---|
| 普通单轮，原复杂样本 | B0/C均成功；C为14 requests、178,520 input、67,902 ms | 0 | 无projection epoch，不能测收益 |
| 10K强制compaction | C出现5个epoch但仅1个completed节点；B0路径退化 | 0 | 阈值过早，作废 |
| 15K强制compaction | C成功但23 requests、243,407 input、103,183 ms；B0超过34 requests后终止 | 0 | 仍未命中eligibility，作废 |
| 两轮live continuation | B0/C与validators均成功；C为35 requests、510,712 input、123,284 ms | 0 | 首轮Map已terminal，resume创建新active Map |
| staged单轮 + 13K | B0成功；C在第28次请求仍停留首节点后终止 | 0 | prompt不能约束节点生命周期，作废 |

上述均为一次校准，不是正式三次收益矩阵。两次主动终止只用于阻止已识别的无效循环继续消耗；对应 artifact
分别保存在 `target/r5-map-compression/S1-compaction-smoke-r4` 和
`target/r5-map-compression/S1-staged-smoke`。有效单臂 performance observation 标记已运行侧为 `complete`、
未运行占位侧为 `skipped`；不得把占位零值当作测量。

## 4. 根因

1. fresh session 在没有compaction时保留自然上下文，不会为每个请求重建 `ContextProjectionV1`；
2. S1只在 projection 构造时评估，且最低需要3个completed、非current、无边、无lease节点；
3. token compaction阈值不知道Map生命周期，模型也可能在单个节点内消耗数十次请求；
4. 正常final response会将当前Map和Task置为completed并清除`active_map_id`；
5. resume虽恢复旧Map，但新用户任务绑定新active Map，旧completed Map只进入task inventory，不进入active projection。

因此，继续调阈值或提示词只是在寻找随机命中，不是可信实验。给Runtime新增“完成3节点就压缩”的触发器则会把
第二个行为策略混入S1，并违反Runtime边界。

## 5. 修订入口

S1下一次候选不改production语义，先补一个机械且可审计的 active-map live continuation fixture：

1. TaskSpace历史在同一active Map中固定包含至少3个S1 eligible completed节点和1个未完成工作节点；
2. Standard获得逐事件等价的自然顺序历史，不获得TaskSpace额外语义；
3. canonical event、Map、prompt、fixture与provider profile全部记录hash和equivalence receipt；
4. continuation由真实Agent完成代码任务和validators，synthetic只负责前置状态，不替代live执行；
5. 不设置低token阈值，不新增Runtime触发器；先1次冒烟，确认激活后再执行STD/B0/C各3次。

该 fixture 和等价性证明后来已经完成；本页的修订入口不再是当前状态。最终判定以
`48-r5-k3-s1-natural-prefix-result.md` 为准，S2 仍保持 `unselected`。
