# R5-K3 S4 远端节点详情折叠设计

- Created: 2026-07-14
- Updated: 2026-07-14
- Version: 1.2
- Status: Evaluated / Rejected / S4.1 reverted to S4.0
- Owner / Responsible: WhaleCode core runtime / TaskSpace Map
- Related Systems: canonical Map、projection、`taskspace_control`、checkpoint/delta、resume/replay、benchmark
- Related Links: `31-r5-map-native-context-compression-charter.md`、
  `48-r5-k3-s1-natural-prefix-result.md`、
  `50-r5-k3-s4-result.md`、
  `benchmarks/taskspace/map-compression/s4-distance-fold-contract.json`
- Risk Level: High
- Plan Type: Full

## 1. 决策摘要

> 2026-07-14实施后结论：S4.1被拒绝并整体回退到S4.0。B0对`distance >= 3`节点已执行D3详情
> 分层，通常只保留1条普通详情；S4新增的state/ref/hash成本大于进一步删除该详情的收益。完整证据见
> `50-r5-k3-s4-result.md`。以下保留原设计合同，用于审计被验证和被否定的假设，不代表当前production行为。

S1、S2、S3全部废弃。三者共同把历史节点或子图替换为archive/macro，削弱了TaskSpace projection持续提供
全局Map视野的产品目标。S4不归档节点、不移除边、不分页全局骨架，只折叠远离活跃前沿的节点局部详情。

S4的完整产品合同是：

1. root、全部nodes、全部edges和active frontier始终可见；
2. 只有已完成、非root、距任一活跃节点至少3条图边、且从未被Agent展开过的节点可以折叠；
3. 折叠节点明确显示`detail_state=folded`，保留目标、状态、拓扑、结果引用和精确详情引用；
4. Agent只能对已经折叠的节点执行`expand_nodes`；
5. 展开会为节点写入canonical `NodeDetailExpanded`事件；
6. 存在该事件的节点此后永久保持详情展开，Runtime不得再次折叠；
7. Agent不能在初始化或创建节点时声明已展开，不能主动折叠、撤销展开或重新折叠节点；
8. Runtime不解释Agent为什么展开，不生成摘要，不重写语义。

自动折叠与Agent展开是一个不可拆开的安全合同，不是两个可独立上线的压缩启发式。只上线自动折叠会造成
Agent无法恢复信息；只上线展开没有可操作对象。因此可以先落无行为schema基建，但provider-visible行为必须原子启用。

### 1.1 能力边界

S4是上下文压缩优化，不是上下文超限的根治方案。它只减少`node-local details`，不会减少：

- root和用户约束；
- 每个node的骨架行；
- canonical edges；
- current node和active frontier；
- provider tool schema及其他自然历史。

因此随着Map持续增长，最终可能出现仅nodes/edges组成的最小骨架就超过provider上下文限制。S4对此保持现有
`map_skeleton_over_budget`显式失败，不分页、不删节点，也不引入临时fallback。骨架超限需要后续独立策略，当前不定义
其目标模型、触发条件或实现路径，避免和S4的收益及回归混在一起。

## 2. 问题与根因

S1证明了archive可以减少projection bytes，但也证明“把刚完成节点从全局视图替换成archive index”会降低证据
显著性。复杂样本中Agent增加Git重查和验证，requests、input和wall均出现明显负收益。S2继续扩大archive覆盖范围，
S3则用checkpoint作为archive边界；它们仍然沿用“节点从全局Map消失”的基本方向，不能解决这一产品冲突。

新设计不再问“哪些节点可以从Map中拿走”，而只问“哪些远端节点可以暂时少展示局部详情”。这使压缩对象从
Map拓扑收敛为node-local projection，canonical Map和Agent全局视野不变。

## 3. 术语与状态模型

### 3.1 工作流状态与详情状态分离

现有`NodeStatus = pending | ready | running | blocked | completed`只描述任务生命周期。S4不得增加`folded`到
`NodeStatus`，否则会把显示状态混入状态机。

S4不增加`importance`或其他语义属性，只增加一个canonical事件类型和一个projection字段：

| Item | Owner | Values | Persistence |
|---|---|---|---|
| `NodeDetailExpanded` | Agent通过`expand_nodes`触发 | node ID、source call/event IDs | canonical event、snapshot、delta、replay |
| `detail_state` | Runtime机械派生 | `full` / `folded` / `expanded` | 每次projection计算，不作为工作流状态写回 |

派生关系：

```text
存在该节点的 NodeDetailExpanded 事件  -> detail_state=expanded
不存在展开事件 && fold_eligible       -> detail_state=folded
不存在展开事件 && !fold_eligible      -> detail_state=full
```

`detail_state=expanded`只表达可验证事实“Agent曾主动展开该节点”，不解释该动作的动机或语义。

### 3.2 root定义

当前数据模型没有单一`parent_id`，依赖关系由`MapEdge(from -> to)`表达。S4保护两类root：

1. Task root及其原始用户约束；
2. Map中入度为0的全部graph root nodes。

存在多个起点时全部视为root。无边Map中所有节点均为graph root，因此不会被折叠；Runtime不得用创建顺序虚构
线性路径。该结果同时把Map坍缩或缺边问题暴露出来，而不是由压缩层掩盖。

### 3.3 活跃前沿与N-3

活跃前沿集合为：

```text
status in {pending, ready, running, blocked} 的全部节点
+ current_node（若存在且有效）
```

节点距离定义为：沿Map边忽略方向后，到任一活跃前沿节点的最短边数。使用全部活跃节点而不是只使用
`current_node`，避免并行分支中一个分支的近端历史被另一个分支误判为远端。

“N-3以上”冻结为`minimum_frontier_distance >= 3`：距离0、1、2的节点保持完整；距离3及更远的节点才可能折叠。
距离无法证明、节点不可达或活跃前沿为空时，节点不满足折叠条件。阈值3是S4唯一初始值，后续若调整必须登记为
新的独立策略，不得在S4测试后修改阈值追求通过。

## 4. 折叠资格

节点必须同时满足以下条件：

1. `NodeStatus == completed`；
2. 不是Task root或graph root；
3. 不是`current_node`或active frontier；
4. 没有active lease，也不存在指向该节点的未释放lease；
5. 到全部活跃前沿的最小图距离至少为3；
6. 距离可由当前canonical edges证明；
7. canonical事件中不存在该节点的`NodeDetailExpanded`。

Runtime只做上述布尔判断。它不得因为节点包含失败、代码读取、测试、结论或某种工具类型而决定是否永久保留。
这些信息在节点接近前沿时已经忠实可见；节点折叠后是否展开，由Agent决定。

## 5. Projection合同

### 5.1 全局骨架不折叠

无论节点详情是否折叠，provider projection都必须保留：

- root task和root source refs；
- 每个canonical node的一行骨架；
- 每条canonical edge；
- current node和全部active frontier；
- 每个节点的`id/kind/goal/status/result_ids/event_count`；
- 每个节点的`detail_state`。

S4禁止`archive_nodes`、macro node、covered node count替代、全局分页或“其余节点请读取引用”等节点级隐藏。

### 5.2 折叠的对象

折叠只作用于B0原本会投影的node-local details，不改变canonical event/result，不改变普通tool反馈，也不改变
节点骨架。`folded`节点额外暴露：

```text
detail_state=folded
frontier_distance=<mechanical integer>
detail_ref=<exact node-local projection ref>
detail_sha256=<exact payload hash>
```

`detail_ref`指向该节点按B0规则本应展示的完整node-local detail payload。Runtime不得为它写自然语言summary、关键词、
结论或展开原因解释。折叠前后的payload hash必须可机械验证。

### 5.3 展开后的可见内容

展开节点恢复B0对该节点的完整node-local detail projection，并显示：

```text
detail_state=expanded
expansion_event_id=<canonical event id>
```

“完整”以B0现有投影合同为准，不借S4扩大成无界原始tool output。已有`raw_ref`和渐进读取合同继续负责超长单项
正文，S4不修改其裁剪策略。

## 6. Agent工具合同

只扩展现有`taskspace_control`，不增加架构层。active tool新增一个action variant：

```json
{
  "action": "expand_nodes",
  "node_ids": ["inspect-old-api", "verify-old-schema"]
}
```

合同要求：

1. `node_ids`至少1项、非空、唯一；
2. 整批节点在动作执行时都必须处于`detail_state=folded`；
3. 先全量验证，再原子提交，任一非法则整批`state_commit=false`；
4. 成功后为每个节点追加一条canonical `NodeDetailExpanded`事件；
5. tool result只返回已提交node IDs、expansion event IDs、detail ref/hash和当前Map机械状态，不生成解释；
6. 下一次projection恢复这些节点的B0详情；
7. sibling ordinary tools仍遵守现有state barrier，但不能假设在同一provider response中读取尚未返回的展开结果。

以下形态在schema中不可表达：

- initialize/create/finish-next-create携带`detail_state`、`expanded`或expansion event；
- `fold_node`、`collapse_node`、`reset_expansion`、`refold_node`；
- 对`full`、`expanded`、root、active或距离不足3的节点执行expand；
- Runtime自动expand、删除或重置Agent展开事件。

Agent只能完成一次单向状态变化：

```text
folded + no NodeDetailExpanded event
  -- taskspace_control.expand_nodes -->
expanded + canonical NodeDetailExpanded event
```

## 7. Canonical与Replay设计

S4不增加Map节点语义属性。Agent展开通过canonical事件保存，实施时需要：

1. 新增一等`NodeDetailExpanded` runtime event，记录node ID、Agent call ID和source event ID；
2. 新节点不携带展开字段，Agent输入schema也不提供预先展开能力；
3. snapshot、delta、checkpoint、resume、fork和replay必须保留并重放该事件；
4. projection通过canonical事件索引判断节点是否曾被展开；
5. S4折叠本身不写canonical mutation event，`detail_state`每次由同一Map状态确定性派生；
6. 相同canonical snapshot必须产生相同折叠集合、detail refs和hash。

不保留旧snapshot兼容。项目没有需要迁移的用户数据；测试fixture和实验artifact按新schema重新生成。

## 8. 实施阶段

### R5-K3-R0：清除废弃策略

- Entry：本设计和机器合同已提交，尚未实现S4。
- Tasks：删除S1 archive production selection、projection archive模块接线、S1日志和active tool读取特例；保留历史报告，
  测试只保留可复用的hash/ref基础能力时必须改成中性命名。
- Independent validation：provider projection与S1前匹配版本P1一致；`strategy_activation_count=0`；S1/S2/S3生产符号扫描为0。
- Exit：没有archive/macro production path、feature flag、兼容adapter或dormant S1代码。
- Fallback：回退R0主题commit；不得同时开始S4以掩盖删除回归。

### R5-K3-S4.0：无行为状态与观测基建

- Entry：R0完成。
- Tasks：增加`NodeDetailExpanded` canonical事件、snapshot/delta/replay；增加`detail_state`内部计算器、距离fixture和日志，
  但production projection仍固定输出B0内容，active tool尚不暴露expand。
- Independent validation：B0与S4.0 provider-visible body/tool schema逐字节一致；snapshot round-trip和20-cycle replay保持
  expansion event不丢失；activation=0。
- Exit：state/schema/logging完整，production行为零变化。
- Fallback：回退S4.0，不增加兼容字段。

### R5-K3-S4.1：原子启用折叠与展开

- Entry：S4.0完成且行为等价证据通过。
- Tasks：一次性启用distance fold projection、`expand_nodes` schema/handler、展开事件提交和folded detail ref读取；不修改阈值、
  普通tool反馈、系统提示词、compaction或Agent行为引导。
- Independent validation：执行第10节全部deterministic和live矩阵。
- Exit：正确性、语义、拓扑、展开、成本和缓存门禁全部通过；单独产出accept/reject报告并暂停。
- Fallback：整体回退S4.1到S4.0；不得只关掉expand或只留下fold，也不得保留长期feature flag。

### R5-K4：长会话稳定性

- Entry：S4.1被accept。
- Tasks：20轮append/finish/fold/expand/resume，覆盖并行frontier、fork、crash recovery和多个已展开节点。
- Exit：展开事件不可逆、fold集合确定性、detail hash和全局骨架均100%；无漂移、partial commit或自动refold。
- Fallback：回退S4.1，保留显式`map_projection_over_budget`。

## 9. 实现完整性矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Status |
|---|---|---|---|---|---|---|
| retire S1-S3 | production无archive策略 | `action_map/runtime.rs`, `projection_archive.rs` | active projection | P1 body parity、forbidden scan | activation=0 | planned |
| canonical expansion event | 只持久化Agent expand事实 | protocol event, snapshot/replay | `taskspace_control` commit | round-trip/delta/fork | expansion event trace | planned |
| fold selector | 只选completed、non-root、distance>=3、无展开事件节点 | projection selector | active projection | graph matrix | eligible/ineligible reason | planned |
| folded renderer | 全节点/边保留，仅详情变ref | `projection.rs` | provider context | skeleton/hash fixture | bytes before/after | planned |
| expand tool | folded节点原子写入展开事件 | `taskspace_tool.rs`, handler, runtime | active tool | schema/atomic failure | requested/committed/failed | planned |
| benchmark | 对比Standard/B0/S4.0/S4.1 | Docker runner/analyzer | live samples | 3 repeats/arm | request/token/cache/wall/map | planned |

## 10. 测试与收益门禁

### 10.1 Deterministic fixtures

至少覆盖：

1. chain图中distance 0/1/2保持full，3/4折叠；
2. 多active frontier取最小距离；
3. graph root即使距离>=3仍full；
4. pending/ready/running/blocked节点永不折叠；
5. 无边、断开或空frontier时距离不可证明，节点不折叠；
6. folded marker、detail ref/hash和全量node/edge coverage 100%；
7. expand成功后`NodeDetailExpanded`持久化，frontier继续前移也不再折叠；
8. expand full/root/active/unknown/duplicate节点整批失败且零提交；
9. initialize/create schema拒绝detail state或预先展开字段；
10. checkpoint/delta/resume/fork/20-cycle保持展开事件和detail hash；
11. 已展开详情使projection超预算时显式失败，不静默refold；
12. synthetic Map增长到骨架超预算时仍显式`map_skeleton_over_budget`，S4不得声称已根治。

### 10.2 Live samples

每个arm至少3次，可并行执行并轮换arm顺序：

| Sample | Arms | Purpose |
|---|---|---|
| simple live | Standard / B0 / S4.0 / S4.1 | 不形成distance>=3节点，验证零激活和普通任务零回归 |
| complex live | Standard / B0 / S4.0 / S4.1 | 自然形成远端历史，至少一次真实fold；任务应自然需要回看旧证据，但prompt不提fold/expand |

复杂样本不能告诉Agent“请展开某节点”，不能把答案写入节点名、goal或validator，也不能因某次Agent恰好展开而筛选
prefix。若3次中Agent从不选择展开，只能说明工具采用未发生；不得增加提示词诱导。可以另用deterministic fixture证明能力，
但不能宣称live Agent收益已验证。

### 10.3 统计口径

报告必须包含每arm的总和、均值、中位数：correctness、provider requests、input/cached/uncached/output tokens、wall、
ordinary/control tools、失败工具、projection bytes、fold count、expand count、node/edge count、folded/expanded node count。
缓存命中率同时报告加权总计、算术均值和中位数。

门禁沿用K章程：simple中S4.1/S4.0的requests、input、wall中位数均不高于1.10，Req2+ cache下降不超过2pp；
complex必须在预登记primary metric上优于S4.0，且correctness、全局骨架、语义hash和动作质量无未解释回退。

## 11. 日志合同

| Change Link | Success Signal | Failure Signal | Required Fields |
|---|---|---|---|
| fold eligibility | `taskspace.node_fold_evaluated` | distance/topology unavailable | task/map/node/frontiers/distance/status/root/expanded_before/reason |
| fold projection | `taskspace.node_detail_folded` | `taskspace.node_detail_fold_failed` | epoch/node/detail_ref/hash/bytes_before/after |
| expand request | `taskspace.node_expand_requested` | schema/precondition failure | call/map/node_ids/current_detail_states |
| expansion commit | `taskspace.node_detail_expansion_recorded` | `taskspace.node_expand_commit_failed` | event/node/call/state_commit |
| expanded projection | `taskspace.node_detail_expanded` | hash/ref mismatch | epoch/node/ref/expected_hash/actual_hash |
| replay | `taskspace.node_detail_expansion_replayed` | replay mismatch | checkpoint/delta/node/event/hash |
| experiment | `taskspace.map_strategy_evaluated` | evaluation failure | strategy/arm/sample/repeat/build/image/profile hashes |

日志只记录机械字段、有限reason code和hash，不记录API key、无界正文或Runtime生成语义。

## 12. 风险与处理

| Risk | Impact | Handling |
|---|---|---|
| Agent展开过多节点导致详情持续增长 | High | 尊重Agent动作记录；显式over-budget，不静默refold；通过live成本观察而非Runtime纠正 |
| 图缺边导致老节点不折叠 | Medium | 距离不可证明即不折叠，并记录reason；Map质量问题单独解决 |
| 把folded混入NodeStatus | High | 独立`detail_state`派生字段，状态机枚举不变 |
| Runtime根据失败/代码类型决定永久保留 | High | selector只接受状态、拓扑、距离和canonical展开事件 |
| Agent在创建时预先展开全部节点 | High | schema不暴露预展开字段，typed parser拒绝未知字段 |
| expand部分成功导致状态歧义 | High | 全量预检、clone提交、整批原子失败 |
| S1代码作为fallback长期残留 | High | R0先物理删除，禁止feature flag/compat/dormant path |
| tool schema增大所有active请求 | Medium | 分账schema bytes；只新增一个窄variant；simple成本门独立阻断 |
| 把S4误报为上下文超限根治方案 | High | 分账skeleton/detail bytes；骨架超限fixture必须继续显式失败；未来策略单独立项 |

## 13. 明确不做

- 不复用S1/S2/S3编号或修改其历史结论；
- 不把节点替换成archive、macro、checkpoint summary；
- 不删除或分页全局节点骨架；
- 不增加node importance或类似语义属性；
- 不允许Agent创建节点时声明已展开；
- 不提供折叠、撤销展开、重新折叠动作；
- 不在系统提示词中增加“应该展开什么”的策略指导；
- 不因样本表现修改阈值、距离定义或选择规则；
- 不在S4中设计或实现骨架超限策略；
- 不做旧实验数据兼容。

## 14. 完成定义

S4只有在以下条件全部满足时才可accept：

1. S1/S2/S3 production path为0，历史证据仍可审计；
2. 全部canonical nodes/edges在每个projection中100%可见；
3. 折叠集合与合同完全一致，未知距离零误折叠；
4. Agent只能通过expand为folded节点追加不可逆`NodeDetailExpanded`事件；
5. 展开、resume、fork、replay的详情hash和展开事件100%一致；
6. Runtime语义摘要、展开动机推断、自动refold均为0；
7. simple和complex live门禁通过，成本、缓存和动作变化可解释；
8. 报告明确S4只减少详情成本，骨架超限仍未解决；
9. S4.1结果单独汇报并暂停，等待用户决定是否进入K4。
