# R5-J6.7 TaskSpace 单一任务上下文计划

- Created: 2026-07-12
- Updated: 2026-07-12
- Version: 1.0
- Status: Planned / implementation not started
- Owner / Responsible: WhaleCode core runtime / TaskSpace context
- Related Systems: `action_map`、`ConversationHistory`、session turn、provider prompt builder、
  `taskspace_control`、compaction、output refs、benchmark observer
- Related Links: `00-r5-taskspace-simplification-charter.md`、
  `01-r5-phased-simplification-plan.md`、`11-r5-feedback-cache-priority-plan.md`、
  `21-r5-input-token-optimization-audit.md`、
  `coe/2026-07-10-22-56-r5-request-amplification.md`
- Risk Level: High
- Plan Type: Full

## 1. 决策摘要

R5-J6.7 插入在 J6.6 input follow-up 与 J7 singular patch carrier 之间。J7 在 J6.7 完成前暂停，
避免继续扩大即将被收敛的 `taskspace_control` carrier 和双轨 provider history。

核心决策：

```text
TaskSpace Map/Event Store = TaskSpace任务上下文唯一事实源
Provider context          = 该Store的忠实、确定性线性化视图
ConversationHistory       = Standard模式事实源 + TaskSpace全局非任务上下文
```

不再维持“基础任务历史 + TaskSpace projection/control journal”两个平行事实源。TaskSpace缺少原始
role、顺序、call/result配对或完整反馈时，先补全Store合同，不用summary替代原文。

## 2. 问题定义与基线

当前TaskSpace provider上下文由以下结构共同组成：

```text
global/base messages
+ original linear task history
+ fixed epoch projection
+ raw taskspace_control journal
+ ordinary tool history
```

该布局优先保证了不丢失反馈和append-only缓存，但存在结构与语义双轨：

| 重复 | 当前证据 | 问题 |
|---|---|---|
| user task / `task_goal` | 当前init再次改写223字符任务 | root语义有两个来源 |
| user task / node goals | 3个goal共192字符 | 合理分解与复述未区分 |
| init carrier / ordinary actions | init内actions 231 bytes | 普通调用被control再次包装 |
| init output / ordinary results | 1,629 bytes中1,444 bytes为普通结果 | 88.6%是普通反馈语义 |
| raw result / `result_summary` | 本轮summary共242字符 | Agent被允许重复已有事实 |
| gate text / recovery JSON | 同一reason出现两次 | 明确机械重复 |
| call args / success ack | node/next id重复 | 必要确认与冗余字段混合 |
| raw feedback / populated projection | 新epoch可能同时保留excerpt和原文 | 潜在正文重复 |

当前Docker基线：Standard 8 requests / 57,857 input，TaskSpace 11 requests / 90,412 input。
按carrier计算，TaskSpace独有结构约28%，但其中control journal包含大量普通工具结果和Agent历史，不能
解释为28%的新增语义。

## 3. 目标与非目标

### 3.1 目标

1. 每个TaskSpace任务语义项只有一个canonical `event_id`和一个owner node/root。
2. 原始user/assistant/tool call/tool result的role、顺序、参数、结果和call配对可无损往返。
3. TaskSpace provider输入完全由Map/Event Store构造；任务项不再从基础history平行读取。
4. root直接引用原始user event；不再强制Agent复述`task_goal`。
5. 普通工具反馈只保存一次；control、result和projection引用同一事件，不复制正文。
6. Map只结构化node type/goal/status/dependency/ownership等明确字段，不生成任务策略。
7. 原始内容过长时只做透明截断、output ref和渐进式读取，不做Runtime语义摘要。
8. provider线性化保持append-only稳定前缀；compaction checkpoint可从canonical events重建。
9. Standard行为和上下文合同不回退。
10. TaskSpace固定Input和重复历史占比下降，且正确性、反馈完整性、缓存不出现负收益。

### 3.2 非目标

1. 不解析reasoning或自然语言相似度来判断重复。
2. 不让Runtime决定哪些任务事实重要、节点是否充分或下一步动作。
3. 不把所有内容扁平化为developer projection；原始role和tool pairing必须保留。
4. 不建立LLM summary/reducer、semantic ledger或第二套事件解释层。
5. 不为旧snapshot、旧Map数据或未发布会话保留兼容读取。
6. 不在本阶段实现J7 singular patch限制。
7. 不以减少节点数、自动finish或隐藏失败反馈制造token收益。
8. 不要求TaskSpace和Standard执行路径逐request相同。

## 4. 外部依据与设计约束

1. [Martin Fowler Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html)：状态变化以事件序列
   保存并可重建当前状态。J6.7采用事件作为事实源，但不引入通用CQRS框架。
2. [Model Context Protocol Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：工具结果可包含
   structured/unstructured content。TaskSpace必须保留原始content和结构化字段，不能只存summary。
3. [W3C Trace Context](https://www.w3.org/TR/trace-context/)：稳定trace/parent标识用于跨组件关联。J6.7沿用
   `task_id/map_id/node_id/event_id/call_id`，不把可变正文放入metrics label。
4. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存依赖稳定前缀。canonical
   event线性化必须保持已发送事件不变，新事件只追加；checkpoint只在新epoch/compaction产生。

这些依据只约束存储、关联、反馈和缓存机制，不授权Runtime增加任务语义。

### 4.1 复杂度与依赖

这是核心provider上下文路径的架构迁移，风险为High。错误会直接导致role扭曲、tool pair断裂、反馈丢失、
缓存退化或Agent行为异常，因此production cutover必须独立成phase并可整阶段回退。

| Dependency | Type | Status | Blocking Risk | Handling |
|---|---|---|---|---|
| `ConversationHistory` task/global边界 | system | Unknown | 无法安全move task items | J6.7.0冻结owner和激活/退出路径 |
| `ResponseItem`完整类型集合 | schema | Ready/需盘点 | codec漏类型 | J6.7.0 inventory + J6.7.1 exhaustive fixture |
| output ref/artifact store | system | Ready | 大输出无法无损承载 | J6.7.1/4复用并验证恢复 |
| provider tool call pairing规则 | third-party | Ready/需live验证 | 重建history被拒绝 | J6.7.2 provider fixture和Docker pair |
| DeepSeek prefix cache | third-party | Ready | event线性化破坏LCP | J6.7.4 same-shape cache gate |
| Docker benchmark/observer | environment | Ready | 收益不可证 | 每phase固定横向样本 |
| 对抗性审查授权 | person | Pending | J6.7.6不能关闭 | 收口前向用户申请 |

| Assumption | Verification | If False |
|---|---|---|
| 无需保留旧TaskSpace数据 | 用户已明确实验产品无保留价值 | 暂停，不自行增加兼容层 |
| Standard继续使用现有history | Standard regression和payload hash | 回退共享路径改动 |
| TaskSpace items可按原顺序线性化 | codec/provider fixture | 暂停cutover，补合同而非summary |

### 4.2 替代方案与取舍

| Option | Decision | Reason |
|---|---|---|
| 保留base + Map双轨，继续局部去重 | Reject | owner仍不唯一，重复会在新carrier和compaction继续出现 |
| 用Map summary替换原始history | Reject | role、tool pairing和失败正文会丢失，Runtime获得语义解释权 |
| 引入通用CQRS/event-sourcing框架 | Reject | 超出项目需要，增加抽象和维护成本 |
| 最小TaskSpaceEvent + deterministic linearizer | Select | 直接复用ResponseItem和现有Map/ref，删除而非新增事实源 |
| 长期shadow dual-write比较 | Reject | 用户已禁止兼容和技术债务；容易形成永久双轨 |

### 4.3 安全与权限边界

1. Store保存的raw payload沿用现有secret redaction、sandbox和tool permission结果，不扩大权限。
2. event codec不得修改approval、success、sandbox outcome或tool identity。
3. telemetry只记录IDs、bytes、hash、状态和错误类别，不落完整敏感正文。
4. output ref读取继续经过现有路径/权限/大小门禁；canonical owner不等于任意读取授权。
5. activation ownership transfer不得跨thread/task混入其他用户或subagent事件。

## 5. 目标数据模型

### 5.1 最小结构

```text
TaskSpaceTask
  id
  source_event_ids[]
  status
  active_map_id

TaskSpaceNode
  id
  kind
  goal                 # Agent-authored，非Runtime总结
  status
  dependency_node_ids[]
  event_ids[]
  result_ids[]

TaskSpaceEvent
  id
  sequence
  owner                 # root或node_id
  event_type
  original_role
  call_id / parent_call_id
  raw_payload | output_ref
  truncation_metadata
  created_at

TaskSpaceResult
  id
  node_id
  evidence_event_ids[]
  agent_conclusion_event_id?   # 只有Agent明确新增结论时存在
```

### 5.2 事件类型

| Event Type | Canonical Payload | Provider Render |
|---|---|---|
| `user_message` | 原始ResponseItem | 原始user role |
| `assistant_message` | 原始ResponseItem | 原始assistant role |
| `tool_call` | name/call_id/raw args | 原生tool call |
| `tool_result` | call_id/raw output/success/ref | 原生tool result |
| `state_transition` | before/after/reason/IDs | 最小control result或Map索引 |
| `gate_failure` | class/reason/raw mechanical facts | 单份结构化失败 |
| `artifact_ref` | ref/hash/size/range | 引用和显式读取入口 |
| `checkpoint` | covered event range/hash/omissions | compaction后Map索引 |

事件必须有全局单调`sequence`。node分组是ownership索引，provider默认按原始sequence输出，不因图分组
重排有因果关系的内容。

## 6. 所有权与边界

| Content | Canonical Owner in TaskSpace | Outside TaskSpace |
|---|---|---|
| system/developer规则 | 不进入task event store | global history |
| permissions/environment | 不进入task event store | turn/global context |
| tool schemas | 不进入task event store | provider tools |
| 原始用户任务 | root `user_message` event | 不保留任务副本 |
| Agent消息/reasoning-visible item | 当前node/root event | 不平行写base task history |
| ordinary tool call/result | 当前node event | control只引用event id |
| control call/transition | root/node transition event | 不再另建journal事实源 |
| large output | tool result metadata + output ref | artifact store保存原文 |
| node goal | node字段 | 不复制为task summary |
| result | evidence event refs | summary仅在Agent新增结论时存在 |

TaskSpace在已有Standard会话中激活时，任务相关items必须执行一次所有权转移：保留原始顺序和role，
从base task history移出并写入root events；不是复制。Phase 0必须确认当前产品是否允许退出TaskSpace；
若允许，退出时必须从events重建Standard history，不能保留双写。

## 7. 总体技术设计

1. `NodeEvent`升级为无损`TaskSpaceEvent` envelope；Map只保存event IDs和结构状态。
2. 建立`ResponseItem <-> TaskSpaceEvent`确定性codec，禁止正文推断。
3. TaskSpace激活后，session记录入口按事件类型直接写canonical store。
4. provider prompt builder读取global context和TaskSpace event linearizer，不再读取task-scoped base history。
5. `taskspace_control`执行产生transition events；nested ordinary结果直接成为node tool events。
6. control function output由同一events构造，满足provider call/output协议，但不创建第二份语义记录。
7. projection降级为Map索引/checkpoint renderer，只展示结构和引用，不复制仍可见raw events。
8. compaction按event range生成checkpoint和output refs；被覆盖event仍可恢复，provider只展示一次有效内容。

## 8. 分阶段执行

### Phase J6.7.0：事实源与路径审计

**目标：** 冻结所有任务上下文写入、读取、替换和压缩入口，证明切换范围完整。

- Entry：J6.6 input follow-up代码和Docker证据可用。
- Tasks：
  1. 列出user/assistant/tool/control/gate/compaction各类ResponseItem的写入和provider读取路径。
  2. 对每类内容标记canonical owner、当前副本数、role/order/call pairing风险。
  3. 记录TaskSpace激活/退出/恢复/压缩语义；未知项不得进入J6.7.2。
  4. 扩展observer输出carrier重复计数，但不做语义相似度判断。
- Deliverables：ownership matrix、call graph、当前payload component baseline、CoE假设。
- Validation：`count-call-stack`与`multi-file-order-pipeline`各1次Standard/R4/R5；只作基线。
- Exit：所有production task-item入口100%有owner，无`unknown`；否则pause。
- Review/Risk：架构owner审查；主要风险是漏掉compaction/subagent隐藏入口。
- Fallback：无代码行为变更，删除未完成观测改动。
- Next Gate：证据完整后进入J6.7.1，否则停止。

### Phase J6.7.1：无损事件合同与往返codec

**目标：** 证明TaskSpace Store有能力完整承载基础任务上下文，尚不切production owner。

- Entry：J6.7.0 100%完成。
- Tasks：
  1. 定义最小`TaskSpaceEvent`和global sequence，不增加semantic fields。
  2. 实现ResponseItem/tool output/image/ref/control failure往返codec。
  3. 覆盖function/custom/MCP/code-mode输出、success、truncation和output ref。
  4. codec仅在fixture/test使用；不得在production双写影子events。
- Deliverables：schema/types、round-trip fixtures、大小与敏感数据审计。
- Validation：逐字段相等：type、role、name、call_id、arguments、output、success、content、order；
  `large-output-ref-smoke`和`count-call-stack`横向各1次确认production无变化。
- Exit：所有已审计event type往返100%；unsupported type必须显式失败；否则pause。
- Review/Risk：schema/codec审查；主要风险是测试只覆盖text而漏image/MCP/custom output。
- Fallback：回退本phase提交，不影响production path。
- Next Gate：codec矩阵100%后进入J6.7.2。

### Phase J6.7.2：Canonical Store原子切换

**目标：** TaskSpace任务items只写Store，provider只从Store读取；同一phase删除旧task-history路径。

- Entry：J6.7.1 100%完成，切换影响清单review通过。
- Tasks：
  1. 激活时对已有任务items执行一次move，不copy，保持sequence和role。
  2. 切换user/assistant/tool/control记录入口到canonical store。
  3. provider linearizer输出原生message/tool call/tool result结构。
  4. 删除TaskSpace对base task history的写入和读取；global context继续使用原history。
  5. 禁止silent fallback；event缺失、pair断裂或sequence冲突必须显式失败并记录。
- Deliverables：唯一production path、provider payload diff、删除清单。
- Validation：session/unit/integration；`count-call-stack`和`multi-file-order-pipeline`横向各1次。
- Exit：每个provider-visible task item有且只有一个source event；round-trip mismatch=0；
  base task-history read/write count=0；correctness不回退；否则整phase回退。
- Review/Risk：production cutover专项审查；主要风险是role/order/tool pair断裂和global item误迁移。
- Fallback：git revert整个cutover提交并丢弃测试会话；不保留运行时compat开关。
- Next Gate：single owner与live provider同时通过后进入J6.7.3。

### Phase J6.7.3：Map/control语义去重

**目标：** 删除已确认的平行复述，让control只表达Map结构和状态变化。

- Entry：J6.7.2 100%完成。
- Tasks：
  1. root task由`source_event_ids`建立，删除强制`task_goal`复述；Agent可新增独立goal event。
  2. nested ordinary calls/results归属node events；control aggregate引用这些event IDs。
  3. `result_summary`不再作为默认result字段；已有events机械成为evidence refs。
  4. Agent明确新增结论时写`agent_conclusion_event_id`，Runtime不生成。
  5. gate failure只生成一份typed result，reason不再同时存在文本和JSON两份。
  6. success ack只返回无法由调用参数推出的committed IDs/状态。
- Deliverables：新tool schema/typed parser/handler、删除旧字段，无兼容分支。
- Validation：provider schema probe；control failure fidelity；`count-call-stack`与
  `subscription-billing-repair`横向各1次。
- Exit：已知重复字段为0；raw tool feedback恢复率100%；protocol/state错误分类准确；否则pause。
- Review/Risk：tool schema与Runtime边界审查；主要风险是删除summary时误删Agent新增结论。
- Fallback：回退本phase schema和handler提交；不接受新旧字段并存。
- Next Gate：provider probe和反馈门禁通过后进入J6.7.4。

### Phase J6.7.4：Projection与Compaction收敛

**目标：** projection只做Map索引/checkpoint，原始事件可见时不复制excerpt。

- Entry：J6.7.3 100%完成。
- Tasks：
  1. renderer输入只接受Map结构、event IDs、ref和机械omission metadata。
  2. 按`raw-visible / referenced / checkpoint-covered`机械状态决定暴露形态。
  3. raw event仍在provider输入时，projection不得再次输出正文excerpt。
  4. compaction checkpoint记录covered sequence range、hash、refs和明确省略原因。
  5. 保持prefix append-only；不得逐请求重写前部完整Map。
- Deliverables：checkpoint contract、progressive exposure测试、cache trace。
- Validation：`large-output-ref-smoke`与`multi-file-order-pipeline`横向各1次；压缩前后
  task/tool facts、role、pairing和可恢复引用一致。
- Exit：provider raw-body duplicate count=0；protected miss=0；ref recovery=100%；
  active warm prefix不低于当前约97%的同shape基线超过2个百分点；否则pause。
- Review/Risk：compaction/cache专项审查；主要风险是checkpoint覆盖范围错误或ref不可恢复。
- Fallback：回退checkpoint提交；不得回退到语义summary。
- Next Gate：语义与cache门禁同时通过后进入J6.7.5。

### Phase J6.7.5：旧双轨代码物理删除

**目标：** 删除不再调用的epoch/task-history composer、旧summary字段和兼容测试。

- Entry：J6.7.4 100%完成。
- Tasks：
  1. call/import graph确认旧路径无production caller。
  2. 删除旧TaskSpace task history、projection正文复制、legacy parser和dead telemetry。
  3. 按模块职责拆分Store/codec/linearizer；Whale自有单文件原则上不超过500行。
  4. 更新docs/CoE/observer字段，不保留旧artifact兼容。
- Deliverables：删除清单、模块边界、clean build。
- Validation：全量相关Rust tests、PowerShell harness、locked build；`count-call-stack`横向1次。
- Exit：旧路径caller=0、compat branch=0、dead duplicate fields=0、git clean；否则pause。
- Review/Risk：删除和模块边界审查；主要风险是误删Standard共享history能力。
- Fallback：回退本phase删除提交；不恢复已废弃production owner。
- Next Gate：全量回归和call graph通过后进入J6.7.6。

### Phase J6.7.6：收益门禁与收口

**目标：** 证明单一事实源带来语义、维护和成本正收益，决定是否解锁J7。

- Entry：J6.7.5 100%完成。
- Tasks：
  1. 运行focused + complex各1个Standard/R4/R5 Docker样本。
  2. 输出request、input/cached/uncached、component bytes、重复计数、map健康和反馈完整性。
  3. 对比J6.6和J6.6 follow-up历史证据，明确并发路径方差，不伪造因果。
  4. 经用户授权后执行对抗性审查，关闭所有critical/high findings。
- Samples：`count-call-stack`、`subscription-billing-repair`。
- Exit：两侧correctness通过；canonical duplication=0；语义round-trip=100%；
  TaskSpace固定结构和control-history重复下降；无>2个百分点warm-cache负收益；
  无未关闭critical/high review finding。通过后才解锁J7。
- Review/Risk：经用户授权执行对抗性审查；主要风险是单样本方差掩盖结构负收益。
- Fallback：若正确性或语义门禁失败，回退到J6.7.5前的最后完整phase；若只有成本未改善，
  保留正确架构但不声明性能收益，并暂停J7等待用户决策。
- Next Gate：全部通过后更新R5/CoE并解锁J7，否则保持pause。

## 9. Phase Gate矩阵

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Next Decision |
|---|---|---|---|---|
| J6.7.0 | inventory/call graph/baseline | 不依赖codec | owner无unknown | proceed/pause |
| J6.7.1 | round-trip fixtures | 不依赖production cutover | all event types 100% | proceed/pause |
| J6.7.2 | provider payload + live pair | 不依赖dedupe | single source path | proceed/revert |
| J6.7.3 | schema/handler/provider probe | 不依赖compaction | known duplicates zero | proceed/pause |
| J6.7.4 | compaction/ref/cache sample | 不依赖dead-code deletion | no raw duplicate/miss | proceed/pause |
| J6.7.5 | call graph/tests/build | 不依赖benefit run | old path zero caller | proceed/pause |
| J6.7.6 | Docker evidence/review | 不依赖J7 | correctness + benefit gate | unlock J7/pause |

任何phase未达到100%时默认pause；后续phase不得补写前一phase的退出证据。

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime Evidence | Status |
|---|---|---|---|---|---|---|
| event envelope | 原始ResponseItem无损承载 | `action_map/map.rs`或新`event_store.rs` | session record | round-trip matrix | event codec trace | planned |
| canonical ingress | task item只写一次 | session/tool completion path | TaskSpace turn | ownership tests | source_event_id | planned |
| provider linearizer | 从Store恢复原生roles/tools | session turn prompt builder | provider request | payload equality | linearization trace | planned |
| control dedupe | transition引用events | taskspace handler/sequence | control call | schema/failure tests | transition trace | planned |
| checkpoint/ref | 渐进暴露无正文重复 | projection/compaction/output ref | context pressure | compaction tests | omission/ref trace | planned |
| old path deletion | 无双写和兼容 | history composer/runtime | all TaskSpace turns | call graph/build | old_path_count=0 | planned |
| benefit gate | 正确性和成本可证明 | Docker benchmark | paired run | validators | performance report | planned |

仅production path接通并取得runtime证据后可标记`landed`；schema、fixture或test-only codec不能单独完成迁移。

## 11. Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Reason Field | Correlation Fields | Level |
|---|---|---|---|---|---|---|
| ownership transfer | started/committed | `taskspace.context_ownership_transferred` | partial transfer | `reason` | task/map/event range | info/error |
| event record | validated/recorded | `taskspace.event_recorded` | codec/owner failure | `error_code` | task/map/node/event/call | info/error |
| call/result pair | paired | `taskspace.tool_pair_closed` | orphan/duplicate | `pair_status` | event/call/parent_call | info/error |
| provider linearize | started/completed | `taskspace.context_linearized` | order/role mismatch | `reason` | request/epoch/event range | info/error |
| transition | committed | `taskspace.transition_recorded` | state reject | `gate_reason` | task/map/node/event | info/warn |
| checkpoint | created/verified | `taskspace.checkpoint_verified` | protected miss | `missing_event_ids` | epoch/range/hash | info/error |
| output ref | created/read | `taskspace.output_ref_resolved` | missing/corrupt | `error_code` | event/ref/call | info/error |
| duplicate audit | measured | `taskspace.canonical_duplicate_count=0` | duplicate found | `duplicate_class` | request/event IDs | info/error |

日志不得记录API key、完整敏感正文或无界正文hash列表；失败和pair断裂不得采样丢弃。

## 12. 风险、缓解与回退

| Risk | Probability | Impact | Trigger | Mitigation | Fallback |
|---|---|---|---|---|---|
| role/order扭曲 | Medium | High | round-trip mismatch | 原生ResponseItem codec | phase revert |
| tool pair断裂 | Medium | High | orphan call/result | call_id contract + hard test | phase revert |
| compaction丢反馈 | Medium | High | protected miss/ref失败 | checkpoint range/hash/ref | pause/revert |
| cache下降 | Medium | Medium | same-shape warm hit下降>2pp | append-only event order | pause/revert phase |
| Store变成语义控制器 | Medium | High | 新summary/hint/inference字段 | schema review + forbidden scan | reject change |
| 临时双写长期存在 | Medium | High | base/store同event双owner | J6.7.2原子切换 | whole-phase revert |
| provider协议不接受重建历史 | Low/Medium | High | API/tool pairing error | live provider fixture early | pause/revert |
| observability膨胀 | Medium | Medium | rollout/log异常增长 | metadata-only bounded logs | reduce telemetry正文 |

本产品无需要保留的历史TaskSpace数据，不做数据迁移、兼容读取、feature flag双轨或silent fallback。

## 13. 收益指标

| Hypothesis | Baseline | Target | Method | Pass Threshold |
|---|---:|---:|---|---|
| canonical source唯一 | 双轨 | 单一 | source event audit | duplicate owner=0 |
| 语义忠实 | 当前focused通过 | 不回退 | round-trip + validators | 100% fields/pairs |
| raw反馈重复 | 已知init 88.6%普通结果包装 | 0重复正文 | final-wire scan | raw duplicate=0 |
| 固定上下文成本 | final active约10,303 input | Unknown | paired request components | 不高于基线且有解释 |
| warm cache | active约97.08% | >=95.08% | request 2+ same shape | 下降不超过2pp |
| 维护复杂度 | base + projection + journal | store + constructor | call graph/LOC/owner | old task path caller=0 |

成本target不预设不可信降幅；J6.7.0冻结更精确的component baseline后再写入目标值。

## 14. 决策记录与开放项

| Decision | Status | Reason |
|---|---|---|
| J6.7先于J7 | Accepted | 避免在待删除carrier/history上继续扩展协议 |
| TaskSpace Store为唯一任务事实源 | Accepted | 消除平行上下文和语义复述 |
| 无旧数据兼容 | Accepted | 实验产品无保留价值，避免技术债务 |
| provider保持原生role/tool结构 | Accepted | developer文本扁平化会扭曲语义 |
| 不用semantic similarity runtime | Accepted | Runtime不得判断自然语言语义 |
| activation退出语义 | Discovery in J6.7.0 | 必须审计当前真实产品路径后冻结 |
| final fixed-cost target | Discovery in J6.7.0 | 当前只完成carrier级估算 |

## 15. 完成定义

J6.7只有同时满足以下条件才完成：

1. TaskSpace任务上下文只有Map/Event Store一个canonical owner。
2. base history不再保存或提供task-scoped items。
3. 原始role、顺序、call/result、正文/ref无损。
4. 已知user/task_goal、tool/control、result summary和gate双表达已删除。
5. projection只做Map索引/checkpoint，无策略、summary和可见raw正文复制。
6. compaction/ref/cache门禁通过。
7. focused和complex Docker样本correctness通过并形成性能报告。
8. 所有改动已测试、记录、commit、push，worktree clean。
9. 经用户授权执行的对抗性审查无未关闭critical/high finding。
10. R5总计划与CoE一致更新，J7才从blocked改为ready。
