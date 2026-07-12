# R5-K Map-native 上下文压缩专项立项

- Created: 2026-07-12
- Updated: 2026-07-12
- Version: 1.0
- Status: Planned / Discovery Required
- Owner / Responsible: WhaleCode core runtime / TaskSpace Map
- Related Systems: canonical Event Store、Map projection、compaction、checkpoint、resume/replay、artifact refs
- Related Links: `22-r5-j6-7-canonical-task-context-plan.md`、
  `30-r5-j6-7-phase7-context-residue-plan.md`
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

1. 在硬上下文预算内持续保留root任务、当前active frontier和全局可导航路径；
2. 将满足硬拓扑条件的历史已闭合子图可逆地折叠为macro/archive node；
3. 压缩后的语义内容只能来自原始事件或Agent明确写入的checkpoint/conclusion；
4. 任意折叠子图均可按稳定ref展开、校验和replay；
5. 多轮压缩、resume、fork和继续工作不产生双事实源或Map坍缩。

## 2. 非目标

R5-K不做：

- 让Runtime根据正文“相关性”“价值”或任务理解挑选节点；
- 用LLM在Runtime后台自动总结、改写或合并Agent语义；
- 通过减少节点、限制Agent建图或提示Agent粗化任务规避规模问题；
- 只保留当前局部并把全局Map分页隐藏；
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
- archive/macro ref是否完整、可读、hash匹配且可replay；
- Agent是否显式提交了可用于压缩的node result/checkpoint。

### 4.2 Runtime不得决定的内容

Runtime不得：

- 判断哪段代码、结论或证据“更重要”；
- 从tool output或reasoning中抽取、改写或补写结论；
- 因正文相似自动合并node；
- 猜测旧失败、读取或决策已经失效；
- 为了满足预算伪造completed、忽略依赖或隐藏active node；
- 把压缩结果包装成Agent说过但实际未说过的内容。

## 5. 候选目标模型

以下仅是K1待验证候选，不是已冻结schema：

```text
TaskSpaceMap
  root
  active_frontier[]
  live_nodes[]
  archive_nodes[]

ArchiveNode
  id
  covered_node_ids[] | covered_range_ref
  entry_edge_ids[]
  exit_edge_ids[]
  terminal_status
  agent_checkpoint_event_id?
  result_refs[]
  archive_ref
  content_hash
  child_count
  created_at_sequence
```

压缩后projection中的macro node不是语义摘要，而是可验证的拓扑索引：它表达“这组已闭合node在这里，入口、
出口、状态和精确archive ref是什么”。只有Agent已明确写入checkpoint/conclusion时，macro node才可以展示该
语义正文；否则只显示机械字段和result refs。

## 6. 不变量

任何候选方案必须同时满足：

1. root原始任务和当前有效用户约束完整保留；
2. active、blocked、failed、in-flight及其依赖闭包不得归档；
3. 每个被折叠node都恰好属于一个可解析archive ref，不重叠、不丢失；
4. macro node完整保留子图的入口边、出口边、terminal status和child count；
5. 展开后node/edge/event/result hash与压缩前100%一致；
6. provider projection中的全局路径在macro粒度仍然连通；
7. Runtime不新增自然语言summary；Agent语义正文必须有source event ID；
8. compaction是派生视图变换，canonical Event Store仍是唯一事实源；
9. 同一checkpoint输入重复压缩得到稳定等价结果；
10. 超预算、缺ref、hash mismatch或replay失败必须显式停止，不能返回partial map。

## 7. 分阶段计划

### R5-K0：长会话规模与预算基线

- Entry：J6.7.7-G和J7完成；Docker benchmark substrate可用。
- Tasks：
  1. 构造100、1,000、10,000 node及不同edge density的synthetic Map；
  2. 建立真实长会话replay fixture，覆盖多次resume/compaction和代码变更；
  3. 分账root、node skeleton、edges、frontier、result refs和node-local details的bytes/tokens；
  4. 测量骨架首次超限点、增长斜率、projection构造耗时和store/replay成本；
  5. 只增加observer，不改变production投影。
- Exit：规模曲线、hard budget profiles和至少两个真实/合成长任务fixture齐全；未知owner=0。
- Fallback：observer revert；不得凭估算进入实现。

### R5-K1：压缩合同与方案选择

- Entry：K0通过。
- Tasks：
  1. 比较closed connected subgraph archive、hierarchical submap和Agent checkpoint boundary；
  2. 冻结eligible subgraph的纯机械判定；
  3. 冻结macro node、archive ref、expand和replay合同；
  4. 证明active frontier与全局连通性不受影响；
  5. 为每个方案执行失败场景审查，不通过则不进入K2。
- Exit：单一方案被选定；schema、ownership、权限、失败语义和回退均无unknown。
- Fallback：保持J6.7.7显式`map_skeleton_over_budget`，不加入临时分页。

### R5-K2：可逆Schema、Tools与日志

- Entry：K1通过。
- Tasks：
  1. 实现archive/macro最小schema和canonical refs；
  2. 为Agent提供显式inspect/expand入口，不把展开动作包装成Runtime建议；
  3. Agent可显式写checkpoint/conclusion，Runtime只校验source ownership和引用完整性；
  4. 增加eligible/archived/expanded/replayed/failed全链日志；
  5. 不保留旧Map压缩格式兼容路径。
- Exit：schema round-trip、权限、hash、ref和failure matrix 100%通过。
- Fallback：整phase revert并丢弃实验Map。

### R5-K3：Map压缩引擎

- Entry：K2通过。
- Tasks：
  1. 在hard budget阈值触发机械candidate选择；
  2. 只归档closed且不属于active依赖闭包的子图；
  3. 原子写archive + macro projection，失败时保持原Map；
  4. 支持按ref展开并恢复完整局部图；
  5. 保持canonical event sequence和result refs不变。
- Exit：100/1,000/10,000 node下不变量全部通过；任何故障均零partial state。
- Fallback：整phase revert，不做双格式读取。

### R5-K4：多轮压缩与恢复

- Entry：K3通过。
- Tasks：
  1. 连续执行至少20轮append -> compress -> resume；
  2. 覆盖fork、rollback、crash recovery、archive嵌套和旧代码读取版本；
  3. 校验重复压缩幂等、展开hash、全局连通和active frontier；
  4. 验证Agent主动读取历史子图后可继续工作，不要求Runtime解释其内容。
- Exit：state/event/result hash 100%；orphan/overlap/partial archive=0；20轮无漂移。
- Fallback：回退K3/K4，保留显式超预算错误。

### R5-K5：收益门禁与对抗性审查

- Entry：K0-K4全部通过。
- Tasks：
  1. Docker执行短、中、长三档Standard/R5对照；
  2. 报告correctness、requests、input/cache、wall、projection bytes和压缩频率；
  3. 检查root/frontier、全局路径、失败和Agent结论的保留率；
  4. 经用户授权执行对抗性审查，关闭critical/high findings。
- Exit：正确性无回退；root/frontier/protected保留100%；展开恢复100%；长任务保持在hard budget内；
  无Runtime semantic summary；无critical/high finding。
- Fallback：不声明收益并回退production压缩，保留K0 observer。

## 8. Phase Gate矩阵

| Phase | Independent verification | Exit evidence | Completion required | Decision |
|---|---|---|---|---|
| K0 | synthetic + real replay observer | scale/budget curve | 100% | proceed/pause |
| K1 | contract and failure review | zero unknown ownership | 100% | select/pause |
| K2 | schema/ref/permission fixtures | round-trip 100% | 100% | proceed/revert |
| K3 | 100/1k/10k engine tests | invariants 100% | 100% | proceed/revert |
| K4 | 20-cycle resume/fork/replay | zero drift/orphan | 100% | proceed/revert |
| K5 | Docker paired + authorized review | all benefit gates | 100% | close/revert |

## 9. 日志合同

| Change link | Success event | Failure event | Correlation fields |
|---|---|---|---|
| budget measurement | `taskspace.map_budget_measured` | `taskspace.map_skeleton_over_budget` | task/map/epoch/bytes/tokens/nodes/edges |
| candidate selection | `taskspace.map_archive_candidate` | `taskspace.map_archive_ineligible` | map/subgraph/nodes/reason code |
| archive commit | `taskspace.map_archive_committed` | `taskspace.map_archive_failed` | archive/macro/event range/hash |
| expansion | `taskspace.map_archive_expanded` | `taskspace.map_archive_expand_failed` | archive/ref/hash/request |
| replay | `taskspace.map_archive_replayed` | `taskspace.map_archive_replay_mismatch` | checkpoint/archive/expected/actual hash |

日志只记录机械ID、hash、count、budget和reason code，不记录API key、无界正文或Runtime生成摘要。

## 10. 关键风险

| Risk | Impact | Required mitigation |
|---|---|---|
| macro node掩盖关键依赖 | High | active dependency closure不可归档；entry/exit edges 100% |
| Agent结论缺失时Runtime补写摘要 | High | schema禁止Runtime自然语言summary；只允许source event ref |
| 旧代码读取被当作当前事实 | High | 保存content identity/revision；展开显示读取时版本 |
| 多轮压缩形成archive套archive漂移 | High | stable covered set/hash；K4 20轮幂等门禁 |
| 预算触发后留下partial Map | High | archive原子提交；失败保持原Map并显式报错 |
| 为压缩诱导Agent减少建图 | High | benchmark检查node granularity；prompt/schema不增加粗化建议 |

## 11. 开放问题

以下问题必须由K0/K1证据回答，当前不预设：

1. eligible subgraph最小规模和closed-age阈值是多少；
2. macro node是否允许分层嵌套，还是每次从canonical events重建扁平archive；
3. Agent checkpoint是压缩前强制动作、可选增强，还是只在自然产生时使用；
4. expand是provider下一轮完整展开，还是读取到普通tool result后由Agent自行使用；
5. 不同DeepSeek profile的skeleton reserve和active frontier reserve如何配置；
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
| 2026-07-12 | 压缩以可逆历史子图为候选 | 保持全局导航并允许精确展开，而不是隐藏未知节点 |
| 2026-07-12 | 语义载荷只接受原始事件或Agent checkpoint | Runtime只管理硬规则和Map，不替Agent解释工作 |

## 14. Plan Quality Checklist

- [x] 目标、非目标和Runtime边界明确。
- [x] K0/K1先发现再冻结实现，不把假设写成结论。
- [x] 全局导航、root、active frontier和可逆性有硬门禁。
- [x] production、schema、tool、日志、测试、Docker和审查路径完整。
- [x] 不做兼容、双写、静默分页、语义summary或Runtime relevance判断。
- [x] 每阶段可独立验证，未达到100%默认暂停或回退。
