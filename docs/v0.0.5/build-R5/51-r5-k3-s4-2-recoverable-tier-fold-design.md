# R5-K3 S4.2 可恢复详情分级折叠设计

- Created: 2026-07-14
- Status: IMPLEMENTED / MECHANISM VERIFIED / LIVE BENEFIT UNVERIFIED
- Previous: S4.0 `7040547`
- Supersedes candidate: S4.1 `afcae13`（已由`64f54b4`回退）
- Scope: 只补全B0详情分级的显式折叠与Agent展开机制
- Implementation: `080ed60`, `935afe1`, `c7aa795`
- Result: `52-r5-k3-s4-2-result.md`

## 1. 问题修正

B0已经按节点与活跃前沿的距离选择详情：D1最多8条、D2最多4条、D3最多1条，P0事件始终保留。
S4.1错误地把B0已经选出的D3详情再次删除，再用state/ref/hash替换，因此既与B0重叠，也没有恢复B0未选中的
历史详情。

S4.2不在B0之后增加第二级裁剪。它把B0在`distance >= 3`节点上未展示的详情变成显式、可验证、可由Agent
恢复的折叠内容。B0选择器是S4.2的默认可见集合，不再是S4.2的输入后还要继续删除的集合。

## 2. 唯一产品行为

对满足原S4拓扑条件的节点，Runtime机械计算：

```text
all_details     = 节点全部canonical详情事件引用
baseline_visible = B0 D3选择出的详情事件引用
hidden_details  = all_details - baseline_visible
```

随后只允许三种结果：

1. `hidden_details`为空：保持B0输出，不增加任何状态字段；
2. hidden正文大于折叠标记：保持`baseline_visible`，并在节点上增加
   `detail_state=folded hidden_event_count=<n> detail_ref=<content-addressed-ref>`；
3. hidden正文不大于折叠标记：直接展示`all_details`，不折叠，避免负压缩。

Agent只能对实际显示为`folded`的节点调用`taskspace_control.expand_nodes`。成功后写入canonical
`NodeDetailExpanded`事件，工具结果立即返回被恢复的事件引用；以后所有projection对该节点展示`all_details`，并显示
`detail_state=expanded expansion_event_id=<id>`。不存在refold、importance、Runtime相关性判断或自然语言摘要。

## 3. 与B0的关系

| 项目 | B0 | S4.2 |
|---|---|---|
| 默认远端可见详情 | D3选择结果 | 原样保留D3选择结果 |
| 未展示详情 | 静默不进入projection | 显式fold引用或因不经济而全文展示 |
| Agent恢复入口 | 无 | `expand_nodes` |
| 恢复后的projection | 不适用 | 该节点全部canonical详情引用 |
| 普通节点固定状态字段 | 无 | 无 |

因此S4.2的收益目标不是让projection小于B0。B0已经通过静默省略取得更小字节数，不能把语义不可恢复当作性能优势。
S4.2的目标是在接近B0成本的前提下，把这部分省略升级为可观察、可恢复的压缩。

## 4. 硬边界

1. 所有canonical node和edge继续全局可见；
2. root、未完成节点、当前节点、活跃租约节点、距离未知或不可达节点不得折叠；
3. `importance`参数、属性和同义替代字段均禁止；
4. Runtime不得分析详情内容、推断重要性、生成摘要或替Agent展开；
5. fold只作用于派生projection，canonical事件不删除、不改写；
6. 展开事件不可撤销、可snapshot/replay，创建节点时不能预声明expanded；
7. `detail_ref`是hidden事件引用集合的内容寻址身份，hash不重复暴露为第二个字段；
8. 展开批次必须全量校验后原子提交，失败时`state_commit=none`；
9. expanded详情导致projection超预算时显式报错，不自动重新折叠；
10. S4.2不压缩node/edge骨架，不解决最终`map_skeleton_over_budget`。

## 5. 验收

### 5.1 确定性合同

- 无hidden详情：provider-visible projection与S4.0逐字节一致；
- 有hidden详情：B0可见事件ID在S4.2中100%保留，hidden集合100%准确；
- 只有`fold_marker_bytes < hidden_detail_bytes`时才fold；
- fold前的语义全集、展开工具返回和展开后projection事件ID 100%一致；
- snapshot/restore 20轮后expanded状态与事件集合无漂移；
- invalid mixed expand批次零partial commit；
- 全部node/edge可见率100%。

### 5.2 Live矩阵

每个arm至少3次：

| Sample | Arms | 目的 |
|---|---|---|
| simple | Standard / B0 / S4.0 / S4.2 | 无hidden路径零固定开销、正确性和成本回归 |
| complex | Standard / B0 / S4.0 / S4.2 | 自然形成依赖链、真实fold/expand和任务正确性 |

必须同时报告request、input/cached/uncached/output token、Req2+ cache、wall、Map节点/边、B0 visible、hidden、folded、
expanded、projection bytes、相对full-details压缩量和相对B0元数据成本。若complex live仍未自然形成可折叠节点，
deterministic只证明机制正确，不能替代Agent侧收益结论。

## 6. 回退

S4.2的fold projection、`expand_nodes`工具面和expanded full-detail projection作为一个原子策略提交。任一合同失败，
整体revert回S4.0；不保留feature flag、禁用工具、半套schema或兼容路径。
