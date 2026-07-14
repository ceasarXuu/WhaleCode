# R5-K Map-native 上下文压缩专项立项

- Created: 2026-07-12
- Updated: 2026-07-14
- Version: 1.6
- Status: K0/K1/K2 COMPLETE / S1-S3 ABANDONED / S4 DESIGNED / PAUSED
- Owner / Responsible: WhaleCode core runtime / TaskSpace Map
- Related Systems: canonical Event Store、Map projection、compaction、checkpoint、resume/replay、artifact refs
- Related Links: `22-r5-j6-7-canonical-task-context-plan.md`、
  `30-r5-j6-7-phase7-context-residue-plan.md`、
  `32-r5-j6-7-phase7-result.md`、
  `43-r5-k0-map-budget-baseline-result.md`、
  `48-r5-k3-s1-natural-prefix-result.md`、
  `49-r5-k3-s4-distance-fold-design.md`
- Risk Level: High
- Plan Type: Full charter；K0/K1通过前不冻结实现方案

## 1. 立项目标

J6.7.7-E负责在普通规模Map中保持完整全局骨架，并按机械规则调整node-local详情。它不解决一个更长期、
更困难的问题：当root、全部nodes/goals/edges和active frontier组成的最小骨架本身接近或超过provider上下文
预算时，完整投影已无法直接装入单个epoch。

现有单sample未达到该规模，但真实日常coding session会跨越大量任务、压缩和恢复周期，骨架超限属于
高概率生命周期问题。R5-K单独建立Map-native压缩合同，不以普通history文本压缩、静默分页或Runtime语义
摘要代替。

目标不是“尽量少展示Map”，而是：

1. 在最小Map骨架仍可容纳的范围内，持续保留root任务、全部nodes/edges、当前active frontier和全局可导航路径；
2. 只折叠远离active frontier的历史completed节点的node-local详情，不归档或替换节点；
3. 折叠内容只能来自原始事件和稳定ref，Runtime不生成摘要或解释Agent展开动机；
4. Agent可把已折叠节点单向展开，canonical展开事件使其此后不再折叠；
5. 展开、resume、fork和replay不产生双事实源、自动refold或Map坍缩。

S4只优化node-local details，不压缩root、node skeleton、edges或active frontier，因此不能根治Map无限增长导致的
上下文超限。当最小骨架本身超过预算时，S4继续显式返回`map_skeleton_over_budget`。骨架级压缩需要未来独立策略，
本阶段不展开设计，也不把它计入S4完成条件。

## 2. 非目标

R5-K不做：

- 让Runtime根据正文“相关性”“价值”或任务理解挑选节点；
- 用LLM在Runtime后台自动总结、改写或合并Agent语义；
- 通过减少节点、限制Agent建图或提示Agent粗化任务规避规模问题；
- 只保留当前局部并把全局Map分页隐藏，或用macro/archive替换canonical nodes；
- 删除canonical events或不可逆清理历史；
- 为旧实验数据增加兼容adapter、双写、feature flag或silent fallback；
- 将普通context compaction直接套在Map JSON/text上并宣称问题已解决。

## 3. 外部依据与适用边界

1. [LangGraph persistence](https://langchain-ai.github.io/langgraph/concepts/time-travel/)区分checkpoint与
   跨线程store，并指出无界checkpoint历史需要生命周期管理。R5-K复用“状态可重放、checkpoint是派生物”原则，
   不照搬其数据模型。
2. [LangGraph memory management](https://langchain-ai.github.io/langgraph/how-tos/memory/manage-conversation-history/)
   展示trim、delete、summary和subgraph persistence等长期上下文手段。R5-K只采纳分层持久化与可恢复性，
   拒绝Runtime自动语义summary。
3. [Anthropic long-running agent harness](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
   强调跨context window依靠持久artifact和明确handoff继续工作。R5-K将Agent-authored checkpoint视为候选
   压缩载荷，而不是Runtime生成交接文案。
4. [Anthropic context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
   把context视为有限资源，并讨论compaction与结构化notes。R5-K只采用预算约束和Agent可控记录，不授予
   Runtime语义编辑权。
5. [Microsoft GraphRAG global/local search](https://www.microsoft.com/en-us/research/blog/graphrag-improving-global-search-via-dynamic-community-selection/)
   和[DRIFT](https://www.microsoft.com/en-us/research/blog/introducing-drift-search-combining-global-and-local-search-methods-to-improve-quality-and-efficiency/?lang=ja)
   说明图结构中全局导航与局部细节可分层处理。其LLM relevance/community ranking不进入Runtime；R5-K只借鉴
   “全局骨架 + 局部展开”的结构区分。

## 4. 核心边界

### 4.1 Runtime可以决定的内容

Runtime只根据机械状态执行：

- 当前projection/profile的实测byte/token hard budget；
- node status是否closed，是否位于active frontier；
- 图的连通、依赖、祖先/后继和最短距离；
- event sequence、时间戳、hash、ref、size和checkpoint覆盖范围；
- node-local detail ref是否完整、可读、hash匹配且可replay；
- Agent是否通过合法expand动作写入了canonical `NodeDetailExpanded`事件。

### 4.2 Runtime不得决定的内容

Runtime不得：

- 判断哪段代码、结论或证据“更重要”；
- 从tool output或reasoning中抽取、改写或补写结论；
- 因正文相似自动合并node；
- 猜测旧失败、读取或决策已经失效；
- 为了满足预算伪造completed、忽略依赖或隐藏active node；
- 把压缩结果包装成Agent说过但实际未说过的内容。

## 5. 冻结目标模型

S1/S2/S3的archive/macro候选已经废弃，S4冻结为唯一待实施方向：

```text
TaskSpaceMap
  root
  active_frontier[]
  nodes[]                 # 始终完整可见
    status                # canonical工作流状态
    detail_state          # full | folded | expanded，projection派生
    detail_ref?
    detail_sha256?
  edges[]                 # 始终完整可见
  node_detail_expanded_events[]
```

`NodeDetailExpanded`由Agent通过`taskspace_control.expand_nodes`触发并持久化；`detail_state`由Runtime只按状态、
root、active frontier、图距离和展开事件机械派生。S4不增加importance属性，也不得产生`archive_nodes`或macro node。完整合同见
`49-r5-k3-s4-distance-fold-design.md`和
`benchmarks/taskspace/map-compression/s4-distance-fold-contract.json`。

## 6. 不变量

S4及后续任何修订必须同时满足：

1. root原始任务和当前有效用户约束完整保留；
2. 全部canonical nodes和edges始终保留，active、blocked、in-flight和graph roots的详情不得折叠；
3. 只有`completed && non-root && min_frontier_distance>=3 && no_expand_event`的节点可折叠；
4. 距离未知、不可达、无边或无active frontier时不得猜测折叠；
5. folded节点必须明确标记并保留目标、状态、拓扑、result refs、detail ref和hash；
6. Agent只能为folded节点单向写入`NodeDetailExpanded`，不能在创建时声明已展开、撤销事件或主动refold；
7. 展开后node/edge/event/result和node-local detail hash与折叠前100%一致；
8. Runtime不新增自然语言summary或展开动机推断；Agent语义正文必须有source event ID；
9. fold是派生视图，canonical Event Store仍是唯一事实源；Agent展开事件是canonical可replay事实；
10. 超预算、缺ref、hash mismatch或replay失败必须显式停止，不能返回partial map或静默折叠已展开节点；
11. S4不减少最小Map骨架，骨架超预算仍显式失败并留给未来独立策略。

### 6.1 单策略实验纪律

R5-K不得把多个压缩策略一次性实现后再用最终结果反推收益。冻结当前版本为不可变基线：

```text
Baseline ID: R5-K-B0
Source commit: 37bddb2bad9f8f92d52b082eb55c0c1a4171654a
Production behavior: 无archive/macro或detail fold；骨架超预算时显式失败
Evidence: 43-r5-k0-map-budget-baseline-result.md
```

K1必须生成包含完整commit、locked binary SHA、Docker image digest、harness/profile hash和sample prompt hash的
baseline manifest。后续不得移动`B0`；代码继续演进时仍从该immutable artifact重跑基线，不用历史均值代替同期运行。

每个策略`S<n>`只能包含一个预先声明的产品行为合同。schema、tool surface、触发条件、候选选择和详情保留中，
只要可独立启用且改变provider可见内容或Runtime行为，就分别视为策略，不得捆绑。纯内部schema/ref/logging基建
可以先落地，但必须证明production行为与B0一致。S4的自动fold与Agent expand属于同一可逆状态转换合同，任一侧
单独启用都会形成不可恢复折叠或无效工具，因此必须原子启用，不能拆成两个不完整provider行为。

每轮固定四个观察臂：

| Arm | Definition | Purpose |
|---|---|---|
| `STD<n>` | 当前candidate build的Standard模式 | 同期自然上下文参照 |
| `B0` | 固定基线commit的TaskSpace模式 | 观察相对初始版本的累计变化 |
| `P<n-1>` | 上一个已接受策略版本 | 隔离本轮唯一策略的边际变化 |
| `C<n>` | `P<n-1>`加且只加`S<n>` | 当前候选 |

首轮`P0=B0`。互斥策略不能累计：每个候选都必须分别从B0构建和运行，再选择其中一个。增量策略可以建立在上一
已接受版本上，但必须同时报告`C<n> vs P<n-1>`、`C<n> vs B0`和`C<n> vs STD<n>`，禁止只报告最终累计结果。

策略代码和测试按单主题commit提交；一个candidate build中不得有两个策略commit。被拒策略使用可审计的revert
恢复到`P<n-1>`，不留下长期feature flag、compat adapter、双写或dormant实现。接受策略后先冻结其commit、binary
SHA和image digest，才能开始下一策略。

### 6.2 每策略样本矩阵

每个`S<n>`至少执行一个简单sample和一个能实际触发该策略的复杂sample；synthetic 100/1k/10k只验证结构和
规模，不能替代Agent live sample。

| Sample class | Required behavior | Required arms | Repeats |
|---|---|---|---:|
| simple live | 不达到压缩阈值，验证普通任务零回归 | STD/B0/P/C | 3；方向混合时增至5 |
| complex live | 达到本策略触发条件，验证真实Agent工作流 | STD/B0/P/C | 3；方向混合时增至5 |
| deterministic scale/replay | 100/1k/10k、corruption、expand hash | B0/P/C；Standard不适用项标N/A | 每fixture至少1 |

K1冻结具体sample、prompt/task hash和等价历史构造。复杂sample必须让Standard看到等价的自然顺序历史，让
TaskSpace看到由同一canonical事件集组织出的Map；需要生成equivalence receipt，禁止通过给TaskSpace额外任务提示
或给Standard删除约束制造差异。各arm使用相同model/profile、Docker base digest、资源、网络、validator和oracle。
可并行运行独立repeat，但必须轮换arm顺序，避免provider cache预热顺序固定偏向某一版本。

### 6.3 单策略门禁

每个策略在进入下一策略前独立关闭，后续策略不能补证前一策略：

| Dimension | Simple gate | Complex gate |
|---|---|---|
| correctness | Agent complete且public/hidden validator全部通过 | Agent complete且public/hidden validator全部通过 |
| activation | `strategy_activation_count=0` | candidate每次运行`strategy_activation_count>0` |
| semantic fidelity | root/user constraints、tool failure/ref无丢失或重写 | root/frontier/protected保留100%，expand/replay hash 100% |
| deterministic cost | projection/tool schema差异必须完全等于strategy manifest | folded detail bytes、tool schema bytes和触发次数可分账 |
| stochastic guardrail | C/P的requests、input、wall三项median ratio均不高于1.10；Req2+ cache下降不超过2pp | 预先登记的primary benefit相对P改善；其他成本和正确性无未解释回退 |
| topology | canonical nodes/edges无变化 | fold前后全局topology可见率100%，expand detail hash 100% |

三次运行若方向混合，或简单sample任一median ratio位于`1.10~1.20`，扩展到五次；五次后仍超过1.10则策略暂停。
任何correctness失败、简单sample意外激活、provider-visible未登记差异、partial expand或hash mismatch都直接阻止
promotion，不得用复杂sample收益抵消简单sample回归。复杂sample的primary benefit及最小改善阈值必须在运行前写入
strategy manifest，运行后不得改指标追求通过。

每个`S<n>`完成后单独产出报告并暂停，明确`accept/reject/revise`。只有accept可以成为新的`P<n>`；revise仍视为
同一策略的新candidate，不得夹带下一策略。

## 7. 分阶段计划

### R5-K0：长会话规模与预算基线

- Entry：J6.7.7-G和J7完成；Docker benchmark substrate可用。
- Tasks：
  1. 构造100、1,000、10,000 node及不同edge density的synthetic Map；
  2. 建立真实长会话replay fixture，覆盖多次resume/compaction和代码变更；
  3. 分账root、node skeleton、edges、frontier、result refs和node-local details的bytes/tokens；
  4. 测量骨架首次超限点、增长斜率、projection构造耗时和store/replay成本；
  5. 只增加observer，不改变production投影。
  6. 分账长期delta replay链、生命周期checkpoint和canonical runtime events；J6.7.7短样本中internal
     replay仍占rollout约60%，该指标不得与provider上下文成本混为一谈。
  7. 冻结checkpoint/delta/archive corruption的session-fatal合同：比较panic、结构化session fatal error和可恢复
     operator error；任何方案都不得silent fallback或恢复partial Map。
- Exit：规模曲线、hard budget profiles和至少两个真实/合成长任务fixture齐全；未知owner=0。
- Fallback：observer revert；不得凭估算进入实现。

实施结果（2026-07-13）：K0验收`7/7`、完成度100%，允许进入K1。100/1,000/10,000 nodes与
`none/chain/forward_4`三种edge profile共9个规模点、15个budget crossing、3档checkpoint/delta replay、
1,000-node 5轮session-native resume/compaction/code revision及真实Docker rollout 3/3 replay均已落盘。
unknown owner=0。完整测量、artifact和未完成项见`43-r5-k0-map-budget-baseline-result.md`。

K0实施中修复了Map runtime rollout内外层重复`type`导致checkpoint/delta不可反序列化的问题。新协议不保留
旧数据兼容；真实rollout的2个checkpoint、87个delta可由生产loader完整读取并稳定重放。该修复恢复的是既有
replay合同，不改变production projection或引入压缩策略。

### R5-K1：压缩合同与方案选择

> 历史phase，已完成。其archive候选选择后来被S1 live证据和S4产品边界推翻，不再作为当前目标设计。

- Entry：K0通过。
- Tasks：
  1. 物化并冻结`R5-K-B0` baseline manifest和可重跑Docker artifact；
  2. 比较closed connected subgraph archive、hierarchical submap和Agent checkpoint boundary；
  3. 冻结所有候选共享的最小archive/ref/expand/replay不变量，不冻结多策略组合实现；
  4. 将候选拆成atomic strategy ledger，标明互斥或增量、唯一行为delta、primary metric和forbidden co-change；
  5. 冻结简单/复杂sample、等价历史receipt、四arm矩阵、重复次数和strategy report schema；
  6. 只选择下一项`S1`进入实施，不预先批准后续策略叠加；
  7. 为B0、公共合同和S1执行失败场景审查，不通过则不进入K2。
- Exit：B0可重跑；公共schema、ownership、权限、失败语义无unknown；strategy ledger可拆分；S1和验收阈值唯一。
- Fallback：保持J6.7.7显式`map_skeleton_over_budget`，不加入临时分页。

### R5-K2.0：无行为公共基建

> 历史phase，已完成。仅保留仍被S4使用的中性ref/hash/runner能力；S1专用archive接线由K3-R0删除。

- Entry：K1通过。
- Tasks：
  1. 实现不改变production projection的archive/ref内部schema和hash codec；
  2. 建立strategy manifest、四arm runner、equivalence receipt和逐策略报告器；
  3. 增加strategy evaluated/accepted/rejected和replay mismatch日志；
  4. provider-visible tool/schema或自动触发不得在本phase落地，它们必须进入独立`S<n>`；
  5. 不保留旧Map压缩格式兼容路径。
- Exit：schema round-trip、hash/ref/failure matrix 100%；简单/复杂B0与K2.0 build行为等价；策略激活为0。
- Fallback：整phase revert并丢弃实验Map。

### R5-K2.F：Corruption fatal单变更

- Entry：K2.0通过。
- Tasks：
  1. 只把当前`panic_via_expect`转换为K0已选定的structured session fatal error；
  2. checkpoint/delta缺失、错序、hash mismatch和损坏archive均返回同一session-fatal类别；
  3. 保持partial restore和silent fallback为禁止，不顺带加入archive、tool或projection策略；
  4. 独立执行corruption matrix，并用simple/complex smoke证明正常路径相对K2.0无变化。
- Exit：corruption fixture 100%命中结构化fatal；partial state=0；正常路径strategy activation=0且行为等价。
- Fallback：revert K2.F单独commit回到K2.0；不得以兼容fallback替代结构化fatal。

### R5-K3-Sn：单策略垂直切片循环

- Entry：K2.0和K2.F分别通过。
- 当前执行顺序：
  1. `K3-R0`先物理删除S1 archive production path，验证回到P1/B0等价行为；
  2. `K3-S4.0`只落expand event/snapshot/distance/logging基建，provider-visible行为保持B0；
  3. `K3-S4.1`原子启用distance fold和Agent expand，不顺带修改prompt、compaction或普通tool反馈；
  4. 执行simple/complex四arm各3次和deterministic scale/replay；必要时扩至5次；
  5. 分别报告对Standard、B0和S4.0的正确性、动作、Map、projection、request/token/cache/wall差异；
  6. 单独判定accept/reject/revise并暂停；未accept不得进入K4。
- Exit：本策略6.3门禁全部通过；任何故障均零partial state；candidate identity和结果artifact齐全。
- Fallback：整体revert S4.1回到S4.0并验证binary/source identity；fold和expand不得只保留一侧，不保留禁用分支。

详细字段、算法、tool合同、phase gate和测试矩阵见`49-r5-k3-s4-distance-fold-design.md`。

### R5-K4：多轮压缩与恢复

- Entry：S4.1已通过K3门禁并被accept。
- Tasks：
  1. 连续执行至少20轮append -> finish -> fold -> expand -> resume；
  2. 覆盖多active frontier、fork、rollback、crash recovery和旧代码读取版本；
  3. 校验展开事件不可逆、fold集合确定、展开hash、全局连通和active frontier；
  4. 验证Agent主动展开历史节点后可继续工作，不要求Runtime解释其内容；
  5. 若暴露回归，使用B0/S4.0/S4.1 artifact定位，不增加修复策略掩盖原因。
- Exit：state/event/result hash 100%；node/edge可见率100%；自动refold/partial expand=0；20轮无漂移。
- Fallback：回退K3/K4，保留显式超预算错误。

### R5-K5：收益门禁与对抗性审查

- Entry：K0-K4全部通过。
- Tasks：
  1. Docker执行短、中、长三档Standard/B0/final R5对照，并保留各已接受`P<n>`的逐策略证据索引；
  2. 报告correctness、requests、input/cache、wall、projection bytes和压缩频率；
  3. 检查root/frontier、全局路径、失败和Agent结论的保留率；
  4. 经用户授权执行对抗性审查，关闭critical/high findings。
- Exit：正确性无回退；root/frontier/protected保留100%；展开恢复100%；在骨架仍可容纳的样本域内详情成本有明确收益；
  骨架超限继续显式失败；无Runtime semantic summary；无critical/high finding。
- Fallback：不声明收益并回退production压缩，保留K0 observer。

## 8. Phase Gate矩阵

| Phase | Independent verification | Exit evidence | Completion required | Decision |
|---|---|---|---|---|
| K0 | synthetic + real replay observer | scale/budget curve | 100% | complete；proceed to K1 |
| K1 | B0/contract/strategy ledger/failure review | 历史完成；S1结果已被后续证据拒绝 | 100% | complete/historical |
| K2.0 | schema/ref/runner parity fixtures | round-trip 100%；相对B0行为等价；activation=0 | 100% | proceed/revert |
| K2.F | corruption matrix + normal-path smoke | structured fatal 100%；partial=0；正常路径等价 | 100% | proceed/revert |
| K3-R0/S4 | P1 parity；STD/B0/S4.0/S4.1；simple+complex+scale | S1代码清零；S4正确性、边际收益与零简单回归 | 每个stage 100% | S4 designed；implementation paused |
| K4 | 20-cycle resume/fork/replay | zero drift/orphan | 100% | proceed/revert |
| K5 | Docker paired + authorized review | all benefit gates | 100% | close/revert |

## 9. 日志合同

| Change link | Success event | Failure event | Correlation fields |
|---|---|---|---|
| budget measurement | `taskspace.map_budget_measured` | `taskspace.map_skeleton_over_budget` | task/map/epoch/bytes/tokens/nodes/edges |
| fold eligibility | `taskspace.node_fold_evaluated` | distance/topology unavailable | map/node/frontiers/distance/status/root/expanded_before/reason |
| fold projection | `taskspace.node_detail_folded` | `taskspace.node_detail_fold_failed` | epoch/node/detail ref/hash/bytes |
| expansion | `taskspace.node_expand_requested` / `taskspace.node_detail_expansion_recorded` | `taskspace.node_expand_commit_failed` | call/map/node/event/state commit |
| replay | `taskspace.node_detail_expansion_replayed` | expansion event/detail hash mismatch | checkpoint/delta/node/expected/actual |
| strategy experiment | `taskspace.map_strategy_evaluated` | `taskspace.map_strategy_evaluation_failed` | strategy/arm/baseline/previous/candidate/sample/repeat |
| strategy decision | `taskspace.map_strategy_accepted` | `taskspace.map_strategy_rejected` | strategy/candidate commit/binary/image/report |
| arm equivalence | `taskspace.map_experiment_equivalence_verified` | `taskspace.map_experiment_equivalence_failed` | strategy/arm/prompt/history/image/profile hash |

日志只记录机械ID、hash、count、budget和reason code，不记录API key、无界正文或Runtime生成摘要。

## 10. 关键风险

| Risk | Impact | Required mitigation |
|---|---|---|
| folded详情掩盖Agent后来需要的证据 | High | 全局节点仍可见且明确标folded；Agent可单向展开，canonical事件阻止再次折叠 |
| Agent结论缺失时Runtime补写摘要 | High | schema禁止Runtime自然语言summary；只允许source event ref |
| 旧代码读取被当作当前事实 | High | 保存content identity/revision；展开显示读取时版本 |
| 多轮距离变化导致展开节点再次折叠 | High | canonical展开事件不可撤销，Runtime不得自动refold |
| 预算触发后静默折叠已展开节点 | High | 尊重Agent动作；显式over-budget，不返回partial Map |
| S4被误当成最终上下文上限方案 | High | 单独报告skeleton/detail bytes；骨架超限保持显式失败，未来策略另行立项 |
| 为压缩诱导Agent减少建图 | High | benchmark检查node granularity；prompt/schema不增加粗化建议 |
| 多策略叠加后无法归因 | Critical | 固定B0、Previous和Candidate四arm；每个build只含一个新策略 |
| 复杂样本收益掩盖简单任务回归 | High | 每策略强制simple live门禁；简单回归不能由复杂收益抵消 |
| 历史基线受provider时序和cache污染 | High | immutable B0同期重跑、arm顺序轮换、3次方向混合扩至5次 |

## 11. 开放问题

S4设计已关闭archive方向的开放问题。仍需由实施证据回答：

1. `distance>=3`在自然复杂任务中的fold频率和净收益是否稳定；
2. 一个窄`expand_nodes` variant对active tool schema和DeepSeek cache的实际成本是多少；
3. Agent在无额外prompt引导时是否会在真实需要下自然展开节点；
4. Agent展开事件长期累积导致projection over-budget的实际频率；
5. 不同DeepSeek profile的skeleton/detail reserve如何配置；
6. 长会话下缓存前缀与新epoch projection之间的最优边界在哪里。

## 12. 路线位置

R5-K不阻塞J6.7.7普通规模projection合同。总路线调整为：

```text
R5-J6.7.7 -> J6.7 final review -> R5-J7 -> R5-K -> R5-G3 -> R5-H
```

K0/K1属于专项发现和合同冻结；只有证据证明骨架规模、触发阈值和可逆方案后，才允许K2-K5进入代码。

## 13. Decision Log

| Date | Decision | Reason |
|---|---|---|
| 2026-07-12 | Map骨架超预算独立立项 | 当前sample不触发，但真实长会话高概率触发，不能继续没有专门合同 |
| 2026-07-12 | 普通projection不分页全局骨架 | projection的核心价值是持续掌握全局路径 |
| 2026-07-12 | 压缩以可逆历史子图为候选 | 历史候选；后由S1负收益和全局视野原则推翻 |
| 2026-07-12 | 语义载荷只接受原始事件或Agent checkpoint | Runtime只管理硬规则和Map，不替Agent解释工作 |
| 2026-07-13 | K0按7/7关闭并开放K1 | 9个规模点、15个阈值、两类长链fixture和真实rollout重放证据齐全，unknown owner=0 |
| 2026-07-13 | corruption目标选择structured session fatal | partial restore、silent fallback和operator recoverable均不符合canonical Map完整性 |
| 2026-07-13 | 压缩改为逐策略实验阶梯 | 固定B0并比较Previous/Candidate，避免多个策略叠加后无法拆解收益和回归 |
| 2026-07-13 | 每策略同时验收简单和复杂sample | 复杂任务压缩收益不能证明普通任务没有request、token或语义回归 |
| 2026-07-14 | S1判定为REVISE并暂停 | codec/scale通过，但live样本未稳定同时满足active projection epoch与3个eligible completed nodes；禁止用低token阈值或新增Runtime触发器制造通过 |
| 2026-07-14 | S1自然active-prefix复验后判定为REJECTED | 同一canonical snapshot上projection减少56.4%，但复杂样本requests/input/wall为P1的1.50x/1.68x/1.51x，简单样本成本门也未通过；停止且不进入S2 |
| 2026-07-14 | S1/S2/S3全部废弃，S4替代 | archive/macro会让节点退出全局视野；S4保留全部节点和边，只折叠distance>=3的completed非root节点详情 |
| 2026-07-14 | 删除node importance属性，以展开事件作为唯一持久机制 | `NodeDetailExpanded`只记录Agent动作事实，避免引入无明确机制的语义参数；事件不可撤销且可replay |
| 2026-07-14 | S4不承担最终骨架超限 | S4只减少node-local details；最小骨架最终仍可能超限，未来必须用独立策略处理 |

## 14. Plan Quality Checklist

- [x] 目标、非目标和Runtime边界明确。
- [x] K0/K1先发现再冻结实现，不把假设写成结论。
- [x] 全局导航、root、active frontier和可逆性有硬门禁。
- [x] production、schema、tool、日志、测试、Docker和审查路径完整。
- [x] 不做兼容、双写、静默分页、语义summary或Runtime relevance判断。
- [x] 每阶段可独立验证，未达到100%默认暂停或回退。
- [x] 每个candidate只新增一个策略，并固定比较Standard、B0、Previous和Candidate。
- [x] 每策略均包含simple和complex live sample，不用synthetic结果替代Agent行为。
- [x] S4保持全部node/edge可见，并把workflow status、projection detail state和canonical Agent展开事件分离。
- [x] S4收益边界明确，不把详情折叠误报为骨架超限根治方案。
