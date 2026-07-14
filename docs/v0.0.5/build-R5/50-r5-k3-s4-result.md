# R5-K3 S4 远端详情折叠实验结果

- Created: 2026-07-14
- Status: REJECTED / S4.1 reverted to S4.0
- Candidate commit: `afcae13`
- Fallback commit: `64f54b4`
- Baseline: R5-K-B0
- Previous: S4.0 `7040547`
- Scope: S4单策略实现、deterministic probe、Docker new-epoch active-prefix各3次

## 1. 结论

S4.1不接受，production已整体回退到S4.0。fold和`expand_nodes`均未作为禁用分支、feature flag或半套能力保留。

拒绝原因不是拓扑或replay正确性失败。工程测试证明S4可以机械地保留全部节点和边、折叠远端详情并由Agent单向
展开；问题在于它与现有B0详情分层重复，无法形成有效压缩收益：

1. B0对距离3以上的completed节点已经使用D3，只保留最近1条普通详情和受保护事件；
2. S4也从距离3开始，把这条已经压缩的详情替换为`detail_state`、距离、ref和hash；
3. ref/hash元数据几乎抵消被移除详情，`detail_state=full`还给所有未折叠节点增加固定成本；
4. 自然active-prefix没有依赖边，三次live中S4激活和fold均为0，不能证明Agent侧实际收益；
5. 零激活矩阵正确性和warm cache通过，但S4.1相对S4.0的wall中位数为1.161x，超过1.10门限。

因此S4当前不是“收益尚不确定”，而是已证明压缩对象与B0重叠、典型路径净字节为负。不得通过降低距离阈值、
增加Runtime语义判断或构造答案型prompt重新包装同一策略。

## 2. 工程正确性

S4.1候选在回退前通过：

| Gate | Result |
|---|---:|
| `codex-core action_map::` | 55/55 PASS |
| `taskspace_control` | 24/24 PASS |
| `codex-tools taskspace_tool` | 3/3 PASS |
| `cargo check -p codex-core -p codex-tools` | PASS |
| snapshot restore | 20 cycles PASS |
| invalid expand atomicity | zero partial commit PASS |
| skeleton over budget | explicit error PASS |

回退到S4.0后重新执行同范围测试和check，结果继续通过。工程正确性只证明机制可实现，不替代收益门禁。

## 3. Deterministic收益

探针使用5节点、4条链式依赖、最后节点active；4个完成节点各记录4条普通读取证据。它不包含答案、模型提示、
压缩阈值修改或Runtime激活触发器。

| Metric | S4内部before | S4 after | Delta |
|---|---:|---:|---:|
| 全部projection bytes | 3175 | 3164 | -11 (-0.35%) |
| node-detail bytes | 2156 | 1942 | -214 (-9.93%) |
| skeleton bytes | 1182 | 1182 | 0 |
| visible nodes | 5 | 5 | 100% |
| visible edges | 4 | 4 | 100% |
| eligible / folded nodes | 1 | 1 | 100% |

`S4内部before`已经包含5个` detail_state=full`，而S4.0/B0不输出该字段。该固定文本每节点18 bytes；扣除
90 bytes后，可比S4.0为3085 bytes，S4 after为3164 bytes，即净增加79 bytes（+2.56%）。自然prefix的
4节点结果也独立验证同一固定增量：S4.0为1963 bytes，零fold的S4.1为2035 bytes，正好增加72 bytes。

根因位于现有`projection_node_details`合同：D1最多8条、D2最多4条、D3最多1条普通事件；S4的
`distance >= 3`与D3完全重合。增加同一节点的普通历史事件不会线性增加S4可删除正文，因为B0在进入S4前已经
把它们收敛成1条。

探针artifact：`target/r5-map-compression/S4-deterministic-probe/probe.json`。

## 4. Docker矩阵

自然prefix在resume后先compact，确保new epoch实际构造一次projection。每个arm执行3次，全部final validator通过。

### 4.1 B0对S4.0

该矩阵证明无行为基建保持B0合同。表中数值均为`总和 / 均值 / 中位数`。

| Arm | Success | Requests | Input | Cached | Uncached | Output | Wall ms | Projection | Req2+ cache 加权/均值/P50 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| B0 | 3/3 | 26/8.67/8 | 280895/93631.67/74318 | 268672/89557.33/71552 | 12223/4074.33/3724 | 9067/3022.33/2735 | 85537/28512.33/24865 | 5889/1963/1963 | 95.64/95.64/95.61% |
| S4.0 | 3/3 | 20/6.67/6 | 197674/65891.33/62066 | 188160/62720/58752 | 9514/3171.33/3314 | 6754/2251.33/2534 | 65351/21783.67/25889 | 5889/1963/1963 | 95.74/95.66/95.60% |

S4.0与B0 projection逐字节等长，Req2+ cache加权差`+0.10pp`。模型动作有随机差异，但没有固定上下文成本。

### 4.2 Standard、S4.0与S4.1

| Arm | Success | Requests | Input | Cached | Uncached | Output | Wall ms | Projection | Req2+ cache 加权/均值/P50 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3/3 | 26/8.67/7 | 227109/75703/58553 | 204800/68266.67/55424 | 22309/7436.33/8233 | 6519/2173/2013 | 70774/23591.33/21201 | n/a | 95.30/95.18/95.59% |
| S4.0 | 3/3 | 38/12.67/14 | 484778/161592.67/188694 | 465792/155264/181504 | 18986/6328.67/7190 | 12500/4166.67/4939 | 126572/42190.67/48798 | 5889/1963/1963 | 96.24/96.33/96.26% |
| S4.1 | 3/3 | 27/9/9 | 332797/110932.33/104476 | 307840/102613.33/96512 | 24957/8319/9884 | 17852/5950.67/6484 | 159174/53058/56647 | 6105/2035/2035 | 96.32/96.33/96.27% |

S4.1相对S4.0：requests中位数`0.643x`、input`0.554x`、Req2+ cache`+0.08pp`，但wall中位数
`1.161x`、output总量`1.428x`、projection固定成本`1.037x`。由于fold为0，request/input下降不能归因于压缩；
wall超过预注册1.10门限，零激活回归门不通过。

Artifacts：

- `target/r5-map-compression/S4-active-b0-control/summary-v2.json`
- `target/r5-map-compression/S4-active-zero-regression/summary-v2.json`

## 5. Live激活缺口

既有自然prefix包含4个Agent创建的节点，但没有依赖边。S4按合同对不可证明距离的节点不折叠，因此3/3中
`activation=0`、`folded=0`是正确行为，不是Runtime故障。

另执行一次明确描述真实工程前后依赖、但不提TaskSpace或S4的探索任务。Agent只初始化1个节点，并在后续响应中
输出多个patch文本而没有调用patch工具，任务未完成。该case既不是有效S4样本，也没有进入统计。它暴露的是当前
Map粒度/依赖使用不足，不能通过Runtime替Agent补边来制造S4激活。

由于complex live“至少一次真实fold”门未满足，S4即使没有确定性负收益也不能accept。

## 6. 边界与后续

S4从未被视为上下文超限根治方案。它不减少root、node skeleton、edges或active frontier；Map持续增长后，最小
骨架仍可能超过provider限制。当前继续保留显式`map_skeleton_over_budget`，不分页、不删除节点、不生成Runtime摘要。

S4被拒绝后，K4/K5保持不可进入，当前没有accepted压缩策略。下一步应先重新审计B0现有D1/D2/D3详情分层和真实
Map拓扑分布，再提出与B0不重叠的新单策略；不得直接把S4改名后重跑。
