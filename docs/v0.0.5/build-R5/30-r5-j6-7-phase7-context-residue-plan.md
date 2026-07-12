# R5-J6.7.7 上下文残留去重与空 Map 收敛计划

- Created: 2026-07-12
- Updated: 2026-07-12
- Version: 1.2
- Status: A-F complete; G engineering/live complete after repair; adversarial review pending authorization
- Owner / Responsible: WhaleCode core runtime / TaskSpace context
- Related Systems: canonical Event Store、provider linearizer、`taskspace_control`、projection、
  session finalization、rollout/replay、benchmark observer
- Related Links: `22-r5-j6-7-canonical-task-context-plan.md`、
  `29-r5-j6-7-phase6-benefit-gate-result.md`、
  `32-r5-j6-7-phase7-result.md`、
  `31-r5-map-native-context-compression-charter.md`、
  `coe/2026-07-10-22-56-r5-request-amplification.md`
- Risk Level: High
- Plan Type: Full

## 1. 决策摘要

J6.7.6 证明旧 base history / TaskSpace projection 双轨已经删除，但进一步按字段 lineage 检查发现：

1. `finish_then_end.final_candidate` 与 canonical assistant final 正文完全相同；
2. bootstrap outer `actions[]` 与展开后的 native nested call 参数重复；
3. 空 Map developer message 把一次性 mode transition 和机械空 Map snapshot 合并，初始化后仍作为
   具有强显著性的旧 hard state 留在前缀；
4. success ack 重复 Agent 已提交的 `node_id/next_node_id`；
5. populated projection 尚未建立“全局骨架完整、局部详情分层”的稳定合同；
6. full `snapshot_updated` 占 focused/complex rollout 约95.2%/96.0%，虽不进入模型上下文，仍构成
   replay/log 结构重复。

因此新增 J6.7.7，在最终对抗性审查前关闭上述残留。J7 继续暂停。

目标结构：

```text
自然任务语义       -> user / assistant / native ordinary call-result canonical events
Map明确状态         -> TaskSpace Map字段和机械transition
TaskSpace运输envelope -> 当前响应执行所需；成功展开/覆盖后不再作为第二份provider语义
失败语义           -> 原始call/output完整保留，不折叠
projection          -> 仅resume/compaction等epoch边界的完整Map骨架与分层局部详情
snapshot            -> 可重建checkpoint，不是每个trace event的事实副本
```

## 2. 当前证据基线

有效 Docker artifacts：

- focused：`target/r5-j6-7-6-live/count-call-stack/20260712-124928-300`；
- complex：`target/r5-j6-7-6-live/subscription-billing-repair/20260712-124928-323`。

| Residue | Focused | Complex | Observer blind spot |
|---|---:|---:|---|
| TaskSpace-only initial developer message | 595 B × 11 requests | 595 B × 14 requests | 只计唯一projection，不判断旧hard state显著性 |
| terminal final exact body duplicate | 396 B × 2 | 1,884 B × 2 | terminal后无next request，payload scan未触发 |
| bootstrap outer args | 637 B | 745 B | 不做跨carrier字段lineage |
| expanded nested args | 146 B / 2 calls | 161 B / 1 call | 与outer `actions[]`语义相同但wrapper不同 |
| bootstrap outer output | 527 B | 329 B | refs之外仍重复tool/call/success字段 |
| full snapshots | 53 / 5.68 MB | 69 / 9.10 MB | 不进入provider cost report |
| snapshot / rollout bytes | 95.2% | 96.0% | 当前只统计event count |

`exact_payload_duplicates=0`仍然成立，但它只证明没有相同完整payload/record，不能证明不同字段和
不同carrier没有承载同一语义。

## 3. 唯一所有权与保留决策

| Information | Unique canonical owner | Provider保留 | 删除/折叠 | Reason |
|---|---|---|---|---|
| 用户任务 | root user event | 原文一次 | 不进入Map summary副本 | 保留自然role和原文 |
| node id/kind/goal/dependency | Map node字段 | epoch视图按需机械暴露 | 成功init envelope不再平行暴露完整node定义 | 这是允许结构化的Map状态 |
| ordinary tool arguments/output | native call/result events | 原生pair各一次 | outer `actions[]`和aggregate正文只作当轮运输 | 工具反馈是自然上下文事实 |
| init committed状态 | Map transition | task/map新生成ID和commit状态 | 不重复Agent提供的node/action参数 | Runtime只确认机械提交 |
| nonterminal finish intent | 原始control call | call保留一次 | 无 | 这是Agent明确状态动作 |
| nonterminal finish success | control output | 新`result_id`和最终binding/status | 删除回显的node/next字段 | node/next可由call推出 |
| control failure | 原始control call/output | 全量保留 | 不折叠、不摘要 | Agent必须看到失败并自行纠正 |
| final answer正文 | assistant final event | 自然assistant正文一次 | committed terminal call中的运输正文未来不可见 | final正文属于Agent回答，不属于Map |
| terminal transition | Map/result transition | 新result IDs与completed状态 | 不复制final正文 | 与回答语义分账 |
| fresh blank Map | Runtime Map状态 + bootstrap tool schema/choice | 不新增developer snapshot | 删除空Map projection和mode transition副本 | tool contract已明确必须初始化 |
| resumed/compacted Map | current Map projection | 完整root/nodes/goals/edges + 分层node详情 | 不分页或裁掉全局骨架；远端过程详情改为ref | 保留全局导航，以机械规则分配局部细节 |
| runtime trace | append-only trace event | 不进入provider | 不嵌入每个完整snapshot | observability不应制造事实副本 |
| replay snapshot | lifecycle checkpoint | 不进入provider | 非边界事件不写full snapshot | snapshot是派生索引 |

### 3.1 失败优先例外

只有成功且机械覆盖关系完整时允许折叠运输envelope。以下情况必须保留原始call/output：

- JSON/protocol解析失败；
- state-machine拒绝；
- nested ordinary action失败或被skip；
- call/result缺失、顺序不闭合或source event不存在；
- terminal commit成功但assistant final event未持久化；
- checkpoint/ref校验失败。

不得用silent fallback生成“看起来成功”的上下文。

## 4. 外部依据与约束

1. [MCP Tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：工具由模型
   控制，tool result可保留structured/unstructured原始内容；错误结果应进入模型上下文以便自行纠正。
2. [MCP schema](https://modelcontextprotocol.io/specification/2025-11-25/schema)：tool result必须用稳定
   tool use ID配对，`isError`和structured content属于结果合同。
3. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存依赖稳定前缀；本计划
   只在事件首次进入provider history前确定其canonical视图，不逐请求重写已发送前缀。
4. [Martin Fowler Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html)：状态由事件重建，
   snapshot是派生加速结构，不应成为与事件并列的第二事实源。

这些依据不授权Runtime评估任务事实的重要性或改写Agent结论。

### 4.1 依赖与假设

| Dependency | Type | Status | Blocking Risk | Handling |
|---|---|---|---|---|
| canonical Event Store/codec | system | Ready | owner关系无法落地 | 复用J6.7.1-.5，不增加第二store |
| provider历史tool pair接受度 | third-party | Ready/需probe | envelope折叠后请求被拒绝 | D在production改动前做pair fixture和live probe |
| output ref/artifact store | system | Ready | node详情降级后不可恢复 | E要求ref round-trip 100% |
| Docker benchmark/logging | environment | Ready | 收益不可证 | 每phase固定sample |
| R4 executable | environment | Unavailable | 无当前三边成本 | 标记historical/unavailable |
| 对抗性审查授权 | person | Pending | G不能关闭 | A-F完成后申请 |

| Assumption | Verification | If False |
|---|---|---|
| successful outer envelope可由Map transition + native pairs无损覆盖 | field lineage和provider pair fixture | pause D，不复制nested body作为fallback |
| fresh blank Map只靠bootstrap schema/choice足够明确 | B schema probe和live init成功率观察 | revert B并向用户报告，不新增策略提示 |
| terminal carrier可在commit后由assistant final机械覆盖 | next-turn/resume exact reconstruction | revert C，保留J6.7.6并重新设计carrier |
| checkpoint + events可完整重建snapshot | replay state hash | revert F，不保留dual snapshot |

### 4.2 替代方案与取舍

| Option | Decision | Reason |
|---|---|---|
| 用LLM summary合并重复 | Reject | 引入语义改写和Runtime判断 |
| 保留所有carrier，依赖cache | Reject | 缓存只降成本，不解决歧义和窗口占用 |
| 动态重写每次完整Map | Reject | 破坏append-only前缀并扩大Runtime显著性 |
| 成功transport机械coverage，失败raw保留 | Select | 按事件关系去重，无自然语言判断 |
| 为旧会话保留兼容linearizer | Reject | 实验产品无数据保留需求，会形成双轨 |

### 4.3 安全与权限边界

1. coverage只改变未来provider可见视图，不改变tool permission、sandbox、approval或执行结果。
2. failed call/output、termination、stderr、exit code和ref权限事实不得被coverage。
3. node-local evidence ref只读取当前会话有权访问的事件或artifact，不因ref存在扩大跨task/thread权限。
4. 日志只记录ID/hash/bytes/count，禁止记录secret和无界正文。

## 5. 目标 Provider 上下文

### 5.1 Fresh blank Map

```text
global Standard context
+ environment
+ user task
+ bootstrap taskspace_control schema/tool choice
```

不再追加 `TaskSpace mode is now active`、`active_task_path_without_nodes` 或空列表projection。bootstrap
tool description和唯一可用action已经机械说明Map需要初始化；Runtime继续以硬状态拒绝未初始化的普通动作。

### 5.2 Active Map

```text
global Standard context
+ root/user canonical events
+ Map transition/control events
+ native ordinary call/result events
+ Agent messages
```

成功bootstrap的outer运输envelope在展开后由Map transition和native pairs机械覆盖，不再第二次进入provider
视图。失败bootstrap保持原始pair。

### 5.3 Terminal与后续turn

```text
terminal transition metadata
+ one canonical assistant final
+ next user turn
```

final正文只出现一次。raw terminal carrier继续保存在audit/replay事件中，但成功覆盖后不进入未来provider
上下文。Map results引用assistant final event和机械transition refs，不引用重复正文。

### 5.4 Resume / Compaction

只在新epoch构造current Map projection。projection始终完整展示root任务详情、所有node的ID/kind/status/goal、
所有edge/dependency和current node，确保Agent无需翻页即可掌握全局路径。局部详情按图距离、事件新旧、
node状态和机械事件类型分层；被降级的只是node内过程与证据正文，必须保留稳定event/artifact ref。

#### 5.4.1 全局骨架合同

以下内容不得因普通projection预算被分页、截断或省略：

1. root原始用户任务、当前有效约束和用户明确追加要求；
2. task/map ID、状态和current node；
3. 所有node的ID、kind、Agent-authored goal和status；
4. 所有edge/dependency及未闭合frontier；
5. node最终result ID和Agent-authored conclusion ref（若存在）。

如果仅上述骨架已经超过hard context budget，J6.7.7-E必须记录`map_skeleton_over_budget`并暂停，不得静默
返回局部Map。该问题由独立R5-K专项解决，见`31-r5-map-native-context-compression-charter.md`。

#### 5.4.2 局部详情分层

分层只使用确定性结构事实，不使用LLM、embedding、关键词或Runtime生成的“相关性/重要性”判断：

| Tier | Mechanical scope | Provider detail |
|---|---|---|
| D0 | root | 原始任务、约束、用户追加要求和root结果完整保留 |
| D1 | current node、未闭合node、与current图距离1 | 最近事件、工具结果、失败、artifact/result ref和Agent结论保留最多详情 |
| D2 | 与current图距离2或最近完成的直接前驱 | 保留最终outcome、关键机械事实、Agent结论和raw ref，过程正文可降级 |
| D3 | 更远且已完成的历史node | 保留完整骨架、最终result/Agent结论和durable evidence ref；中间过程只保留ref |

同一层内按原始event sequence排序，不按Runtime推测的任务价值重排。图距离相同的详情预算由稳定
sequence和固定byte/item规则分配，保证同一Map输入得到相同projection。

#### 5.4.3 证据效用分类

这里的“效用”是事件类型合同，不是正文语义判断：

| Class | Examples | Retention rule |
|---|---|---|
| P0 protected | 用户要求、失败/hard-state error、node result、final、Agent-authored conclusion | 不得整体省略：Agent正文逐字保留；超长工具失败保留机械结果、透明excerpt/truncation和raw ref |
| P1 durable outcome | patch/edit结果、测试/validator结果、artifact ref、代码读取的path + content hash/revision + event ref | 长期保留机械结果与引用；正文过长时渐进读取 |
| P2 operational outcome | shell exit/termination、成功工具结果、结构化计数 | 结果优先于调用过程；保留原始结果ref |
| P3 transient process | 中间进度、重复list/probe、已被结果覆盖的执行过程 | 远端node只保留event ref；canonical store不删除 |

代码读取正文可能在后续修改后过时，因此长期层保存读取时的path、content identity/revision和raw event ref，
而不是把旧正文提升为当前事实；Agent明确写出的结论仍由Agent负责，可作为node result保留。Runtime不得从
工具正文自动抽取“结论”，也不得自行总结过程。

## 6. 分阶段执行

每个subphase仍按R5总规则报告Standard、R4、当前R5。当前R4可执行快照不可用时，R4列必须标记
`historical/unavailable`，不得用0或其他版本数据补位；行为改动阶段至少执行一次列出的Standard/R5 Docker
sample，observer-only阶段可重放冻结artifact并另跑一次focused live sample。

### J6.7.7-A：字段Lineage观测与Owner冻结

- Status：Complete。

- Entry：J6.7.6 artifacts有效。
- Tasks：
  1. observer增加`cross_carrier_lineage`，机械匹配final candidate、expanded nested args、success ack回显；
  2. 增加post-terminal下一turn重建检查；
  3. 输出TaskSpace-only fixed messages、stale hard-state markers、snapshot bytes ratio；
  4. owner矩阵转成测试fixture，不做自然语言相似度。
- Validation：复用两个J6.7.6 artifacts并执行`count-call-stack` Standard/R5 Docker 1次。
- Exit：已列类型coverage=100%，unknown owner=0，observer self-test通过。
- Fallback：回退observer提交，不改变production。
- Next Gate：100%后进入B，否则pause。

### J6.7.7-B：空 Map 与Mode上下文收敛

- Status：Complete。

- Entry：A通过。
- Tasks：
  1. fresh blank Map不生成developer projection；
  2. 删除单独mode transition message，稳定机械规则只由bootstrap tool schema/choice表达；
  3. 保留Map内部mechanical blank、owner和硬状态校验；
  4. protocol/state失败继续原样进入canonical event。
- Validation：bootstrap schema/unit/session tests；`count-call-stack` Standard/R5 Docker 1次。
- Exit：first request TaskSpace-only developer message=0；成功init后stale blank hard-state marker=0；
  pre-init ordinary success=0；正确性通过；request 2+ cache不低于Standard超过2pp。
- Fallback：整phase revert；不得恢复旧projection同时新增新提示。
- Next Gate：全部通过后进入C。

### J6.7.7-C：Terminal正文单一Owner

- Status：Complete。

- Entry：B通过。
- Tasks：
  1. assistant final event成为final正文唯一provider owner；
  2. committed terminal carrier记录`covered_by_final_event_ref`机械关联，raw call/output仅audit/replay；
  3. 未来turn linearizer不再暴露carrier内的final正文；
  4. Map result引用final event和terminal transition，不存body；
  5. failed terminal和missing-final保持原始pair并显式失败。
- Validation：unit/round-trip/resume/compaction；新增`terminal-followup-context-smoke` Docker R5 1次，
  `count-call-stack` Standard/R5 1次。
- Exit：post-terminal provider final正文exact occurrence=1；UI final、resume final和Map refs一致；失败恢复率100%。
- Fallback：整phase revert；不得保留双写兼容开关。
- Next Gate：全部通过后进入D。

### J6.7.7-D：Bootstrap Nested与Success Ack去重

- Status：Complete。

- Entry：C通过。
- Tasks：
  1. native nested call/result成为ordinary工具唯一provider owner；
  2. successful outer bootstrap envelope在展开后只留下Map transition，新视图不重复`actions[]`；
  3. init success只返回Runtime新生成的task/map ID和必要commit状态；
  4. finish success只返回新result IDs、最终binding/status，不回显node/next；
  5. nested失败、skip和protocol错误保留完整原始pair；
  6. provider历史始终满足call/result配对和稳定顺序。
- Validation：provider pairing fixtures、nested failure matrix、large output ref；`count-call-stack`和
  `large-output-ref-smoke` Standard/R5各1次。
- Exit：successful nested arguments/output provider occurrence各1；success ack inferable field count=0；
  failed feedback byte/hash恢复100%；orphan=0。
- Fallback：整phase revert；不得回到outer output复制全部nested body。
- Next Gate：全部通过后进入E。

### J6.7.7-E：Projection全局骨架与局部详情分层

- Status：Complete。fresh未压缩会话由canonical init/control自然历史保有全局Map；projection只在
  resume/compaction/new epoch构造一次，避免平行副本和DeepSeek prefix断裂。

- Entry：D通过。
- Tasks：
  1. root详情、所有nodes/goals/status、所有edges/dependencies和current frontier始终完整暴露；
  2. 按5.4定义的图距离、node状态、event sequence和机械事件类型分配node-local详情；
  3. 远端过程正文降级为稳定event/artifact ref，不删除canonical event，也不生成Runtime摘要；
  4. 失败、用户要求、node result和Agent-authored conclusion按P0保护；
  5. 代码读取保留path、content identity/revision和raw event ref，避免旧正文伪装成当前事实；
  6. projection只在resume/compaction/new epoch产生一次；骨架本身超预算时显式失败并转R5-K。
- Validation：1/10/100/1000 nodes边界fixture、全骨架覆盖、detail tier确定性、ref round-trip、
  protected failure和stale code-read identity；
  `subscription-billing-repair`与`multi-file-order-pipeline` Standard/R5各1次。
- Exit：root/nodes/goals/edges覆盖率100%；D1-D3分类确定性100%；降级详情可100%按ref恢复；
  protected miss=0；需要epoch重建时projection count=1，fresh自然历史epoch为0；semantic replacement=0；
  正确性不回退。1000-node fixture若
  骨架超预算，必须产生`map_skeleton_over_budget`而不是partial map，且不阻塞D1-D3合同验收。
- Fallback：回退本phase，不使用Map分页、LLM summary、语义相似度或Runtime优先级heuristic替代。
- Next Gate：全部通过后进入F。

### J6.7.7-F：Replay Snapshot增量化

- Status：Complete。delta相对前一状态链接；full checkpoint仅保留生命周期边界，不再按固定provider
  response间隔写入不断变大的全量副本。

- Entry：E通过。
- Tasks：
  1. trace event只append事件，不附带full snapshot；
  2. snapshot只在task/map lifecycle、compaction/checkpoint和显式export写入；
  3. replay从最近checkpoint加后续events重建；
  4. 删除旧per-event full snapshot production path，不做兼容读取；
  5. observer分别统计provider context和internal replay成本。
- Validation：replay state hash、resume/rollback/fork、crash recovery；`subscription-billing-repair`
  Standard/R5 Docker 1次。
- Exit：最终Map/task/result hash 100%一致；snapshot bytes下降至少80%；snapshot/rollout bytes低于30%；
  runtime event和失败reason不丢失。
- Fallback：整phase revert并丢弃实验会话，不增加dual snapshot模式。
- Next Gate：全部通过后进入G。

### J6.7.7-G：收益门禁与对抗性审查

- Status：Engineering/live complete。`0032a38`删除plain final自动provider follow-up，确定性open Map
  集成测试通过；修复后focused/complex各3 repeats全部solved，0 final rejection、0 zero cache hit。
  对抗性审查等待用户授权。

- Entry：A-F全部100%完成。
- Tasks：
  1. `count-call-stack`和`subscription-billing-repair` Standard/R5 Docker各1次；
  2. R4可执行仍不可用时只引用历史正确性，不补造成本；
  3. 输出request/input/cache/wall、cross-carrier duplicate、post-terminal、projection和snapshot指标；
  4. 经用户授权执行对抗性审查，关闭critical/high findings。
- Exit：两侧correctness通过；provider-visible known semantic duplicate=0；失败反馈100%；
  stale blank marker=0；warm cache无>2pp负收益；snapshot门禁通过；无critical/high finding。
- Fallback：正确性/反馈失败则回退对应最近phase；仅总成本方差时保留正确架构但不声明总成本收益。
- Next Gate：通过后关闭J6.7并解锁J7。

## 7. Phase Gate矩阵

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required | Proceed Decision |
|---|---|---|---|---|---|
| A | lineage fixtures + focused live | 不依赖behavior change | owner unknown=0 | 100% | complete |
| B | bootstrap/session + Docker | 不依赖terminal | blank developer=0 | 100% | complete |
| C | post-terminal reconstruction | 不依赖nested去重 | final exact occurrence=1 | 100% | complete |
| D | provider pairing + nested failure | 不依赖projection详情分层 | native pair unique | 100% | complete |
| E | global skeleton/detail tier fixtures + complex live | 不依赖snapshot storage或R5-K | skeleton 100% + ref recovery 100% | 100% | complete |
| F | replay hash + bytes gate | 不依赖benefit run | snapshot -80% | 100% | complete |
| G | paired Docker + authorized review | 不依赖J7 | all gates | 100% | engineering/live complete；review pending authorization |

任何phase未达到100%时pause，不允许后续phase补写退出证据。

## 8. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime Evidence | Mock/Stub | Status |
|---|---|---|---|---|---|---|---|
| lineage observer | 识别跨carrier机械重复 | benchmark observer | performance report | self-test | lineage events | none | complete |
| blank context removal | fresh blank只由tool contract表达 | session/action_map context | first request | session/schema | fixed message bytes | none | complete |
| terminal owner | final正文未来上下文一次 | finalization/event linearizer | finish_then_end | resume/compaction | post-terminal trace | none | complete |
| nested owner | ordinary pair各一次 | sequence/event store/linearizer | initialize_then_actions | nested matrix | pair/orphan metrics | none | complete |
| sparse success ack | 只返回新ID和状态 | taskspace handler | control output | schema tests | ack field count | none | complete |
| global Map projection | fresh走canonical自然历史；新epoch完整骨架 + 分层详情可恢复 | projection/event ref | resume/compaction | skeleton/tier/ref | coverage/detail bytes | none | complete |
| incremental replay | full snapshot只在边界 | rollout/state replay | session persistence | replay hash | snapshot ratio | none | complete |
| benefit gate | 无语义/成本负收益 | Docker benchmark | paired samples | validators | final report | none | engineering/live passed after repair; review pending |

## 9. Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Reason Field | Correlation Fields | Level |
|---|---|---|---|---|---|---|
| carrier coverage | detected/covered | `taskspace.carrier_covered` | incomplete relation | `coverage_error` | call/event/final IDs | info/error |
| terminal owner | persisted/visible | `taskspace.final_owner_committed` | duplicate/missing | `owner_error` | task/call/final event | info/error |
| nested expansion | expanded/paired | `taskspace.nested_pair_visible` | orphan/duplicate | `pair_error` | parent/call/output event | info/error |
| blank context | omitted | `taskspace.blank_context_omitted` | stale marker | `marker` | request/epoch/map | info/error |
| map detail tier | classified/rendered | `taskspace.map_detail_rendered` | protected/ref/coverage failure | `error_code` | map/node/event/tier | info/error |
| map skeleton budget | measured/within-budget | `taskspace.map_skeleton_measured` | skeleton over hard budget | `map_skeleton_over_budget` | map/epoch/bytes/nodes | info/error |
| checkpoint | written/replayed | `taskspace.checkpoint_replayed` | hash mismatch | `mismatch` | checkpoint/event range | info/error |

日志只记录ID、hash、bytes、count和错误类别，不记录API key或无界正文。

## 10. 风险与回退

| Risk | Probability | Impact | Trigger | Mitigation | Fallback |
|---|---|---|---|---|---|
| envelope折叠破坏provider pair | Medium | High | provider拒绝/Agent缺反馈 | pre-wire pairing fixture + live early gate | revert phase |
| terminal正文被误删 | Medium | High | next turn缺final | post-terminal exact reconstruction | revert C |
| 空Map信息不足 | Medium | Medium | init未发生/错误上升 | schema/choice明确且只看稳定多run证据 | revert B，不加策略提示 |
| 详情分层丢protected失败 | Low/Medium | High | protected miss | P0保护+ref恢复 | revert E |
| 全局骨架自身超预算 | Medium（长会话） | High | `map_skeleton_over_budget` | J6.7.7显式暂停；R5-K专门做可逆Map压缩 | 不返回partial map |
| snapshot减少影响恢复 | Medium | High | replay hash mismatch | lifecycle checkpoint + crash fixtures | revert F |
| 为去重重写语义 | Medium | High | body/hash变化 | field lineage机械覆盖，禁止summary | reject change |
| 缓存下降 | Medium | Medium | request2+下降>2pp | 事件首次可见前确定视图，不逐请求改旧前缀 | pause/revert |

本产品无需要保留的旧TaskSpace数据，不做兼容adapter、双写、feature flag或silent fallback。

## 11. 验收收益

J6.7.7完成后应得到：

1. final正文、ordinary tool参数和ordinary tool结果在未来provider上下文各只有一份；
2. fresh blank Map不再注入陈旧结构消息，Agent通过工具schema理解初始化合同；
3. Map全局骨架始终可见，root和近层node保留更多详情，远端过程只按机械合同降级为引用；
4. failure semantics、call pairing、role/order和output refs保持100%；
5. rollout由事件主导，snapshot不再占绝大多数存储和解析成本；
6. observer能发现post-terminal和跨carrier重复，不再只报告完整payload hash。

## 12. Open Questions

| Question | Resolution Gate |
|---|---|
| DeepSeek最终wire是否接受省略已被Map/native events覆盖的successful outer pair | D provider probe；不通过则pause |
| D1/D2图距离和详情byte/item profile具体阈值 | E用fixture与paired sample冻结；不使用正文语义优先级 |
| 全局骨架超过hard budget后的Map压缩合同 | R5-K专项，不在J6.7.7中用分页临时解决 |
| lifecycle checkpoint最小安全频率 | F crash/replay矩阵冻结 |

## 13. Decision Log

| Date | Decision | Reason |
|---|---|---|
| 2026-07-12 | J6.7重开J6.7.7 | exact payload gate漏掉跨carrier和terminal后重复 |
| 2026-07-12 | final正文归assistant final | 自然对话role优先于状态tool运输字段 |
| 2026-07-12 | native pair归ordinary语义 | 不让outer wrapper替代原生工具反馈 |
| 2026-07-12 | fresh blank不注入projection | bootstrap schema/choice已表达机械初始化合同 |
| 2026-07-12 | snapshot改为checkpoint派生物 | Event Store才是事实源 |
| 2026-07-12 | projection保留完整全局骨架 | Map视图用于持续掌握全局路径，分页会破坏该能力 |
| 2026-07-12 | 局部详情只按结构与事件类型分层 | 控制上下文成本，但不授予Runtime语义判断权 |
| 2026-07-12 | 骨架超预算独立立项R5-K | 长会话需要Map-native可逆压缩，不能用静默裁剪补丁代替 |

## 14. Change Log

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-07-12 | 建立J6.7.7 owner、A-G phases、收益和回退门禁 |
| 1.1 | 2026-07-12 | E改为完整全局骨架与局部详情分层；骨架超预算移交R5-K专项 |

## 15. Plan Quality Checklist

- [x] 每类重复有唯一owner和失败例外。
- [x] 每个phase可独立验证，未完成时默认pause。
- [x] production path、test、live evidence和mock暴露均已列出。
- [x] correctness、语义、缓存、token和日志存储收益分别验收。
- [x] 日志覆盖trigger、coverage、pair、projection detail tier、skeleton budget、checkpoint和失败原因。
- [x] 不做兼容、双写、semantic summary或Runtime动作推断。
- [x] 最终审查需要用户授权，J7保持锁定。
