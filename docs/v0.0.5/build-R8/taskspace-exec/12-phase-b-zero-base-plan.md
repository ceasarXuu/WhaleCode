# Phase B 零基线重建计划

- Created: 2026-08-06
- Status: Active / Phase B0-B2 and Phase B3 EX-05, MS-01～MS-02 verified offline / MS-03 next
- Supersedes: [`02-engineering-plan.md`](02-engineering-plan.md) 中 TX-06B 之后的兼容迁移顺序
- Completed foundation: TX-06A (`54fc781fc`)
- Paid Whale Agent run: 本阶段删除与离线建设不需要

## 1. 决策

TaskSpace Exec 不再从当前 sibling 协议渐进迁移。先删除旧协议对 schema、parser、handler、sequence、context、response
和 feedback 的影响，再从 Standard 原生链与 canonical Action Map 原语建设新协议。

Phase B0 完成后，用户进一步冻结最简 Map：节点直接包含 goal、state、content、parents、Agent 可见 children 和 actions；
Agent 只声明 parents，Runtime 机械反算 children；Map 不再拥有顶层 edges、任何 `*_ref`、语义分类模块或生命周期子账本。
这不是对 v3 字段搬家或改名，而是从新产品模型重建 canonical Map。旧 Map 代码只要不符合新模型、已经失效或没有生产
消费者，就连同类型、调用链、测试和文档入口彻底删除。

不做：

1. 旧 `taskspace_control.actions[]` 到新 calls 的 adapter；
2. 旧、新两套 Agent-visible schema 或 parser；
3. 为保持旧 TaskSpace 可运行而增加 feature branch、fallback 或补字段逻辑；
4. 把旧 sibling tests 改名后继续作为新合同；
5. 从旧 handler 参数反推新的 Map Tool 合同。
6. 将 `result_ref` 改名为 `result`、将顶层 edge 搬成另一种平行关系，或保留 event/replay/detail-fold 等无生产价值的旧层；
7. 用 deprecated、legacy、dormant、adapter、fallback、双写或“暂时保留”延续旧 Map 设计。

旧 TaskSpace rollout、专用对话事件和未发布数据不提供迁移、读取或 fallback；零基线只接受 Standard
原生历史格式与独立 canonical Action Map Store。

## 2. 零基线边界

### 2.1 可复用白名单

| Foundation | Reuse Boundary |
|---|---|
| Standard ToolSpec / Tool registry plan | Tool 事实、静态 schema、能力排序；TaskSpace 不修改普通 Tool |
| Standard ToolRouter / ToolCall | 原生权限、sandbox、hook、handler 和结果执行 |
| Provider response lifecycle | 原始 Function Call、Hosted output item、call ID 和完成边界 |
| Canonical Action Map | 只复用经重新证明仍符合最简模型的 SQLite 持久化、revision/CAS 和图硬校验基础；v3 schema、顶层 edges/ledger、间接状态推导、event/replay、detail-fold 与无消费者代码不在白名单 |
| Projection 三模式 | 只保留 Map 如何进入 context 的既有产品差异，不参与动作协议 |
| TX-06A neutral capability projection | Function/Freeform/Namespace 的单一机械 ToolSpec 投影 |
| Cache gate / request usage evidence | Standard 0-diff、静态 Tool declaration 和成本验证 |

### 2.2 必须删除的旧影响

| Surface | Old Influence To Remove |
|---|---|
| Tool declaration | `taskspace_control` 旧 schema 及 `actions[]` sibling manifest |
| Parser / handler glue | 旧 `TaskSpaceControlArgs`、wire parser、actions 校验和 sibling 专用输出 |
| Shared sequence | control-first、manifest/sibling 配对、TaskSpace barrier 与 prepared sibling executor |
| Runtime context | sibling call 的 node metadata、terminal carrier、额外 pairing receipt |
| Provider response | named control/tool-choice gate、隐藏 Tool 后置拒绝、terminal follow-up 控制 |
| Feedback | developer factual duplicate、supplemental carrier、状态与 Tool 失败重复包装 |
| Conversation backend | 专用 `TaskSpaceEventStore`、单 call owner、outer-control parent 和区别于 Standard 的历史替换路径 |
| Canonical Map v3 | 顶层 `edges[]`、action/result/evidence/completion/block/terminal ledger、`source_refs`、`reason_ref`、`summary_ref`、结果/evidence ref 和 Agent 手填 expected revision |
| Map runtime leftovers | 无生产消费者的 `graph_events`、`node_events`、MapFact replay、detail-fold/archive、旧 projection/snapshot 字段和仅验证旧 shape 的 fixture；实施前逐调用链坐实，坐实后直接删除 |
| Phase A prototype | `source:string` exec、Agent 回显 version/capability/item ID 的 decoder/preflight |
| Tests / prompts | 以旧合法序列、旧字段或旧反馈为正确答案的 active fixture 与说明 |

历史文档和 benchmark evidence 可保留，但不能被编译、注册、加载或作为新测试 fixture 输入。

### 2.3 参考依据

以下资料只用于校验当前状态、历史和 schema 边界，不意味着引入新的工作流框架、CQRS 服务或 Event Store：

1. [LangGraph：Persistence](https://docs.langchain.com/oss/javascript/langgraph/persistence)
   将当前 graph state 保存为 checkpoint。对应本计划：Map 保存当前有效状态，不复制 Standard 的完整 Tool 历史。
2. [Temporal：History service architecture](https://github.com/temporalio/temporal/blob/main/docs/architecture/history-service.md)
   区分 current mutable state 与 event history。对应本计划：Map 是当前工作状态，Standard rollout/log 负责执行历史；不把旧
   event/replay 层当作 Map 必需结构。
3. [Stately/XState：Events and transitions](https://stately.ai/docs/transitions)
   transition 与状态节点直接关联。对应本计划：依赖关系在 Node 中直接可见，但 TaskSpace 不引入 Runtime 可执行的自然语言 guard。
4. [Serde：Container attributes](https://serde.rs/container-attrs.html)
   `deny_unknown_fields` 可明确拒绝未知字段。对应本计划：新 schema 直接拒绝旧字段，不以忽略字段实现静默兼容。

## 3. 工作单元

| ID | Objective | Location / Target | Concrete Action | Resulting Behavior / Benefit | Side Effects | Verification / Stop | Status |
|---|---|---|---|---|---|---|---|
| ZB-01 | 冻结零基线边界 | R8 global constraints、TaskSpace Exec docs | 写明白名单、删除面和禁用过渡方案 | 后续实现不再被旧 schema/迁移思路牵引 | Complexity: 文档权威切换；Reach: 后续全部单元 | 文档交叉引用一致；旧计划明确 superseded | verified |
| ZB-02 | 删除 Phase A active prototype | `core/src/tools/taskspace_exec/` | 删除 source decoder、旧 plan/preflight/reconcile/catalog 接入；保留 TX-06A 中立投影 | 新实现没有伪 carrier、旧字段或候选 parser 可误用 | Complexity: 净删除；Reach: 仅未注册原型和 tests | `rg` 无 active `source/taskspace.plan/capability_id/item_id`；core build | verified |
| ZB-03G | 切换清零期缓存门禁 | cache final-wire fixtures、cache surface contract | 删除依赖旧 TaskSpace wire 的主动夹具；清零期只比较 Standard 两请求 final wire | 旧协议删除不会因夹具崩溃变成 `uncomparable`，同时 Standard 缓存回归仍阻断 | Complexity: 控制面净删除；Reach: 新 TaskSpace 发布保持阻断 | policy-only commit；Standard final-wire 与 cache gate PASS | verified |
| ZB-03A | 断开旧 control Tool 暴露 | `codex-tools taskspace_tool*`、registry plan、handler kind | 删除旧 declaration、ToolSpec 插入和 Router handler 注册 | Agent 和 Provider 不再看到旧 Map/sibling wire | Complexity: 净删除 Tool 声明；Reach: TaskSpace 暂不可运行，Standard Tool 不变 | registry/spec tests、Standard Tool snapshot；不得保留 adapter | verified |
| ZB-04A | 恢复 Standard 流式 Tool 调度 | `stream_events_utils.rs`、`session/turn.rs` | Tool item 到达时直接构造原生执行 future；恢复 `FuturesOrdered` 收集与统一落账；删除 response-completed 批处理入口和 provider declaration 中间态 | 普通 Tool 重新沿 Standard 原生 ToolCallRuntime 执行，不再等待旧 TaskSpace response sequence | Complexity: 一个原子执行链切换；Reach: response stream 与 Tool result 入历史 | core build；Standard Tool stream tests；不得同时保留 declaration 与 future 双轨 | verified |
| ZB-04B | 删除 sequence 执行抽象及附属状态 | `sequence*`、`provider_tool_declaration.rs`、`parallel.rs`、`context.rs` | 删除 manifest/preflight/prepared-sibling、sequence-only runtime 方法、node metadata context 与 active tests | 共享工具层不再携带旧 TaskSpace 序列、归属或 barrier 概念 | Complexity: 大量净删除；Reach: Tool runtime 与 tests | `rg` 无 active sequence/provider declaration/sibling 调用；parallel/router tests | verified |
| ZB-04C | 证明 Standard 执行基线 | Standard Tool/response/cache fixtures | 验证并行能力仍由 Tool 原生 parallel-safety 决定，串行 Tool 仍由原生锁保证；增加旧执行层 forbidden audit | 删除旧层没有改变 Standard Tool 行为或 provider 前缀 | Complexity: 低成本门禁；Reach: Phase B 后续全部单元 | Standard response/tool tests、cache gate PASS、forbidden set 为零 | verified |
| ZB-03B | 删除旧 control parser/handler | core control handler/args/output files | 删除已无调用的旧 wire parser、actions 校验、handler 和 sibling 输出 | 新 Map Tool 必须从 canonical Map operation 重新设计 | Complexity: 净删除实现；Reach: canonical Action Map 不变 | `rg` 无旧 control 类型或 actions wire；core build | verified |
| ZB-05 | 删除旧 Provider response 控制 | `session/turn.rs`、provider declaration/context helpers | 删除 named-control gate、terminal carrier、后置 follow-up/reject 和重复事实注入 | 新 response envelope 从原生完成事件零基础建立 | Complexity: 净删除分支；Reach: turn lifecycle 与 snapshots | Standard response tests、cache gate；不得新增临时 fallback | verified |
| ZB-06A | 删除旧协议说明与 active fixtures | TaskSpace base instructions/skill、旧 active tests | 移除要求旧 control/sibling/actions 的加载内容和测试 | Agent 不再接收已失效协议，测试不再奖励旧行为 | Complexity: 净删除内容；Reach: TaskSpace 暂无工作协议 | prompt/context snapshots；历史 docs 不计 active residual | verified |
| ZB-06B | 从 Map 删除工具执行状态 | rooted DAG、protocol、snapshot、Viewer | 删除 reservation/tool name/call index/release；只保留 Agent 声明的 `action_id -> node_id` 事实，结果引用不驱动节点生命周期 | Map 不再替 Tool 执行管理节点；Agent 可在动作尚无结果时完成节点 | Complexity: canonical schema 直接升级且不迁移；Reach: Map Store 新数据、API 和 Viewer | replay/invariant/schema/Viewer tests；旧字段静态为零 | verified |
| ZB-06C | 恢复 Standard 原生对话历史 | `TaskSpaceEventStore`、session state、rollout reconstruction | 删除 TaskSpace 专用历史替换、单 call owner、outer-control parent 和专用 compaction checkpoint；所有模式复用 Standard `ContextManager` | 新 Exec 从相同自然上下文基线建设，旧绑定模型不再预设 Hosted/client 归属 | Complexity: 删除第二历史后端；Reach: session/resume/compaction | Standard history/resume/compaction tests；canonical SQLite Map Store 保持不变 | verified |
| ZB-07 | 证明零基线 | 全仓 active source/test/config | 建立 forbidden-symbol audit 和 Standard regression | 新建设从可证明的干净基线开始 | Complexity: 一个静态审计；Reach: pre-commit 增加低成本检查 | forbidden set 为零、Standard exact wire、cache gate PASS | verified |
| MM-00 | 冻结最简 Map 合同 | R8 global constraints、`00-product-contract.md`、本计划 | 写明 Node 全字段、parents 单一写入、children 必显、Standard 结果复用和旧设计净删除原则 | 后续代码只能实现已确认模型，不能从 v3 字段反推新模块 | Complexity: 切换活动设计权威；Reach: 全部未开始单元 | 三份活动文档无 edges/ref/ledger 目标冲突；用户确认记录可追溯 | verified |
| MM-01 | 生成旧 Map 删除清单 | `protocol/taskspace.rs`、`core/action_map/**`、state Store、snapshot/Viewer、scripts/tests | 逐类型和生产调用链标记 keep/delete；无消费者、只由测试引用或与新模型冲突的一律列入当前阶段删除，不建立候选保留区 | 清理基于可复核证据，又不把 discovery 变成旧代码保留理由 | Complexity: 一份短清单；Reach: 决定 MM-02～MM-10 删除面 | `rg`、callers、Store/schema 证据；每个非 keep 项有删除单元；出现重大产品语义缺口才停 | verified |
| MM-02 | 替换 canonical protocol schema | `protocol/src/taskspace.rs` / canonical Map、Node、Action | 建立 map_id/root/work_nodes/finish/revision 与 Node goal/state/content/parents/actions；删除 edges、所有 `*_ref`、source、completion/block/terminal ledger 和旧 v3 类型，不做 migration | Map 序列化只包含最简当前状态；旧数据明确失败 | Complexity: 破坏性 schema 替换且总体删减；Reach: 所有 Map caller/fixture | round-trip、unknown-field rejection、旧 shape reject；新 schema 出现语义分类模块即停 | verified |
| MM-03 | 建立 parent 图硬规则与 children 视图 | `core/action_map` graph validator、Node view | 只从 Agent 声明的 parents 构图；校验唯一 Root/Finish、多 parent、多 child、端点、重复、自环、环、双向可达；机械反算 children | Agent 在每个 Node 直接看到父子关系且无需双写 | Complexity: 以 parent adjacency 替换 edge table；Reach: readiness/projection | fork/join/cycle/reachability fixtures；canonical bytes 无 children，所有 Agent-visible Node 有 children | verified |
| MM-04 | 建立直接节点状态与内容事务 | `core/action_map` transaction/state | 用 Node.state 和 Node.content 的原子更新替换 completion/block/terminal 间接记录；finish 原子完成 Finish/Root，reopen 继续同一 Map | 状态和重要语义直接可读，不再跨账本推导 | Complexity: 删除旧 transition facts，保留一套状态表；Reach: finish/reopen | 状态转换表、finish/reopen、stale request tests；Tool outcome 不改变 state | verified |
| MM-05 | 建立最小 node actions | canonical Action、TaskSpace Exec binding seam | 只保存真实 action identity、Tool 名和机械 outcome；client 单节点及 Hosted 多节点归属均写入目标 Node actions；不保存参数、输出或 ref | Map 记录必要归属而不复制 Standard Tool 历史 | Complexity: 一个小 action record；Reach: provider/client reconciliation | 0/1/N action、Hosted 多节点、冲突身份、Tool failure fixtures；Map 中无原始输出 | verified |
| MM-06 | 删除旧 Map 执行与压缩层 | `rooted_dag/events.rs`、replay、`graph_events`、`node_events`、detail-fold/archive 及 MM-01 坐实的同类代码 | 在新 transaction 可工作后直接删除无生产价值的事件重放、旧折叠和辅助状态及其专属测试，不改名搬家 | Map 始终从 SQLite canonical state 读取，不靠聊天或旧事件重建，也无 dormant 旧模型 | Complexity: 净删除模块/字段/tests；Reach: action_map runtime | active caller/static audit 为零；Store restart 从 canonical JSON 恢复；发现真实生产责任则拆出明确 keep 证据 | verified |
| MM-07 | 替换 canonical Store 合同 | `state/runtime/taskspace_maps*`、`core/session/taskspace_store*` | SQLite 只保存新 canonical Map 与必要 CAS 元数据；删除旧 shape 解码、重复 terminal/revision 列、migration、fallback 和旧 fixture | 独立 Map 持久存在且只有一个事实源 | Complexity: 破坏性表/codec 简化；Reach: restart/debug | create/update/restart/hash/CAS tests；旧 schema 失败；不得加 dual-read/write | verified |
| MM-08 | 重建 projection 与 snapshot | `core/action_map/runtime/projection.rs`、snapshot/protocol | 从 canonical Node 直接输出 goal/state/content/parents/actions，并为每个 Node 补全 children；删除旧 result/evidence/event/sentinel/maintenance 空字段 | 三种 projection policy 共享同一完整 Map 语义，只改变进入 context 的方式 | Complexity: 删除旧 projection shape；Reach: context/snapshot/cache fingerprint | always/append/request deterministic fixtures；每个 Node 含 parents+children；无重复权威 | verified |
| MM-09 | 更新所有生产消费面 | CLI debug、Viewer、export/benchmark parser、skills/tests | 直接改读新 Node shape并删除旧 alias、拼装器和无消费者 UI 字段；不提供 v3 adapter | 调试、可视化和评测不会复活旧模型 | Complexity: 消费端破坏性更新；Reach: CLI/TS/scripts | build/typecheck/snapshot/report fixture；全仓 active consumer 无旧字段 | verified |
| MM-10 | 建立最简 Map 零残留门禁 | `scripts/taskspace-exec/check_zero_base.py`、schema fixtures、pre-commit | 对结构化 schema/AST 与消费者建立门禁，阻止 edges、Map `*_ref`、旧 ledger、parent/child 双写输入和 MM-01 删除符号回流 | 后续 Exec 只能依赖新模型 | Complexity: 扩展一个静态门禁；Reach: Map 变更提交 | 正反 fixture、targeted suites、cache gate；避免把 Standard output-ref 误报为 Map ref | verified |
| EX-01 | 建立最小 Map 操作合同 | canonical transaction 邻接模块、新内部 Map ToolSpec | 定义 initialize/update/read/reopen/finish；关系变化随普通 Node parents 更新，不增加 connect/disconnect Tool；Agent 不填 revision | Agent 用少量操作维护同一 Map，无额外关系协议 | Complexity: 一个新合同；Reach: Exec schema | schema/parser/transaction fixtures；出现独立 edge/ref/binding Tool 即停 | verified |
| EX-02 | 建立结构化 TaskSpace Exec schema | 新 `taskspace_exec` declaration/catalog | 从 Standard ToolSpec 与 EX-01 生成结构化 calls/hosted_bindings variants，普通 Tool 只暴露一次 | Agent 在一个 Function Call 中声明合法序列和节点归属 | Complexity: 一个静态 Function schema；Reach: declaration/cache | 1/N client、map+work、hosted-only、empty reject、确定性字节 | verified |
| EX-03 | 建立 request-local revision 与身份 | provider request context、outer call envelope | Runtime 记录请求所见 revision、ToolSpec identity 和 outer call identity；Agent schema 不暴露 expected_revision/version/capability ID | 并发安全不转化为 Agent 填表成本 | Complexity: 一个短生命周期 envelope；Reach: request lifecycle | stale concurrent response、retry、same-revision fixtures；不跨 response 持久化 | verified |
| EX-04 | 建立零副作用 preflight | Exec parser、Map candidate transaction、Tool batch validator | 在 client/map 副作用前完成结构、能力、节点、DAG、状态、单 Patch 和 Hosted 声明完整性检查 | 非法计划明确拒绝且普通 Tool/Map 零执行 | Complexity: 一个预检入口；Reach: all TaskSpace calls | failure matrix 逐项 fixture；不得加入语义判断或修复建议 | verified |
| EX-05 | 接入 client 原生 dispatch | Exec executor、`ResponseItem`、ToolRouter | 将通过预检的内部 client calls 机械还原为原生 `ResponseItem`，再走现有 `ToolRouter::build_tool_call` 与 `ToolCallRuntime`；不复制 alias/MCP/Tool Search 分支，结果原样返回 | TaskSpace 复用 Standard Tool 能力且不侵入 Tool | Complexity: 一个 dispatch adapter；Reach: client tools，不接 Map Store | Function/Freeform/Namespace/Tool Search、并行/串行、失败 tests；Standard exact wire 0-diff | verified |
| MS-01 | 将 canonical Map 分解为关系化唯一事实 | `state` migration、`taskspace_maps/nodes/node_parents/node_actions` | 以 Map head、Node、parent relation 和归属 Node 的 Action 表直接持久化当前 Map；删除整图 `canonical_json` 作为生产写模型，不迁移实验数据 | 工具结果只改变所属 Action 行，Map 仍是一份固化事实 | Complexity: 四类天然实体表，无 Event Store/双写；Reach: state schema 与 Store | schema/FK/index tests；出现整图镜像、delta replay 或兼容 reader 即停止 | verified |
| MS-02 | 建立细粒度 canonical Store transaction | `state/runtime/taskspace_maps*`、`core/session/taskspace_store*` | 用 Map head revision CAS 保护整个 DAG，同一短事务内只 insert/update/delete 变更的 Node/parent/Action 行；读取时组装回同一 canonical domain Map | 高频 outcome 结算不再重写整图，并发仍有全图硬约束 | Complexity: 一个 repository，删除整图 hash 热路径；Reach: hydrate/CAS/restart | create/update/delete/restart、fork/join、stale CAS、无孤立 Action tests | verified |
| MS-03 | 接入低延迟 Action 结算 | Exec coordinator、canonical Store | 整个 Exec 先完成一次零副作用预检；候选 Map 与 client `Pending` 归属持久化成功后立即 dispatch，各 Tool 一旦完成就独立结算 outcome，不等待同批其他 Tool | 不人为增加 Tool 依赖或反馈延迟，Map 以最低可行延迟反映已发生事实 | Complexity: 每 Map 仅短提交串行化，Tool 执行不串行化；Reach: revision/WAL/commit metadata | 独立快慢工具、部分失败、取消、崩溃遗留 Pending、CAS rebase tests；不允许自动重试 Tool | planned |
| EX-06 | 接入 Hosted 逐项核对 | response envelope、provider reconciliation、Node actions | 按真实 output index/ID/Tool 类型核对 Agent node_ids 并写入 Node actions；不重执行、不默认绑定、不复制结果 | Hosted action 获得可靠节点归属 | Complexity: 一个 response-local reconciler；Reach: Web/Image/provider route | 0/1/N、多节点、漏绑/错绑/重复/failed fixtures；不增加 provider result store | planned |
| EX-07 | 收敛唯一反馈 | outer FunctionCallOutput、context history | 返回一次机械的 Map commit、各内部 Tool 原生结果和失败范围；删除重复 developer carrier 或 TaskSpace 结果重写 | Agent 获得忠实、无污染反馈 | Complexity: 一个 outer output formatter；Reach: context/token | pairing、failure semantics、large output 与 Standard 一致性 tests | planned |
| EX-08 | 注册生产入口并清理临时接缝 | Tool registry/provider request builder、所有 prototype/fixture | 只注册正式 Exec 路径，删除实施期 spike、未使用 helper 和候选 schema；TaskSpace 顶层仅 Exec+Hosted | 生产只有一条协议路径 | Complexity: 净删除后单入口；Reach: payload/cache | registry/payload snapshots、zero-base gate、cache source gate | planned |
| OB-01 | 建立可追踪日志 | Exec/Map/Hosted transaction trace | 记录 request revision、preflight verdict、内部 action identity、node attribution、commit revision 和失败码，不记录敏感 Tool body | 新链路可诊断且不靠猜测 | Complexity: 增加结构化事件；Reach: logs/report | fixture 逐 ID 对账、敏感字段审计 | planned |
| OB-02 | 更新缓存与性能观测 | cache regression gate、benchmark parser、performance skill fixture | 让新稳定 Tool declaration 进入敏感面，报告 Map/Exec 动作而不解析旧字段 | Prompt/schema 变化可阻断，成本数据可复算 | Complexity: 更新既有工具；Reach: CI/benchmark | policy-only cache gate、fixture report；不运行真实 Agent | planned |
| VA-01 | 完成离线集成验收 | Docker build、core/protocol/state/CLI/Viewer suites | 执行新 Map、Exec、Standard 回归、零残留和缓存门禁，逐项修复后再进入真实验证 | 先消除确定性缺陷，避免付费调试 | Complexity: 测试执行；Reach: build time | 全套指定测试通过；任一旧符号或 Standard diff 阻断 | planned |
| VA-02 | 申请并执行最小 Provider shape 验证 | disposable probe、run ledger | 另行申请 1 sample × 1 arm × repeat 1、最多 2 requests 的预算，验证最终结构化 Function schema | 在完整 benchmark 前验证 Provider/Agent 可生成合同 | Complexity: 零新生产代码；Reach: 付费 API | 首个结构失败即停；未批准不得运行 | planned |
| VA-03 | 申请并执行产品对比 | Docker benchmark、run ledger、performance report | 根据 VA-02 结果单独申请 Standard + 三种 projection policy 的 sample/arm/repeat 预算，比较正确性、路径、Map、request/token/cache/time/cost | 判断新 Map/Exec 是否产生真实收益 | Complexity: 零协议变化；Reach: 付费与耗时 | 逐 run/逐 request trace；异常先归因，不自动扩大 repeat | planned |
| VA-04 | 重排 R8 已知问题 | `01-r8-known-issues.md`、各主题计划 | 用新架构证据重新判定 I01～I10：已消失则关闭，仍存在则重写根因和依赖，新问题只在证据成立时新增 | 已暂停的问题队列不再继承旧架构假设 | Complexity: 文档状态更新；Reach: R8 后续 | 唯一问题全集与 commit/evidence 对应；用户确认后继续 | planned |

MM-10 通过前不得开始 EX-01；EX-08 和 OB-02 通过前不得申请真实运行。旧 NX-00A～NX-04 以及
`02-engineering-plan.md` 中 TX-07～TX-19 的未执行部分全部 invalidated，不改名继承；只有已完成 B0 证据继续有效。

## 4. 阶段门禁

### Phase B0：Zero-Base Reset

- Entry: 用户明确要求不保留过渡方案，从 Standard 零基础建设。
- Units: ZB-01～ZB-07。
- Exit: active code/config/prompt/test 不再包含旧 sibling/control 协议；Standard 构建、Tool wire、sequence、response 与缓存门禁成立。
- Stop: 删除触及 canonical Action Map 数据事实、Store、projection 三模式或 Standard 原生 Tool 行为时立即停下重新划界。
- Clarification: 上述 Store 指 SQLite canonical Action Map Store；ZB-06C 删除的是旧方案的对话 Event Store，不是 Map 事实源。

### Phase B1：Minimal Canonical Map Rebuild

- Entry: B0 forbidden-symbol audit 和 Standard 回归通过。
- Units: MM-00～MM-10。
- Exit: canonical Map、Store、projection、snapshot 和所有生产消费者只存在最简 Node 模型；Agent 可见 Node 同时展示
  parents/children；旧 v3、edges/ref/ledger/event-replay/detail-fold 及无消费者代码归零且没有兼容路径。
- Stop: 需要保留旧代码、增加旁路事实源、聊天重放重建 Map、迁移旧实验数据，或无法保持 Tool 结果与节点生命周期正交。

### Phase B2：Clean Map And Exec Contract

- Entry: MM-10 通过，最简 Map 零残留门禁生效。
- Units: EX-01～EX-04。
- Exit: Map operation、结构化 Exec、request-local revision 和零副作用 preflight 均有确定性证据。
- Stop: 需要独立 edge/ref/binding Tool、Agent 回显 revision、复制普通 Tool schema、兼容 adapter 或 Runtime 推断 Agent 动作。

### Phase B3：Native Tool Execution And Feedback

- Entry: EX-04 完整失败矩阵通过。
- Units: EX-05、MS-01～MS-03、EX-06～EX-08。MS 单元在 EX-05 原生 dispatch 成立后实施，不在
  EX-05 中插入整图 JSON 临时落账。
- Exit: client、Hosted、Map 和反馈走唯一生产链；Standard 路径无变化；实施期 spike 与旧候选代码归零。
- Stop: 必须修改普通 Tool 合同、复制 Provider 原始结果、引入未绑定池或重复反馈才能继续。

### Phase B4：Observability And Offline Gate

- Entry: EX-08 正式入口通过静态和单元验收。
- Units: OB-01～OB-02、VA-01。
- Exit: 新链路日志可逐动作对账，缓存/性能工具识别新合同，Docker 离线回归和零残留门禁全部通过。
- Stop: 日志需要记录敏感 Tool body、观测依赖旧字段，或缓存门禁无法区分 Standard output-ref 与禁止的 Map ref。

### Phase B5：Authorized Provider And Product Validation

- Entry: VA-01 通过；每次真实运行另有明确预算和 planned ledger。
- Units: VA-02～VA-04。
- Exit: Provider shape、产品效果和成本有逐 run 证据，R8 唯一问题全集按新架构重新排序。
- Stop: 未获预算、首个结构性失败、usage 不可信，或异常需要扩大 repeat 才能解释。

## 5. 执行记录

| Unit | Date | Evidence | Conclusion | Next |
|---|---|---|---|---|
| TX-06A | 2026-08-06 | `54fc781fc`；tools 154 passed / 1 ignored；TaskSpace Exec 33 passed；cache gate PASS | 中立 ToolSpec projection 保留；旧 TaskSpace prototype integration 不保留 | ZB-01 |
| ZB-01 | 2026-08-06 | `1143706a1`；全局约束、README、active plan 交叉引用 | 旧兼容迁移计划 invalidated；零基线计划成为唯一 active Phase B 计划 | ZB-02 |
| ZB-02 | 2026-08-06 | `2960ea03a`；`cargo check -p codex-core --lib` PASS；`codex-tools` 154 passed / 1 ignored | Phase A source-only prototype 无生产依赖并已从 active code 全部删除 | ZB-03A |
| ZB-03G | 2026-08-06 | `4472c2afa`；cache gate policy-only PASS；Standard 两请求 final-wire tests PASS | 清零期缓存门禁已删除旧 TaskSpace wire 夹具，只验证 Standard；发布保持阻断，原 NX-03 后续已由 EX-08/OB-02 取代 | ZB-03A |
| ZB-03A | 2026-08-06 | `cd327d938`；`codex-tools` 145 passed / 1 ignored；core build、ToolSpec 与 provider visibility unit tests PASS | 旧 control declaration、schema、registry handler 和 active schema fixtures 已删除；未增加替代或兼容入口 | ZB-04A |
| ZB-04A | 2026-08-06 | core build PASS；stream events 15 passed；mailbox、malformed arguments、missing client identity Standard tests 各 1 passed | Tool item 恢复为原生 future；turn 恢复 `FuturesOrdered` 落账；response completion 不再调用旧 sequence | ZB-04B |
| ZB-04B / ZB-03B / ZB-05 | 2026-08-06 | `5228efd80`；旧 sequence、control parser/handler、Provider response gate 静态为零；core build 与 Standard stream tests PASS | Tool response 已回到 Standard 原生调度与反馈路径，无兼容 adapter | ZB-06A |
| ZB-06A / ZB-06B | 2026-08-06 | core build；rooted DAG 19 tests；protocol 3 tests；schema fixtures 4 tests；Viewer 3 tests；core-skills 95 tests；skills 1 test | 旧 Prompt/Skill/fixture 已删除；Map 只保存 action-node 归属，不再保存或等待工具 reservation | ZB-06C |
| ZB-06C | 2026-08-06 | 删除约 2200 行旧 Event Store/codec/checkpoint/test；core/protocol build；历史后端目标测试 2 条；Standard rollout reconstruction 19 条；普通 Tool 错误反馈与 Hosted output 各 1 条；schema fixtures 4 条；Standard final-wire 1 条 | 所有模式只使用 Standard `ContextManager` 与 Standard rollout；模式切换不再搬运聊天历史；canonical SQLite Action Map Store 未改动；不兼容旧专用事件 | ZB-07 |
| ZB-04C / ZB-07 | 2026-08-06 | `scripts/taskspace-exec/check_zero_base.py`；门禁单测 3 条；全仓 active surface PASS；Standard final-wire PASS；cache gate PASS | 旧 control/sibling/sequence/Event Store/reservation 符号在活动表面为零；pre-commit 自动阻止回流；历史 docs 与 benchmark evidence 不误报 | MM-00 |
| MM-00 | 2026-08-06 | R8 全局约束、产品合同、活动计划与 README 交叉检查；`git diff --check`、TaskSpace zero-base gate、cache regression gate 均 PASS | 最简 Map 和旧设计净删除原则成为唯一活动设计权威；旧 NX 未执行方案只保留为失效历史证据 | MM-01 |
| MM-01 | 2026-08-07 | [`13-mm01-old-map-deletion-inventory.md`](13-mm01-old-map-deletion-inventory.md)；protocol/core/state/CLI/TUI 逐符号调用链审计；zero-base/cache gate PASS | Store CAS/线程绑定/projection policy 保留职责并重建；v3 ledger/edges/events/replay/detail-fold/Map refs 及旧 fixtures 全部绑定到 MM-02～MM-10 净删除 | MM-02 |
| MM-02～MM-09 | 2026-08-07 | `f8dc23612`；[`14-phase-b1-minimal-map-result.md`](14-phase-b1-minimal-map-result.md)；Rust/CLI/TUI/App Server/PowerShell targeted suites PASS | canonical Map、Store、projection、snapshot 和生产消费者统一为最简 Node 模型；旧 v3 与过渡 migration 净删除 | MM-10 |
| MM-10 | 2026-08-07 | `67a7e7a1b`；zero-base 6 tests + repository PASS；cache gate PASS，Standard final wire unchanged | 旧 Map 专属类型/schema/ledger/marker 回流被阻断，Standard output refs 不误报；Phase B1 离线验收完成 | EX-01 |
| EX-01 | 2026-08-07 | `0bd813e7a`；TaskSpace Exec 6 tests；Action Map 15 tests；cache gate PASS | 五项 Map 操作直接调用 canonical transaction；Agent 不填写 revision，不引入 edge/ref/binding Tool | EX-02 |
| EX-02 | 2026-08-07 | `e6887ab8f`、`671a213c8`；TaskSpace Exec 33 tests；ToolSpec capability 5 tests；code-mode 15 tests；cache gate PASS | 结构化 Exec catalog 从原生 ToolSpec 确定性派生；`tool_search` 复用原生参数合同，code-mode 保持原有过滤；Hosted 仅声明归属 | EX-03 |
| EX-03 | 2026-08-07 | `a513acfd2`；TaskSpace Exec 19 tests；cache gate PASS | revision、catalog snapshot 和内部调用身份由请求级 envelope 机械维护，不进入 Agent 参数 | EX-04 |
| EX-04 | 2026-08-07 | `2440a1446`、产品复核 `4a155c12b`；TaskSpace Exec 36 tests、Action Map 17 tests | 整批结构、Map、节点、参数、单 Patch 与 Hosted 归属在副作用前机械判定；仅 Work 承载 action，新节点状态按完整候选 DAG 推导，`read_map` 独立返回完整视图 | EX-05 |
| EX-05 | 2026-08-07 | `3b578b08d`；[`16-phase-b3-ex05-native-dispatch-result.md`](16-phase-b3-ex05-native-dispatch-result.md)；TaskSpace Exec 39 tests、Tool Router 7 tests、Action Map 17 tests；跨 crate check、zero-base/cache gate PASS | 整批 client calls 在执行前全部还原为原生调用；执行复用 Standard Router、alias/MCP/Tool Search 解析及并行策略，结果按完成顺序返回；未接 Store、Hosted、最终反馈或生产入口 | MS-01 |
| MS-01～MS-02 | 2026-08-07 | [`17-phase-b3-relational-store-result.md`](17-phase-b3-relational-store-result.md)；state 127 tests、core Store 8 tests | canonical Map 由关系表直接持久化并重新组装；Map revision CAS 保留，单 Action 变化不重写 Node/parent；无整图 JSON、Event Store、双写或旧数据兼容 | MS-03 |

## 6. 证据校准

| Date / Evidence | New Fact | Prior Conclusion | Validity Change | Downstream Change | Plan Validity / Next |
|---|---|---|---|---|---|
| 2026-08-06 / `protocol/src/taskspace.rs` 静态检查 | `taskspace-canonical-map-v3` 仍在 Map 顶层分别保存 `action_records`、`result_refs`、`evidence_refs`、`completion_records`、`block_records`；action 再用 `node_id` 反向关联节点 | “B0 后 canonical Action Map 可原样作为 NX-01 的事实底座” | 当时 qualified；已被下一条最简 Map 决策进一步 invalidated | 原 NX-00A～NX-00G 不再执行，后续以 MM-00～MM-10 为准 | invalidated / 读取下一条 |
| 2026-08-06 / 用户确认最简 Map | Node 必须直接展示 goal/state/content/parents/children/actions；Agent 只声明 parents，Runtime 机械反算 children；无 edges、Map ref、语义分类模块或 handoff condition | “把 v3 ledger 搬入节点并保留 result/evidence refs” | invalidated：旧 NX-00A～NX-04 未执行部分不再代表目标模型 | 以 MM-00～MM-10 重建 Map，再执行 EX/OB/VA；旧代码失效或无消费者即删除 | valid / 先完成 MM-00 后执行 MM-01 |
| 2026-08-06 / 用户确认净删除原则 | 旧设计不得改名、残留、暂留或以 dormant/compatibility 形式保留；无生产消费者代码也必须删除 | “可先保留 replay/detail-fold 等基础，后续再判断” | invalidated：keep 必须由新模型的当前生产责任证明，未来可能需要不是理由 | MM-01 建删除清单；MM-02～MM-10 每单元同步净删除并以零残留门禁收口 | valid / MM-01 |
| 2026-08-07 / 用户确认低延迟结算与关系化 Store | 整图 JSON 覆盖不适合逐 Tool 结算；合法 Exec 应在完整预检后立即执行，每项结果完成即写回唯一 Map | “按整批 Tool 完成后统一结算以减少 revision” | invalidated：revision 增长不得以反馈延迟为代价；整图重写也不得成为临时生产方案 | EX-05 先独立证明原生 dispatch；MS-01～MS-03 随后建立关系化 Store 与逐项 outcome 结算，再继续 Hosted/反馈/注册 | valid / EX-05 |
