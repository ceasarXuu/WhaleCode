# Phase B 零基线重建计划

- Created: 2026-08-06
- Status: Active / Phase B0～B5 engineering complete / Phase B6 LS-01～LS-08 verified offline / LS-09 same-response pairing retired / PA-00～PA-07 active
- Supersedes: [`02-engineering-plan.md`](02-engineering-plan.md) 中 TX-06B 之后的兼容迁移顺序
- Completed foundation: TX-06A (`54fc781fc`)
- Paid Whale Agent run: 本阶段删除与离线建设不需要
- Product Authority: [`00-product-contract.md`](00-product-contract.md)
- Applicable Decisions: 当前产品合同全部已确认规则；本计划只安排工程实现和验证，不新增产品语义

### 执行合同

1. `00-product-contract.md` 是本专题唯一产品权威；R8 全局约束是其必须遵守的项目级边界，不构成第二套 TaskSpace Exec 产品合同。
2. 已确认产品规则只能由用户明确修改；工程证据只能调整工作单元、状态和验证顺序，不能静默改写产品规则。
3. 每个阶段结束只审计本阶段实际产生的产品决策增量，并标记为已覆盖、纯工程、临时或冲突；存在未确认的临时行为或冲突时不得进入依赖阶段。
4. 后续工作采用最小充分建设：先复用 B3 已有事件、I07 request facts、缓存门禁和测试链，不建立第二观测事实源、平行日志系统或旧协议兼容路径。

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

### 2.4 Phase B5 Codex Exec 对照边界

Phase B5 以 OpenAI Codex 主线提交 `646f7c0a91b8e327d263335da68ae8ef212895ce`（2026-08-09）为固定对照，参考的是完整
Exec 生产链，而不是只复制 Tool description：

1. effective Tool registry/exposure 是模型可见能力的事实源；
2. outer Exec 合同与内层 Tool 定义从同一份当前能力生成；
3. Function、Freeform、Namespace 和 Tool Search 均保留各自原生输入、输出与身份；
4. 内层调用重新进入 `ToolCallRuntime`/Router，继续复用权限、sandbox、hook、取消与并行策略；
5. 内层结果复用公共 `ToolOutput` 的 nested-result 转换，不把传输 envelope 当作业务结果；
6. deferred Tool、Hosted Tool、名称冲突和模型 Tool mode 都有明确边界；
7. 离线测试只证明 wire/Runtime，不能替代 DeepSeek 的真实遵循验证。

TaskSpace Exec 只复用这些职责，不照搬 JavaScript Freeform、Lark grammar、V8 isolate、cell/wait、JS identifier normalization 或
process host。Codex JS Exec 可以在同一 cell 中等待 A 的结果后动态决定 B；结构化 Function `taskspace_exec` 在执行前已经封闭，
因此同批只允许 Agent 已确认无结果依赖的动作。依赖 A 结果的 B 必须由 Agent 在下一次 Provider 推理中决定，Runtime 不补写后续动作。

重点源码：

- [effective Tool 注册与 Code Mode 入口](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/spec_plan.rs)
- [Exec 协议渲染](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/code-mode-protocol/src/description.rs)
- [ToolSpec 投影](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/tools/src/code_mode.rs)
- [嵌套调用委托](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/code_mode/delegate.rs)
- [公共 Tool 结果转换](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/tools/src/tool_output.rs)

### 2.5 Phase B5 已确认缺口

| ID | 当前事实 | 风险 | 收敛边界 |
|---|---|---|---|
| CP-G01 | Catalog 取自 `client_router.specs()`，尚未证明等同于 effective exposure | prompt、decoder 与 Runtime 能力集合可能漂移 | 先建立 exposure matrix，再复用现有 effective view；不建第二 Registry |
| CP-G02 | capability identity 包含 `output_schema`，outer declaration 却没有结果 schema | Agent 只获得输入合同，没有结构化结果合同 | 从同一 Catalog 和固定 envelope 生成；Provider 不支持时带 wire 证据停下决策 |
| CP-G03 | client result 当前返回 `ResponseInputItem` envelope | 传输结构可能替代原 Tool 结果语义 | 复用公共 `ToolOutput::code_mode_result()` 或同一既有转换点 |
| CP-G04 | Namespace public name 用单下划线展平 | 不可逆且可能冲突 | 先做原生名称 round-trip/collision 证明，再确定最小可逆表示 |
| CP-G05 | deferred MCP 与 dynamic Tool 的 Catalog 生命周期不对称 | 搜索后不可调用或首轮提前展开 schema | 跟随 Standard effective/deferred 生命周期；不能静默全展开或隐藏 |
| CP-G06 | 协议把整个 `calls[]` 写成顺序执行 | Agent 可能把普通 work 错误串行化 | 数组只表达 Map 边界、稳定身份和结果关联；业务依赖只由 DAG 表达 |
| CP-G07 | Map operation 说明只有一句摘要 | schema 自身不足以解释操作硬合同 | 从 canonical operation 定义生成自包含描述，不加入工作建议 |
| CP-G08 | 唯一协议权威门禁主要只检查 base instructions | 详细 wire 可能在其他固定层重复出现 | 检查 active provider-visible 构建链，精确 allowlist，历史文档不报警 |
| CP-G09 | 离线测试不证明 DeepSeek 稳定生成 outer Exec | 不能以 mock/decoder 测试宣布产品行为通过 | 所有离线门禁通过后，单独申请 VA-02 真实预算 |
| CP-G10 | TaskSpace + Code Mode 时可能复制已追加 JS Exec 语法的 Tool description | 内层 Tool 混入第二套调用协议 | Catalog 消费未被其他 surface 改写的原生描述；禁止字符串清洗 |
| CP-G11 | 内层调用 trace 使用 `ToolCallSource::Direct` | 观测把 synthetic call 当成模型顶层直调 | 仅增加机械 requester identity，不参与授权、排序或结果语义 |
| CP-G12 | `LocalShell` 会让 Catalog 构建失败，目标 DeepSeek surface 尚未坐实 | 某些配置可能在请求前直接失败 | 先查目标有效 Tool 面；不适用则不建设，适用时复用原生 payload |
| CP-G13 | protocol/preflight、Map schema、Hosted 分类、feedback/recovery 存在手写双点 | 后续字段或规则可能单边漂移 | 只在真实重复边界共享事实/合同测试，不为形式统一新增规则 DSL |

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
| MS-03 | 接入可靠且低延迟的 Action 结算 | Exec coordinator、Session settlement executor、canonical Store、既有 rollout | 整个 Exec 先完成一次零副作用预检；候选 Map 与 client `Pending` 归属持久化成功后立即 dispatch。每个内部 Tool 由 Session 登记的 producer 持有到窄化终态事实进入 FIFO，完整结果继续进入唯一 outer feedback 和 Standard rollout；执行器按 `Pending -> terminal outcome` 幂等短事务结算，SQLite writer busy 持续退避，下一次 TaskSpace Provider 请求和 graceful shutdown 均等待 producer/FIFO 屏障。恢复只对 rollout 已保存结果且 Map 仍为 Pending 的 Action 做机械对账 | Tool 反馈不等待 Map 写入；outer collector 取消不撤销已登记 producer；Map Store 仍是唯一当前状态，rollout 仍是唯一执行历史 | Complexity: 一个 Session producer tracker、既有 FIFO 执行器和 outcome-only Store API；不新增持久化消息队列、Event Store 或整图 mutation closure | 独立快慢工具、部分失败、outer abort、graceful shutdown、超过 5 秒 writer busy、同/异终态、错归属、恢复、持久化组合生产链；永久冲突阻断请求 | completed-qualified |
| EX-06 | 接入 Hosted 逐项核对 | response envelope、provider reconciliation、Node actions | 按真实 output index/ID/Tool 类型核对 Agent node_ids 并写入 Node actions；不重执行、不默认绑定、不复制结果 | Hosted action 获得可靠节点归属 | Complexity: 一个 response-local reconciler；Reach: Web/Image/provider route | 0/1/N、多节点、漏绑/错绑/重复/failed fixtures；不增加 provider result store | verified |
| EX-07 | 收敛唯一反馈 | outer FunctionCallOutput、context history | 返回一次机械的 Map commit、各内部 Tool 原生结果和失败范围；删除重复 developer carrier 或 TaskSpace 结果重写 | Agent 获得忠实、无污染反馈 | Complexity: 一个 outer output formatter；Reach: context/token | pairing、failure semantics、large output 与 Standard 一致性 tests | verified |
| EX-08 | 注册生产入口并清理临时接缝 | Tool registry/provider request builder、所有 prototype/fixture | 只注册正式 Exec 路径，删除实施期 spike、未使用 helper 和候选 schema；TaskSpace 顶层仅 Exec+Hosted | 生产只有一条协议路径 | Complexity: 净删除后单入口；Reach: payload/cache | registry/payload snapshots、zero-base gate、cache source gate | verified |
| OB-01A | 审计现有观测身份链 | B3 tracing events、I07 canonical request facts、Provider boundary facts | 逐事件列出 request/response/outer call/action/node/revision 身份及权威来源，区分已有字段与无法机械关联的缺口 | 后续只修真实缺口，不重复建设 B3 已有日志 | Complexity: 只读盘点和短证据矩阵；Reach: 决定 OB-01B 最小改动面 | 每个生产事件有唯一事实来源和关联路径；发现需要第二事实源时停止 | verified |
| OB-01B | 补齐最小跨层关联 | 现有 Exec/Map/Hosted/settlement tracing 点及其 fixture | 只为 OB-01A 证明无法关联的事件补充 Runtime 已持有的机械身份和稳定失败码；不记录 Tool body，不建立聚合数据库或平行事件流 | 同一请求可沿 response、outer call、action、node 和 revision 复算执行链 | Complexity: 修改现有事件字段和 fixture，不新增日志框架；Reach: tracing consumers | 成功、拒绝、结算失败 fixture 逐 ID 对账；敏感字段审计；无重复事实 | verified |
| OB-02A | 接入新合同缓存敏感面 | cache surface contract、TaskSpace request declaration/prompt/provider payload builder | 审计最终 Exec declaration 的真实构建入口，只把尚未覆盖且能改变 Provider 稳定前缀的生产路径纳入现有门禁 | 未知 schema/prompt/payload 变化可在付费运行前被发现 | Complexity: 扩展既有规则，不新增门禁；Reach: staged commit 与 CI | 正反 fixture、policy-only gate、Standard 独立比较；普通执行代码变化不得误报 | verified |
| OB-02B | 迁移性能观察消费 | I07 request facts、TaskSpace benchmark parser、performance skill fixture | 使用 canonical request facts 统计请求/token/cache/time/cost，使用 OB-01 事件统计 Exec/Map/action；删除 active 报告路径对旧 control/sibling 字段的依赖 | 新协议成本和动作路径可复算，旧协议解析器不再污染结论 | Complexity: 更新既有消费端，不改 Runtime；Reach: benchmark/report fixtures | 合成 trace 逐 ID 对账、缺失身份判不可比较、旧字段不参与 active 结果；不运行真实 Agent | verified |
| VA-01 | 完成固定离线验收 | Docker build、TaskSpace Exec、Map/Store/settlement、Standard final-wire、CLI/Viewer、zero-base/cache gates | 按冻结清单执行已有测试和构建，不在该单元临时扩建测试体系；失败回到对应实现单元修复 | 在付费验证前确认生产链、消费者和 Standard 基线一致 | Complexity: 主要是测试执行和证据汇总；Reach: build time，无 Provider 请求 | 冻结清单全部 PASS，证据记录命令、结果和 commit；旧符号、Standard diff 或不可复算观测均阻断 | verified |
| VA-04A | 离线重映射 R8 问题 | `01-r8-known-issues.md`、当前源码与确定性测试 | 识别 I01～I10 中已被新架构删除的旧根因、仍可静态复现的缺陷和必须等待真实 trace 的行为问题 | 不把已淘汰架构假设带入付费验证，也不靠静态证据误关行为问题 | Complexity: 文档证据重排；Reach: 决定 B5 观察重点 | 每项标记为静态关闭候选、仍成立或待运行验证；只在确定性证据充分时关闭 | verified |
| ID-01 | 收敛 I10 唯一能力身份 | Exec catalog、Router、request metadata、provider/Exec trace、performance observer | 从同一最终声明序列机械派生 Runtime-only identity，沿既有请求和报告链传播；不进入 Agent schema、Provider payload、Map 或普通 Tool | 能力变化可与任务行为、缓存和成本变化分开归因 | Complexity: 一个只读身份字段和既有事件可选字段；Reach: TaskSpace Router/trace/report，Standard 为 null | 语义变化敏感性、HTTP/WS、dispatch、报告冲突 fixture；zero-base/cache gate | verified |
| VA-02 | 申请并执行最终生产路径 Provider shape 验证 | Docker 中当前 Whale 二进制、最终 Structured TaskSpace Exec declaration、run ledger | 只验证当前 Structured 生产合同；Source A/B 已结束且实现删除 | 确认目标模型能稳定生成初始化、父子节点交接、修改、验证和结束的合法 Structured batch | Complexity: 测量，不增加协议分支；Reach: 真实 Provider 成本 | Source active symbol 为零；父子节点 handoff、observer 和 Responses base identity 离线通过；真实复验必须新预算 | offline repair complete / provider revalidation pending |
| VA-02R | 收敛 outer Exec 操作合同 | `taskspace_exec/protocol.rs`、catalog declaration、合同测试 | 把外层调用方式、序列、归属和最小示例集中在 outer Tool description；补清父节点完成后子节点 readiness 机械派生的交接方式，不改状态机、普通 Tool schema 或 Runtime 行为 | Agent 从唯一 Tool 声明获得结构语法和正确交接方法，不再重复声明派生状态 | Complexity: 一个协议渲染模块和 canonical handoff；Reach: TaskSpace Tool declaration 与缓存指纹 | 初始化、读取、交接、结束示例均反向通过同一 decoder/preflight；真实遵循只由新预算复验 | verified offline / provider revalidation pending |
| SR-01 | 建立单闭合符号自愈原语 | `taskspace_exec` plan decoder、当前 request Catalog、确定性 fixtures | 原始参数严格解析失败后，只在 parser 首个失败位置的有界候选与 EOF 尝试插入一个 `}`/`]`；仅一个候选通过严格 JSON 与当前 Catalog plan decode 时返回修正版 | 明确可修复的序列化闭合错误不再浪费一次 Agent 请求 | Complexity: 一个纯函数和有界候选集；Reach: 仅 `taskspace_exec.arguments` | 真实历史坏例、缺 `}`、缺 `]`、歧义、多错误、合法输入、超界输入；不得修逗号/引号/字段/值/动作 | verified offline |
| SR-02 | 在正式上下文入口替换自愈结果 | `session/turn` 的 `OutputItemDone`、TaskSpace Router/Catalog、conversation/rollout/raw-item 路径 | 在 response scope、history、rollout 和 dispatch 之前替换同一个 FunctionCall；保留 call identity；Provider raw wire 仅作诊断，并记录无参数正文的 repair event | 后续请求、恢复和回放只看到修正版，不持续强化原错误 | Complexity: 一个 pre-record response hook；Reach: TaskSpace FunctionCall 落账边界 | scope/history/rollout/dispatch 字节一致；原错误串不进入正式历史；WebSocket 增量基线不一致时退回完整请求；Standard 零差异 | verified offline |
| SR-03 | 关闭自愈边界回归 | response/session/rollout tests、TaskSpace Exec tests、zero-base/cache gates | 串联唯一修复、正常 preflight、拒绝、取消和恢复；证明修复不跳过任何硬约束，也不改变 Provider 固定请求前缀 | 自愈是可审计的机械规范化，不演变成语义容错层 | Complexity: 定向 fixture 与既有门禁；Reach: 无真实 Provider 请求 | 修复后仍可触发 waiting/多 Patch/DAG 拒绝；合法输入零改写；模糊输入原样拒绝；缓存敏感面门禁 PASS | verified offline |
| SR-04 | 修正非 ASCII 参数的 parser 坐标 | `self_heal.rs`、真实中文 Map content 形状 | 按 `serde_json` 的 UTF-8 字节列号定位首个语法错误，不再把 byte column 当 Unicode 字符序号；候选、唯一性和 Catalog decode 规则均不变 | 中文目标、节点内容或 Tool 参数不会让单闭合符号自愈窗口偏离真实错误位置 | Complexity: 一处坐标换算；Reach: 仅自愈候选定位 | 中文内容后的缺 `}` fixture 必须被修复；ASCII、多个缺符号、非法 plan 和合法输入保持原结果 | verified offline |
| FF-01 | 去除 syntax reject 的错误 wrapper 注入 | plan decode typed error、`render_envelope_rejection`、handler tests | 只有合法 JSON 顶层实际出现 `arguments` 字段时才返回 direct-`calls`/no-wrapper 事实；纯 JSON syntax 错误只返回 parser 事实和整批零执行 | Agent 不再被与本次错误无关的恢复提示带向新的错误结构 | Complexity: 一个 typed decode branch；Reach: TaskSpace Exec 拒绝反馈 | syntax、unexpected arguments、其他 envelope 三类互斥；不建议下一步、不改写输入、不执行副作用 | verified offline |
| WF-01 | 忠实表达 waiting 节点拒绝 | `preflight.rs`、typed rejection formatter、TaskSpace feedback tests | `ClientNodeNotExecutable` 携带并输出当前状态、未完成直接父节点和整批零执行范围；删除 `Debug` 枚举直出，不加入下一步动作建议 | Agent 能看到为什么目标节点尚不可执行，不需要猜 Map 的缺口 | Complexity: 扩展一个机械错误事实；Reach: TaskSpace preflight feedback | chain/fork/join 的未完成父节点精确；状态和父节点来自 candidate Map；Runtime 不自动完成、选点或重试 | verified offline |
| OB-03 | 修复 Exec 内嵌 patch 生命周期观测 | `patch-observability.ps1`、canonical Exec action decoder、observer fixtures | Patch 专项消费当前 `calls[].client` 事实与 outer result，不再只识别顶层 Tool 和旧 `taskspace_control`；声明、preflight reject、执行和结果分别计数 | I07 不再把已声明/已执行 patch 报为零 | Complexity: 复用当前 Exec 解码事实，不增加第二 Runtime 事件源；Reach: benchmark/report only | 最新 artifact 离线复算为 2 次声明、1 次 preflight reject、1 次执行；非法 JSON 标记不可解析而非静默计零；Standard 结果不变 | verified offline |
| CP-01 | 坐实有效能力与延迟暴露事实 | Registry plan、Router、deferred Tool Search、目标 DeepSeek Tool config | 参数化列出 Standard 与 TaskSpace 中 enabled/deferred/hidden/hosted/client/Code Mode/LocalShell 的 effective surface，不改生产行为 | 后续 Catalog 只基于已证明的当前事实收敛 | Complexity: 静态追踪与离线 fixture；Reach: 解锁 CP-04/05 | 集合逐项可解释；若需要第二 Registry 或产品取舍立即停 | verified |
| CP-02 | 坐实原生 Tool identity | ToolName、Function/Freeform/Namespace fixtures | 对所有当前合法名称做 encode/decode/collision 测试，确定 Namespace 最小可逆表示 | Agent-visible identity 可无歧义恢复到原生 ToolName | Complexity: tests first；Reach: schema/decoder/cache | 未证明前不改 wire；不得沿用 JS normalization 假设 | verified；wire confirmed |
| CP-03 | 坐实公共结果转换覆盖 | `ToolOutput`、Function/Freeform/MCP/Tool Search 输出 fixtures | 对照 Standard/Code Mode 验证 `code_mode_result()` 覆盖当前内层 Tool 结果与错误 | TaskSpace 无需私有反馈语义转换层 | Complexity: tests first；Reach: 解锁 CP-08 | 公共转换有缺口时只修公共边界，不建 TaskSpace converter | verified |
| CP-04 | 让 Catalog 消费 effective capability view | Registry projection、`router.rs`、`catalog.rs` | 从 CP-01 证明的既有有效视图生成 request-local immutable Catalog，并同时驱动 declaration、decoder、identity 和 dispatch lookup | 模型可见、可解码、可执行能力一致 | Complexity: 中性抽取既有事实；Reach: TaskSpace schema/cache | exposure matrix、Standard zero-diff；不得增加第二注册系统 | verified |
| CP-05 | 闭合 deferred 与 surface-specific description | Catalog builder、Code Mode augmentation、LocalShell seam | 按 Standard 生命周期暴露 deferred 能力；TaskSpace 读取未被 Code Mode 改写的原生描述；LocalShell 仅在 CP-01 证明适用时接原生 payload | 首轮不提前展开动态 schema，内层只有一套调用语法 | Complexity: 小范围 projection；Reach: MCP/apps/shell | deferred lifecycle 与组合 declaration tests；需要字符串清洗或 fallback 即停 | verified offline |
| CP-06 | 修正 Tool identity 投影 | capability projection、decoder | 用 CP-02 已证明的表示替换单下划线展平，不保留旧 alias | public identity 与原生 dispatch identity 一一对应 | Complexity: TaskSpace wire 破坏性替换；Reach: fixtures/cache | round-trip、collision、final-wire；无兼容 reader | verified offline |
| CP-07 | 完整化模型可见输入合同 | `protocol.rs`、Map operation capabilities、canonical examples | 明确 `calls[]` 不表达普通 work 依赖；从 canonical operation/Catalog 生成各变体硬合同和可反解最小示例 | Agent 获得完整但无语义注入的 TaskSpace Exec 使用合同 | Complexity: Tool declaration；Reach: input token/cache | 示例通过正式 decoder/preflight；不得复制到 base/developer context | verified offline |
| CP-08 | 生成同源 outer 结果合同 | Catalog、outer output structs/schema | 从固定结果 envelope、canonical Map view 和 capability output schema 生成 typed outer result schema | 输入、输出和 capability identity 不再各自漂移 | Complexity: schema 构造；Reach: Provider payload/cache | schema round-trip、per-tool output diff；Provider 不支持时带 wire 证据停下 | verified offline |
| CP-09 | 忠实返回原生 nested result | dispatch、outer feedback、settlement recovery | 延后 transport conversion；在真实消费点建立中性公共 nested result，保留 CP-03 坐实的 MCP/Tool Search 结构、Patch 文本、Standard output reference 和非致命错误文本；用同一 typed result 支撑反馈与恢复 | Agent 看到原 Tool 成功、失败、结构化结果和大输出语义 | Complexity: 局部返回边界；Reach: feedback/rollout | Function/Freeform/MCP/Tool Search/output-ref/error parity；Standard 零差异；不得新增 TaskSpace 私有 converter | verified offline |
| CP-10 | 修正内层调用观测归属 | ToolCallSource、Exec dispatch trace、observer fixtures | 标记 outer call/call index/node 的机械 TaskSpace requester identity | trace 可区分模型顶层调用与 Exec 内部调用 | Complexity: 一个观测身份；Reach: rollout/report | Direct/CodeMode/TaskSpace 对账；身份不得参与授权、执行或状态 | verified offline |
| CP-11 | 收敛 Hosted 分类与原生 ToolSpec 核对 | Catalog、response scope、Hosted reconciler | 身份逐字取自原生 ToolSpec，不定义别名；同一 ToolSpec 的内部 item 聚合为一个 action，覆盖多节点、失败、漏绑和错绑 | Provider 已执行的原生 ToolSpec 与 Agent 归属声明机械一致，内部过程不进入 TaskSpace | Complexity: 删除逐 item 身份与顺序处理；Reach: Web/Image | ToolSpec identity/capability-set fault matrix；不得重执行、猜配、默认 Root 或改变节点状态 | verified offline |
| CP-12 | 加固单一协议与 final-wire 门禁 | active provider-visible context builders、cache regression、contract tests | 检查所有活动固定层只由 outer declaration 承载详细协议；覆盖 deferred/namespace/output/surface 变化 | 协议漂移和缓存敏感变更在付费运行前被发现 | Complexity: 精确 allowlist；Reach: commit/CI | 正反 fixture、Standard exact diff、cache gate；普通词汇与历史 docs 不误报 | verified offline |
| CP-13 | 完成 Codex parity 离线总验收 | TaskSpace Exec、Router、ToolOutput、Hosted、Map、workspace/gates | 汇总 CP-01～CP-12 的最小相关测试，再运行冻结的 workspace、zero-base 和 cache gates | 生产链完整后才重新打开真实 Provider 验证 | Complexity: tests only；Reach: build time | 任一缺口或未确认产品选择阻断 VA-02；不运行模型 | verified offline |
| LS-00 | 冻结闭集顺序产品合同 | `00-product-contract.md`、`43-closed-legal-sequence-design.md` | 审计七个核心场景的 trace/确定性证据，确认 Ready 工作、纯 Map update、统一 Provider Tool 动作和 blocked 去留；写入 PD1～PD7 | 实现不会暗中决定 Agent 可用动作、删减 Map 能力或为空想场景扩张 schema | Complexity: 文档决策；Reach: 阻断全部 LS 实现 | 七场景逐项有 E1 或 E2+E3；Product Decision Delta covered | verified / user confirmed |
| LS-01 | 删除无证据的 blocked 生命周期 | NodeState、transaction、Store、projection、CLI/Viewer、feedback | 目标状态收敛为 Waiting/Ready/InFlight/Completed；删除 blocked 字段、转移、规则、序列化和消费者，不做兼容或 migration | 减少无收益状态和行动限制，不改变 DAG、节点完成权或外部事实透传 | Complexity: 跨面机械净删除，预计单元少于 140 行生产净改动；Reach: canonical Map wire | forbidden-symbol audit；四状态转移、hydrate/projection、Standard 0-diff；外部阻碍可保存在 content | verified offline |
| LS-02 | 建立统一 Tool action catalog | `catalog.rs`、Tool action typed structs | 从同一 ToolSpec/Hosted Catalog 生成一份统一 `tools[]` action union；client 与 Provider 位于同一 Agent-visible 槽位，归属 metadata 不进入原生 input | Tool 工作模型只有一份，Provider 不成为闭集外例外 | Complexity: 中性 catalog 投影，预计少于 160 行生产改动；Reach: declaration/cache | client/Freeform/Namespace/Provider schema；catalog 只展开一次；Standard final wire 0-diff | verified offline |
| LS-03 | 替换 Agent-visible 顺序 schema | `catalog.rs`、typed sequence structs | 用 exact discriminator + disjoint `anyOf` 表达 L1～L8；每个分支自包含适用条件；Map operation 复用 canonical input，含 Tool 的分支引用 LS-02 catalog；删除 Agent-visible `calls[]` | Agent 只能选择有证据的 Map/Tool 顺序，并能从所选分支直接判断当前 Map 是否适用；不按场景复制 Map 或 Tool 协议 | Complexity: 一个判别联合，预计少于 180 行生产改动；Reach: TaskSpace declaration/cache | L1～L8 正反 schema、八分支适用条件、deterministic bytes、分模块 wire bytes、generic escape reject | verified offline |
| LS-04 | 建立单向机械归一化 | `plan.rs`、`envelope.rs` | typed sequence 解析为 `NormalizedExecPlan(pre_map, tools, terminal_map)`；直接删除 RawPlan/calls decoder，不创造高层 completion/handoff 命令 | Runtime 复用 Map transaction、Router 和 Hosted reconciler，不维护第二执行系统 | Complexity: 替换 parser并净删除旧 decoder，预计少于 150 行生产净改动；Reach: envelope/internal IDs | L1～L8 normalization snapshot；unknown type/field拒绝；无旧 wire fallback | verified offline |
| LS-05 | 按顺序阶段重写 preflight | `preflight.rs`、Map operation adapter | 固定 pre-map -> Tool admission -> optional terminal-map；允许纯 update；Ready 上的 Agent Tool 声明机械转 InFlight；完整候选 DAG、单 Patch和 revision规则保持 | handoff/join/fork 在副作用前准确判定，Runtime不要求额外 in-flight 动作，也不自动 completion | Complexity: 替换任意边界扫描，预计少于 180 行生产净改动；Reach: candidate Map | L1～L8动态矩阵；ready/inflight/waiting/completed；单/多父、独立 Tool、stale、finish | verified offline |
| LS-06 | 接入 Provider Tool 执行适配 | response scope、Hosted reconciler、internal response kind | Provider 内部 items 按原生 ToolSpec 聚合，与同响应 Agent 声明核对、绑定和记录，不重执行、不创建名称；client 仍走原 Router | Agent 看见原生 Tool 身份，Runtime保留不可回滚事实和漏绑错绑硬门，但不暴露 Provider 内部过程 | Complexity: ToolSpec-set 对账；Reach: Provider/client settlement | provider-only/mixed/missing/duplicate/multi-node/failed；内部 item 不进入 Map/反馈 | verified offline |
| LS-07 | 替换唯一模型合同、反馈和 observer | `protocol.rs`、canonical examples、typed rejection、observer decoder | description只解释闭集顺序和 Tool 无序语义；示例反向通过正式 decoder/preflight；反馈和性能观察读取同一新合同 | Agent、Runtime和报告不再使用旧字段或平行解释 | Complexity: 删除旧 calls/hosted术语并更新消费者；Reach: Tool wire/feedback/report | 协议唯一性、反馈无建议、observer self-test、旧 active terminology audit | verified offline |
| LS-08 | 清除旧模型并完成离线门禁 | 旧 parser/preflight/tests/docs、zero-base/cache gate、TaskSpace suites | 删除 `calls[]`、`hosted_work/hosted_bindings`、blocked、任意 interleaving 测试和无消费者 helper；运行 focused、workspace、Standard exact wire 和缓存门禁 | active code只有闭集顺序路径，无兼容、双轨或失效奖励 | Complexity: 预期净删除；Reach: build与缓存基线 | forbidden symbols、focused suites、workspace check；缓存门禁阻断时按全局规则申请 | verified offline |
| LS-09 | 最小 Provider 行为复验 | Docker benchmark、run ledger、逐 request trace | 离线通过且单独获批后，用简单 handoff sample 验证闭集选择、Ready启动、Provider统一动作、请求/token/cache/time | 判断闭集是否减少协议试错，不以离线结构正确替代模型行为证据 | Complexity: 测量，无代码；Reach: 真实 API成本 | 已批准三项顺序 repeat=1；首个异常停下归因；不得沿用旧预算余额 | ready / authorized |
| VA-03 | 申请并执行首轮四臂产品测量 | Docker benchmark、run ledger、performance report | VA-02 通过后单独申请 Standard、map-always、map-append、map-request 的同版本同样本预算，比较业务结果、动作路径、Map、request/token/cache/time/cost | 获得新协议的首轮产品事实，而不是用旧 benchmark 推断收益 | Complexity: 零协议变化；Reach: 付费与耗时 | 逐 run/逐 request trace；首轮仅测量，不自动作发布判断、不自动扩大 repeat；异常先归因 | planned |
| VA-04B | 最终重排 R8 已知问题 | `01-r8-known-issues.md`、VA-02/03 trace、各专题计划 | 将 VA-04A 候选与真实证据合并：已消失则关闭，仍存在则重写根因和依赖，新问题只在独立证据成立时新增 | R8 恢复到基于新架构证据的唯一问题队列 | Complexity: 文档状态变化；Reach: 决定 R8 后续顺序 | 每项有当前 commit 和证据路径；成本阈值未获用户确认时只报告测量结果，不自行判定发布 | planned |

MM-10 通过前不得开始 EX-01；EX-08、OB-01B、OB-02A、OB-02B、VA-01 通过前不得申请真实运行。旧 NX-00A～NX-04 以及
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
- Units: OB-01A～OB-01B、OB-02A～OB-02B、VA-01、VA-04A。
- Exit: 新链路日志可逐动作对账，缓存/性能工具识别新合同，Docker 离线回归和零残留门禁全部通过；旧问题已完成离线重映射。
- Stop: 日志需要记录敏感 Tool body、观测依赖旧字段，或缓存门禁无法区分 Standard output-ref 与禁止的 Map ref。

### Phase B5：Codex Parity Completion And Authorized Product Validation

- Entry: VA-01 通过；每次真实运行另有明确预算和 planned ledger。
- Units: ID-01、VA-02（历史生产证据）、VA-02R、CP-01～CP-13、SR-01～SR-03、WF-01、OB-03、VA-02（再次验证）、VA-03、VA-04B。
- Order: 当前先按 `SR-01 -> SR-02 -> SR-03 -> WF-01 -> OB-03` 完成离线修复；全部通过后才可重新申请 VA-02。VA-02 通过前不得启动 VA-03。
- Exit: effective capability、输入/输出合同、原生 dispatch/result、Hosted 对账、观测和门禁形成同一生产闭环；随后 Provider shape、
  产品效果和成本有逐 run 证据，R8 唯一问题全集按新架构重新排序。
- Stop: 需要第二 Registry、TaskSpace 私有结果语义、字符串清洗 Tool description、Runtime 补写 Agent 动作、未确认 Provider/deferred/
  LocalShell 产品取舍、未获预算、首个结构性失败、usage 不可信，或异常需要扩大 repeat 才能解释。

### Phase B6：Closed Legal Sequence Model

- Entry: PD1～PD7 已写入唯一产品合同；现有 `calls[]` trace 只作为场景证据，不再作为目标 wire。
- Units: LS-00 -> LS-01 -> LS-02 -> LS-03 -> LS-04 -> LS-05 -> LS-06 -> LS-07 -> LS-08 -> LS-09。
- Exit: Agent-visible schema 只包含 L1～L8 闭集顺序和统一 Tool action；旧 calls/Hosted平行通道/blocked规则和测试归零；Standard
  无变化；当前 Tool/Map/Provider生产链全部复用；最小获批 trace 证明 Agent能直接选择合法 handoff，而非依赖 waiting reject学习。
- Stop: 任一顺序需要 generic escape、Runtime语义推断、普通 Tool原生合同修改、Provider能力静默缩减、单个实施单元新增超过
  500 行手写生产代码、缓存门禁未获授权，或出现与 PD1～PD7 冲突的产品行为。
- Downstream: 旧 VA-02/VA-03 运行只保留历史证据；LS-09 通过后重新核定四臂测量计划和预算，不沿用旧协议结论。

#### 历史预授权真实运行预算（已失效）

用户于 2026-08-10 将两项预算各批准为原申请的两倍，额外额度只用于取得明确失败证据后的针对性修复复验，不允许盲目重试：

| Gate | Scope | Hard Budget | Retry Boundary | Activation |
|---|---|---|---|---|
| VA-02 | `deepseek-v4-flash`；`single-file-fast-fix`；`map-request`；每次 repeat 1 | 最多 2 次顺序 sample run、4 requests、100K input、24K output、USD 0.04、24 分钟 | 首次失败后必须先完成证据归因和离线修复；最多使用第二次，任一结构/业务/usage/证据异常即停 | CP-13 全部通过后登记 planned ledger |
| VA-03 | `deepseek-v4-flash`；同一 `single-file-fast-fix`；Standard、map-always、map-append、map-request；每臂每轮 repeat 1 | 最多 2 轮、8 sample runs、120 requests、2M input、120K output、USD 0.36、60 分钟 | 首轮失败后必须先完成证据归因和针对性修复；不增加 arm/sample/repeat，单臂业务失败不自动重试 | VA-02 成功后登记 planned ledger |

上述预算只适用于已废弃的 `calls[]` 协议，未使用余额不迁移到 Phase B6，也不得作为 LS-09 或后续四臂测量授权。

#### LS-09 真实运行验收预算（已批准，待激活）

- Authorization: 用户于 2026-08-12 明确批准 `R8-LS09-LIVE-ACCEPT-20260812`。
- Activation: LS-01～LS-08 全部完成；focused/workspace/zero-base/Standard exact-wire/cache gate 全部通过；最终提交已推送。
- Model: `deepseek-v4-flash`。
- Projection: `map-request`。
- Retry: 0；任何复验必须先完成根因分析和离线修复，再申请新预算。

| Run | Sample | Repeat | 验收重点 |
|---|---|---:|---|
| A | `single-file-fast-fix` | 1 | 初始化、连续 Tool、handoff、Ready -> InFlight、finish |
| B | `subscription-billing-repair` | 1 | 复杂 DAG、fork/join、多节点推进、Map 调整和闭合 |
| C | 最小 Provider-hosted probe | 1 | Provider Tool 位于统一 `tools[]`、节点绑定、顶层事实对账且不重复执行 |

硬上限：3 sample runs、52 Provider requests、1,180,000 input tokens、80,000 output tokens、60 分钟、
USD 0.19 / CNY 1.35。费用采用运行前冻结的 DeepSeek 官方价格结算；任一硬上限先到即停止。

运行必须顺序执行。每个 run 启动前在 `benchmarks/whale-agent-run-ledger.json` 创建独立 `planned` 记录，结束、失败或取消后
立即结算；不得先创建三条伪记录，也不得用包装脚本自动重试。出现顶层 client 逃逸、Provider 重执行/漏绑/错绑被接受、
Map/DAG 损坏、同一拒绝重复、业务失败伴随协议/反馈/状态异常、usage/trace 不可信或同形暖请求零缓存命中时，立即停止
剩余 run。纯 Agent 业务判断错误且 Runtime 与反馈正确时只记录行为事实，不自动修复或重跑。

通过条件：三个样本均通过业务验证和隐藏 oracle；Agent 只使用 L1～L8；全部 Tool 有合法归属；Provider Tool 只执行一次；
Ready -> InFlight 只由 Agent 声明的 Tool action 触发；Runtime 不选节点、不补动作、不自动 completion；反馈无重复、丢失或
语义改写；目标 wire 不含旧 `calls[]`、独立 Hosted binding 或 `blocked`。request 2+ 缓存低于 90% 不自动判业务失败，
但必须暂停后续产品测量并完成前缀归因。

预算批准不绕过缓存门禁、账本和前置门禁；LS-09 激活前不创建 `planned` 记录，未使用余额不转移到 VA-03。

#### TaskSpace Exec 输入压缩预算包

用户于 2026-08-11 批准总额 USD 1.00 的顺序优化预算包。该预算只用于 `taskspace_exec` 模型可见合同的单变量压缩：

1. 每轮只改变一个可准确命名的因素；离线合同、final-wire 和缓存敏感面门禁通过后，才允许一次
   `single-file-fast-fix × map-request × repeat=1` 真实运行。
2. 每轮先判断业务结果、结构错误、请求数、input/cached/uncached/output token、耗时和费用；结论明确后才保留并进入下一因素。
   失败或收益无法与行为波动区分时，回退或跳过该因素，禁止把未验证变化叠入下一轮。
3. 第一因素 `SC-01` 只移除模型可见的完整 outer result TypeScript 展开；内部 typed result、实际 JSON 反馈、能力身份、输入
   schema、Map、序列、示例和 Runtime 均保持不变。
4. 后续候选依次为 `SC-02` 协议示例去重，以及仅在不削弱硬合同前提下评估的 `SC-03` 输入 schema 表达压缩。没有明确安全
   方案的候选直接证伪跳过，不为消耗预算而运行。
5. 每次真实运行必须先登记独立 `planned` ledger，单轮无自动重试；累计估算费用达到 USD 1.00 立即停止。预算授权不允许扩大
   sample、arm 或 repeat，也不绕过缓存门禁和付费运行证据要求。

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
| MS-03 / EX-06～EX-08 | 2026-08-07 | `1347606e0`；[`18-phase-b3-execution-feedback-result.md`](18-phase-b3-execution-feedback-result.md)；TaskSpace Exec 50 tests、core 1830 passed / 3 ignored、state/API/rollout suites、workspace check、zero-base/cache gate PASS | 预检候选与 Pending 先固化、原生 Tool 逐项低延迟结算；Hosted 按真实 response index/ID 对账；唯一 outer 反馈；TaskSpace 顶层仅 Exec+Hosted，Standard 路径不变；Phase B3 离线验收完成 | OB-01 |
| MS-03 closure reopened | 2026-08-09 | `03acb2db6`、Round 3 review `df274ae97`；[`18-phase-b3-execution-feedback-result.md`](18-phase-b3-execution-feedback-result.md) | 短事务 latest-head 结算成立，但 5 秒 writer timeout、post-Tool cancellation 和 generic whole-map API 仍可破坏事实持久化或 CAS 边界；原“Phase B3 完成”结论失效 | 先确认并实施 durable、outcome-only Action 事实结算；不得进入 OB-01/B4 |
| MS-03 repair design | 2026-08-09 | 用户确认；[`18-phase-b3-execution-feedback-result.md`](18-phase-b3-execution-feedback-result.md) | 否决独立持久化消息队列：它无法与任意外部 Tool 副作用形成原子事务，且会复制 rollout 已保存的执行事实。收敛为 Session FIFO 结算执行器、outcome-only Store API、请求前屏障和 Pending-only rollout 对账 | 实施并完成故障注入回归后再关闭 MS-03 |
| MS-03 repair verified | 2026-08-09 | `e5925b45d`、`702f885a0`；[`18-phase-b3-execution-feedback-result.md`](18-phase-b3-execution-feedback-result.md)；State 133、TaskSpace Exec 56、settlement 4、output-reference 11 tests；workspace check、zero-base/cache gate PASS | Store 权限已收窄为 outcome-only；Tool 返回后同步投递到 Session 执行器，outer cancellation 不撤销事实；writer busy 无固定 5 秒丢弃；下一请求前屏障和 Pending-only rollout 对账成立。未新增持久化队列或第二份 Tool 结果 | MS-03 与 Phase B3 关闭；进入 OB-01/B4 |
| MS-03 adversarial review reopened | 2026-08-09 | [`vs_review/2026-08-09-r8-ms03-settlement-review.md`](../../../../vs_review/2026-08-09-r8-ms03-settlement-review.md)；fresh reviewer `019fe337-6475-7042-896d-4c338c40d420` | `AbortOnDropHandle` 子 Tool 完成到父 future enqueue 之间仍可丢失结果；graceful shutdown 未等待 producer/FIFO；现有测试未覆盖 persisted handler 到 provider barrier 的组合生产链。另确认 extended SQLite code、recovery identity 和 cache 单调性缺口 | 用户确认 producer/shutdown 生命周期方向后修复；不得进入 OB-01/B4 |
| MS-03 engineering hardening | 2026-08-09 | `4be93ba31`；State 4、Session settlement 6、TaskSpace Exec 56；workspace、zero-base/cache gate PASS | extended SQLite busy、恢复 identity/冲突历史和结算日志时序已闭合；未改变 producer、shutdown 或 cache 安装语义 | B01/B02/B03 与 cache 单调性仍 open；不得进入 OB-01/B4 |
| MS-03 cache monotonicity | 2026-08-09 | `2aa968348`；cache install 5、Store 9、settlement 6、Exec 56；workspace、zero-base/cache gate PASS | 所有并发 cache 更新路径共用 revision 单调安装门禁；旧读取晚到不再覆盖新 cache，同 revision 异 hash 硬拒绝 | N04 关闭；B01/B02/B03 仍 open，不得进入 OB-01/B4 |
| MS-03 producer/shutdown closure | 2026-08-09 | `aba41ff04`；Tokio `TaskTracker` + admission gate；TaskSpace Exec 56、settlement 8 | producer 持有 Tool 到 enqueue；shutdown 先关闭并等待 producer，再 drain 既有 FIFO；永久错误不报告 `ShutdownComplete` | B01/B02 fixed，B03 待组合测试 |
| MS-03 composed production proof | 2026-08-09 | `4d7387a86`；TaskSpace Exec 57、settlement 9；workspace、zero-base/cache gate PASS | persisted handler、原生 Router、SQLite outcome、Standard rollout/output-ref、provider preparation 串成一条离线生产链；错误归属在 transport 前阻断 | B01～B03 implementation fixed；focused closure review pending，暂不进入 OB-01/B4 |
| MS-03 focused engineering closure | 2026-08-09 | `24c54333b`；[`vs_review/2026-08-09-r8-ms03-settlement-review.md`](../../../../vs_review/2026-08-09-r8-ms03-settlement-review.md)；TaskSpace Exec 57、settlement/recovery 11、workspace、zero-base/cache gate PASS | admission-before-abort、shutdown error 退出 submission loop、pending turn shutdown guard 均由 focused reviewer 复核 PASS；B03 采用 production chain + output-ref recovery + preparation failure 三层证据，不为单体 mega-test 增加生产 hook | MS-03 离线工程闭环；0 production blocker；Phase B4 不自动启动 |
| B3 后续计划复审 | 2026-08-09 | B3 生产事件、I07 canonical request facts、缓存门禁和当前 benchmark 调用面静态审计 | B4 从新建观测层收敛为关联审计与最小补缺；缓存和性能消费拆分；离线问题重映射前置；B5 禁止复用旧 A2 source-only probe | OB-01A |
| OB-01A | 2026-08-09 | [`19-phase-b4-observability-audit.md`](19-phase-b4-observability-audit.md)；Provider/response/outer/action/node/revision 静态调用链 | B3 事件充分但身份传递不完整；成本继续以 I07 request facts 为唯一事实，TaskSpace 只补 response-local 关联，不新建观测层 | OB-01B |
| OB-01B | 2026-08-09 | [`19-phase-b4-observability-audit.md`](19-phase-b4-observability-audit.md)；TaskSpace Exec 57 tests | response-local scope 复用 Provider response/wire identity；既有事件补齐 outer/action/node/revision 关联与稳定失败码；无新事实库、Agent-visible 字段或 Standard 改动 | OB-02A |
| OB-02A | 2026-08-09 | [`20-phase-b4-cache-surface-result.md`](20-phase-b4-cache-surface-result.md)；cache surface 9 tests；gate 30 tests | 门禁从整个 Tool Runtime 目录收敛到真实 schema/可见性/Exec declaration 构建链；声明变化必报，普通 handler/dispatch/preflight/tracing 变化不误报 | OB-02B |
| OB-02B | 2026-08-09 | [`21-phase-b4-performance-observer-result.md`](21-phase-b4-performance-observer-result.md)；Exec observer、performance report、request facts consumer 与 Docker call graph tests PASS | R8 以 outer Exec + 内部 map/client/hosted/node/result 复算动作；I07 request facts 仍是请求/usage 权威；旧 control/sibling 仅服务历史 artifact | VA-01 |
| VA-01 | 2026-08-09 | `3601dcf0b`；[`22-phase-b4-offline-acceptance.md`](22-phase-b4-offline-acceptance.md)；Core 1856/3、Exec 57、settlement 11、State 134、CLI 5、Viewer 4、Protocol 183；Docker/workspace/zero-base/cache PASS | 固定离线生产链全部通过；Viewer 陈旧 fixture 已按真实配置源和既有大栈方式修复；虚拟 key 复验不触发 Provider | VA-04A |
| VA-04A | 2026-08-09 | [`23-phase-b4-issue-remap-result.md`](23-phase-b4-issue-remap-result.md)；[`../01-r8-known-issues.md`](../01-r8-known-issues.md)；当前生产符号、Router/Exec/DAG 测试及 OB-01/OB-02 证据 | I01/I02/I05/I06 为静态关闭候选；I10 是当前静态缺口；I07 待生产验收；I03/I04/I08 只由 Phase B5 行为与成本 trace 判断。B4 退出条件全部满足 | Phase B5；真实运行前另行预算 |
| ID-01 | 2026-08-09 | `8481d24bc`、`d669c62a0`；[`../I10/00-i10-capability-identity-repair-plan.md`](../I10/00-i10-capability-identity-repair-plan.md)；TaskSpace Exec 58、Core 1857/3、workspace、zero-base、observer fixture、cache gate PASS | 单一 Catalog 身份沿 dispatch/request/Provider/Exec/report 机械传播；不进入 Agent schema、Map 或 provider payload；缺失/冲突 fail closed。缓存门禁同时发现 Standard Skills 固定前缀相对旧 accepted snapshot 漂移，发布保持阻断 | VA-02；真实运行前另行预算 |
| VA-02 | 2026-08-09 | [`24-phase-b5-va02-first-result.md`](24-phase-b5-va02-first-result.md)；`WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5`；观测修复 `cca76e921` | 获批的 map-request 单样本在首个响应失败：模型把 Exec 内部 `exec_command` 提升为非法顶层 call；Runtime 在零副作用边界正确拒绝。原 v11 parser 漂移已修复并从原始 trace 恢复 1 request / 11,715 input / 108 output / USD 0.00167034 | 停止 VA-03；用户确认 Agent 可见工作协议方向后再实施并另行申请复验预算 |
| VA-02R | 2026-08-09 | [`25-phase-b5-protocol-authority-repair.md`](25-phase-b5-protocol-authority-repair.md)；最新 Codex 主线 `646f7c0a9`；TaskSpace Exec 60/60 PASS | outer Tool description 成为唯一模型可见操作合同；base instructions、普通 Tool 与 Runtime 语义均未改变；首次示例由同一 catalog 生成并通过 decoder/preflight | 运行缓存门禁；通过后提交。真实 VA-02 复验必须另行申请预算，未复验前不启动 VA-03 |
| Phase B5 Codex parity replanning | 2026-08-09 | OpenAI Codex `646f7c0a91b8e327d263335da68ae8ef212895ce` 的 registry、protocol、ToolSpec projection、delegate、ToolOutput 与测试链；当前 Whale production call graph | VA-02R 只完成 outer description 局部修复，不能代表完整 Codex 对照闭环；CP-G01～CP-G13 已合并进本唯一活动计划，不另建决策或阶段体系 | CP-01；CP-13 通过前保持 VA-02 blocked |
| CP-01 | 2026-08-10 | [`26-phase-b5-cp01-effective-surface-result.md`](26-phase-b5-cp01-effective-surface-result.md)；DeepSeek/default/deferred/Code Mode 参数化测试 2 条；既有 Registry/Catalog 回归 14 条 | 目标使用 UnifiedExec 与 hosted web search；LocalShell 不适用；raw specs、Standard model-visible 与 TaskSpace inner effective surface 不能互相替代；deferred 和 Code Mode 差异已精确落到 CP-04/05 | CP-02；不改生产行为 |
| CP-02 evidence | 2026-08-10 | [`27-phase-b5-cp02-tool-identity-result.md`](27-phase-b5-cp02-tool-identity-result.md)；`codex-tools` collision tests 2 条；`codex-protocol` round-trip test 1 条 | 当前单下划线 alias 对 Namespace/Namespace、Namespace/plain 均会碰撞；原生二元 `ToolName` 可无损往返。用户确认只给 Namespace call variant 增加精确 `namespace` 字段，普通 Tool 结构不变 | CP-06 破坏性替换；不保留兼容协议 |
| CP-03 | 2026-08-10 | [`28-phase-b5-cp03-result-conversion.md`](28-phase-b5-cp03-result-conversion.md)；新增 Tool Search / Patch fixtures 2 条；既有 Function、Freeform、MCP、exec output-ref 证据 | `ToolOutput` 是正确公共事实边界，但 `code_mode_result()` 含 Patch 空结果和 exec 截断等 Code Mode 专属策略；Tool Search 非致命错误文本也在现有 response conversion 中丢失 | CP-09 延后 transport conversion，并在真实消费点建立中性公共 nested result；不新增 TaskSpace 私有 converter |
| CP-04 | 2026-08-10 | [`29-phase-b5-cp04-effective-capability-result.md`](29-phase-b5-cp04-effective-capability-result.md)；同一 `ConfiguredToolSpec` 的 Provider/native 视图；Code Mode/TaskSpace/Router fixtures | TaskSpace Catalog 改读唯一注册条目的原生能力合同；Standard/Code Mode Provider spec 和 Router 不变；TaskSpace 不再复制 JS Exec 调用语法 | CP-05 处理 deferred 生命周期；CP-06 处理已确认后的 Namespace wire |
| CP-06 | 2026-08-10 | [`30-phase-b5-cp06-tool-identity-result.md`](30-phase-b5-cp06-tool-identity-result.md)；TaskSpace Exec 62、Tool capability 7、ToolName round-trip | TaskSpace call 用独立 `namespace` 与 leaf `tool`；Catalog/decoder/preflight 按原生 `ToolName` 查找；旧扁平 alias 无兼容入口；Code Mode 不变 | CP-09 先建立 typed nested result，再用其闭合 CP-05 deferred 生命周期 |
| CP-09 | 2026-08-10 | [`31-phase-b5-cp09-nested-result.md`](31-phase-b5-cp09-nested-result.md)；context 23、TaskSpace Exec 62、Router 与门禁 | 公共 `ToolOutput` 在真实 TaskSpace 消费点生成中性 tagged result；Patch、output reference、MCP、Tool Search 和失败文本忠实保留；Standard/Code Mode 入口不变 | CP-05 从自然上下文中的 Tool Search result 机械恢复 deferred capability；CP-08 生成同源 output schema |
| CP-05 | 2026-08-10 | [`32-phase-b5-cp05-deferred-lifecycle.md`](32-phase-b5-cp05-deferred-lifecycle.md)；TaskSpace Exec 65、目标 DeepSeek effective surface、缓存门禁 | 首轮隐藏 deferred；后续只从已配对成功 Tool Search 自然反馈恢复 schema，并以当前精确 handler 过滤；dynamic/MCP 共用同一 Catalog，无隐藏 ledger | CP-07；真实缓存结果留到 CP-12/13 后统一复验 |
| CP-07 | 2026-08-10 | [`33-phase-b5-cp07-input-contract.md`](33-phase-b5-cp07-input-contract.md)；TaskSpace Exec 67、canonical decoder/preflight、缓存门禁 | `calls[]` 只表达 Map 边界，Work 依赖仅来自 parents；Map operation 硬合同和三类示例由 canonical 类型生成，不复制到 base/developer prompt | CP-08；Provider 遵循留到 VA-02 |
| CP-08 | 2026-08-10 | [`34-phase-b5-cp08-output-contract.md`](34-phase-b5-cp08-output-contract.md)；TaskSpace Exec 67、typed feedback/schema round-trip、Provider serialization | typed outer result 成为反馈唯一结构；同一 schema 驱动内部声明、能力身份和 outer Tool 可见返回类型。Provider 不支持原生 Function output schema，故按 Codex 做法渲染进唯一 Tool description，不新增 prompt 层 | CP-10；缓存/final-wire 统一留到 CP-12/13 |
| CP-10 | 2026-08-10 | [`35-phase-b5-cp10-dispatch-requester.md`](35-phase-b5-cp10-dispatch-requester.md)；rollout trace 38、dispatch trace 4、TaskSpace Exec 67 | 内层 client Tool 不再伪装成模型顶层直调；outer call、call index、node 和真实 nested result 可在通用 rollout trace 中机械回放。该身份不进入执行、权限、Map 或 Agent 合同 | CP-11 |
| CP-11 | 2026-08-10 | [`36-phase-b5-cp11-hosted-reconciliation.md`](36-phase-b5-cp11-hosted-reconciliation.md)；TaskSpace Exec 69 | Catalog 与 Provider response 共用 Web/Image Hosted 分类事实；真实 index、Provider ID、类型、多节点和终态逐项核对，失败不改变节点状态；无重执行、猜配或默认归属 | CP-12 |
| CP-12 | 2026-08-10 | [`37-phase-b5-cp12-final-wire-gate.md`](37-phase-b5-cp12-final-wire-gate.md)；TaskSpace final-wire 1、Exec 69、cache contract 9/30/12、free commands 8 | 详细协议仅在 outer Tool；消息上下文不重复。四个真实 declaration 源纳入精确敏感面，正式请求冻结 model/tool_choice/tools；离线 runner 隔离主机 HOME 且复用构建缓存；执行内部仍不误报，既有付费基线未被改写 | CP-13 |
| CP-13 | 2026-08-10 | [`38-phase-b5-cp13-offline-acceptance.md`](38-phase-b5-cp13-offline-acceptance.md)；Core 1873/3、State 134、CLI 5、Viewer 4、Protocol 183；workspace、zero-base、8 项免费缓存合同与 cache gate PASS | CP-01～CP-12 的生产调用链和冻结门禁整体通过；没有新增产品决策、协议分叉或真实 Provider 请求。候选缓存敏感面仍保持发布阻断，等待已批准的 VA-02 真实复验 | VA-02 |
| VA-02 second run | 2026-08-10 | [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)；`WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4` | 第二响应合法执行 `initialize_map + exec_command`；在线结算 2 provider requests / 3 local attempts，request 2+ cache hit 96.20%。首响应再次在无 Hosted output 的机械空字段附近生成非法 JSON；该字段现已改为可省略并完成离线验证 | 申请最小 VA-02 复验预算；通过前 VA-03 保持 blocked |
| VA-02 zero-Hosted revalidation | 2026-08-10 | [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)；`WAR-20260810-061241-CACHE-REGRESSION-A143B6F0` | 首响应省略 `hosted_bindings` 后合法初始化 Map 并执行 client Tool，局部修复在线成立；第二响应生成未声明的顶层 `exec_command`。两次 wire 的 Tool 集合均为 `taskspace_exec + web_search`，Runtime 未重暴露普通 Tool 并正确拒绝 | VA-03 保持 blocked；先归因 I03 当前生产表现，不自动再跑 Agent |
| VA-02 map-client wire revalidation | 2026-08-10 | [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)；`WAR-20260810-174818-CACHE-REGRESSION-0EF76553` | 目标模型连续两次正确使用 outer `taskspace_exec` 和新 `map/client` wire；初始化与两个 client Action 成功，第二请求缓存命中 94.69%。第三请求在 Provider 前被批准的两请求上限截止，故没有 patch | 不自动重试；重新申请足够覆盖最小任务闭环的 VA-02 预算，完成前 VA-03 保持 blocked |
| VA-02 end-to-end revalidation | 2026-08-10 | [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)；`WAR-20260810-180151-CACHE-REGRESSION-7E11A055`；[`coe case`](../../../../coe/2026-08-10-06-00-r8-b5-va02-evidence-closure.md) | 6 requests 完整结算；outer Exec 与缓存健康。一次 waiting 节点误选后，Agent 两次在 `update_map + apply_patch` map/client 边界少一个 `}`，没有 patch 或 Map 副作用 | 停止增加预算；用户决定最小协议收敛方向后实施，VA-03 保持 blocked |
| TaskSpace base escape repair matrix | 2026-08-10 | [`40-va02-source-structured-ab-plan.md`](40-va02-source-structured-ab-plan.md)；`WAR-20260810-230951-R8-E01-ESCAPE-R3`；commit `85f14967c` | Standard/Structured/Source 各 repeat 3；TaskSpace 顶层 client escape 0/6，专用 base 实际进入 Provider。Standard 3/3，Structured 与 Source 均 0/3；父子节点 handoff 未被 Tool 合同清楚表达，outer decode reject 分别 8/18 和 7/18；observer 不能识别当前 carrier 和 Responses `instructions` | 先离线修复 I03 handoff 合同与 I07 观测，再申请新的最小真实预算；VA-03 继续 blocked |
| VA-02 Structured 收口修复 | 2026-08-11 | [`40-va02-source-structured-ab-plan.md`](40-va02-source-structured-ab-plan.md)；COE H-012～H-014 | Source 路线退役并从 active code 删除；Tool 合同补清父子节点 handoff，canonical 示例通过生产 preflight；observer 与 Responses base identity 对齐当前 wire | 运行缓存门禁并提交；VA-02 Structured 真实复验另行申请预算，VA-03 继续 blocked |
| VA-02 latest revalidation | 2026-08-11 | `219e1bb1d`；`WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7`；[`coe case`](../../../../coe/2026-08-10-06-00-r8-b5-va02-evidence-closure.md) | 8 个 Provider 请求完成代码修复、3 项测试和 Map；I05 在线关闭。首请求仍少一个闭合括号；一次 patch 绑定 waiting `fix` 被正确拒绝；Patch lifecycle 把两次内嵌声明误报为 0；最终回复在第 9 次本地尝试被预算门禁截断 | 不增加运行；先实施 SR-01～SR-03、WF-01、OB-03，VA-03 继续 blocked |
| Phase B5 latest-gap replanning | 2026-08-11 | 用户确认受限自愈产品规则；`session/turn`、preflight、patch observer 静态调用链 | 自愈必须发生在正式 ResponseItem 落账前，修正版成为唯一上下文；waiting 是正确派生状态，缺口是父节点事实未在拒绝中展开；Patch 专项仍解析旧载体 | SR-01；不新建平行计划，不运行真实 Agent |
| Phase B5 SR/WF/OB offline closure | 2026-08-11 | 80 个 TaskSpace Exec tests、9 个 Router tests、3 个 observer self-tests；`WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7` 原 rollout 离线复算 | 唯一单闭合符自愈发生在正式落账前；waiting 拒绝列出未完成直接父节点并保持零执行；Patch 专项复算为 2 声明/1 预检拒绝/1 执行结果/1 无法解析 | 原离线闭环被后续中文 production payload 暴露的 byte-column 缺口限定；读取下一条 |
| VA-02 SR/WF/OB production revalidation | 2026-08-11 | `WAR-20260811-052713-CACHE-REGRESSION-AD3C808C`；10 requests；171,324 input / 157,312 cached / 14,012 uncached / 6,409 output；request 2+ hit 91.20%；USD 0.0041966736 | 缓存和 usage 完整，但业务失败。自愈未触发的直接工程根因是把 `serde_json` UTF-8 byte column 当字符列号；syntax reject 又无条件注入 no-wrapper 提示，Agent 随后实际生成 wrapper。后续 mixed batch 还存在超出单闭合符范围的结构错误，未执行 patch；本轮没有 waiting preflight | 已实施 SR-04、FF-01 并通过 81 个 TaskSpace Exec tests；不得据此宣称 I03 在线稳定，也不得未经新预算再跑 Provider |
| SR-04 / FF-01 authorized package | 2026-08-11 | [`41-phase-b5-sr04-ff01-revalidation-result.md`](41-phase-b5-sr04-ff01-revalidation-result.md)；`R8-SELFHEAL-USD050-20260811` | 3 次有效 Provider 运行全部业务成功；21 requests，344,635 input / 323,200 cached / 21,435 uncached / 6,555 output，估算 USD 0.00574126。无 syntax、wrapper 或逃逸；参数均原生合法，未自然触发自愈。R04 两次 waiting 拒绝均忠实列出未完成父节点并由 Agent 下一请求纠正 | 简单样本生产路径通过；I04 行为继续观察；不为等待偶发自愈触发继续消耗预算；VA-03 仍按原计划单独决策 |
| SC-01 result contract compression | 2026-08-11 | [`42-phase-b5-schema-compression-result.md`](42-phase-b5-schema-compression-result.md)；`WAR-20260811-183331-CACHE-REGRESSION-A5B4F20D`；`WAR-20260811-183627-CACHE-REGRESSION-0A1511B5` | 两轮业务成功；production Tool wire 每请求减少 4,749 bytes。同为 7 requests 时 input 减少 12,435（10.83%）；暖缓存 req 2+ hit 94.04%。无结果语义、schema、syntax、wrapper 或逃逸回归 | 保留 SC-01；下一轮只评估 SC-02 示例去重；有效运行累计 USD 0.0048342 |
| SC-02 / SC-03 compression closure | 2026-08-11 | [`42-phase-b5-schema-compression-result.md`](42-phase-b5-schema-compression-result.md)；`WAR-20260811-185507-CACHE-REGRESSION-508F6FC2`；`a07dfd11e` / revert `a58666eb1` | SC-02 仅省 439 bytes 且会删除终态合批示例，静态证伪未运行；SC-03 省 473 bytes，但在线第二请求发生顶层 client Tool 逃逸，收益不足以支持追加复验，已整 commit 回退 | 压缩专题以 SC-01 收口；总费用 USD 0.0061536；不以 `$ref`、隐藏能力或弱化合同继续压缩 |
| LS-01 | 2026-08-12 | canonical protocol 4、Action Map 20、TaskSpace Exec 82、State 16、CLI 5 tests；PowerShell Store 环境契约；blocked active-symbol audit 为零 | canonical Map 收敛为 Waiting/Ready/InFlight/Completed 并破坏性升级到 v5；旧 v4 与 `blocked` 均明确拒绝，不提供迁移、兼容或默认映射；DAG、Agent 完成权与 Tool 结果语义未变 | 缓存门禁及提交完成后进入 LS-02 |
| LS-02 catalog foundation | 2026-08-12 | TaskSpace Exec 82 tests；client/Hosted capability 构建、identity、deferred 与现有 declaration 回归 | client 与 Provider capability 已合并为单一 Catalog 事实表；当前 Agent-visible wire 暂未改变，统一 `tools[]` 只允许由 LS-03 从该表生成 | 提交后进入 LS-03；LS-03 未完成前不宣称统一 Agent-visible Tool 槽位完成 |
| LS-03～LS-05 | 2026-08-12 | TaskSpace Exec 72 tests；L1～L8 正反 schema、统一 `tools[]`、normalization、DAG/preflight、Ready→InFlight、self-heal 与 handler 副作用边界 | Agent-visible 任意 `calls[]` 已替换为 8 个 exact sequence；Runtime 只归一化为 pre-map/tools/terminal-map 并按固定阶段预检，不推断 Tool 依赖或节点完成 | 缓存门禁及提交完成后进入 LS-06 Provider 稳定索引与对账适配 |
| LS-06 | 2026-08-12 | TaskSpace Exec 72 tests；Provider-only/mixed/missing/wrong-tool/multi-node/failed 对账矩阵；outer result 与 trace identity | Provider 动作保留统一 `tools[]` 稳定位置并与同响应事实逐项核对；client 保持原 Router；Provider 不重执行、不默认绑定，Tool 结果不改变节点生命周期 | LS-07 |
| LS-07 | 2026-08-12 | L1～L8 observer self-test、Patch observer、performance observer 全部 PASS | canonical rollout、性能报告和拒绝反馈读取同一闭集合同；报告术语收敛为 Provider action/result，不再把 Provider 描述为独立 binding 通道 | LS-08 |
| LS-08 | 2026-08-12 | TaskSpace Exec 72/72；Core 1881 passed / 3 ignored；final-wire 2/2；workspace check、zero-base、observer suites 与缓存敏感面门禁 PASS | 旧 A2 probe、无消费者 helper、Agent-visible `calls[]`、`hosted_bindings` 和 blocked 生命周期均从 active code 清除；Standard final wire 未改变 | 提交推送后激活已批准 LS-09 |
| LS-09 Run A | 2026-08-12 | `WAR-20260812-232516-CACHE-REGRESSION-55EA8834`；`single-file-fast-fix × map-request × repeat=1`；8 Provider requests；115,779 input / 95,488 cached / 20,291 uncached / 2,352 output；USD 0.0037666664；业务与隐藏 oracle 通过 | 8 次请求保持同一 Tool schema 与 `tool_choice`，无零命中或 shape transition；request 2+ 为 87.99%，其中 request 2 仍在预热，第 3 次起为 93.65%。严格前缀差异来自 `map-request` 每轮替换请求尾 Map handle，不是 Tool schema 漂移。一次 L4 尝试同时完成父节点并手工把 Tool owner 设为 InFlight，触发零副作用拒绝；既定 PD4 已由 Tool action 机械启动 Ready owner，缺口是该规则未在模型可见合同中直述，现以一行合同和断言补齐，不改状态机 | Run A 结算完成；候选 final-wire 免费门禁通过，发布基线继续阻断。重建当前二进制后按原授权首次执行 Run B；不得重跑 A |
| LS-09 orphan preflight | 2026-08-12 | `WAR-20260812-232437-CACHE-REGRESSION-8B1AF391` 仅完成 Provider route preflight；runner 因相对 `run-root` 参数在创建 ledger/sample 前退出 | 0 sample、0 Provider request、0 token、未认领预算；保留原始 preflight 证据，不伪造真实 run ledger | 不计入三项真实验收，也不删除证据 |
| LS-09 Run B | 2026-08-12 | `WAR-20260812-235208-CACHE-REGRESSION-273B1476`；`subscription-billing-repair × map-request × repeat=1`；12 Provider requests；211,107 input / 182,144 cached / 28,963 uncached / 5,956 output；USD 0.0062325032；业务与隐藏 oracle 通过 | 11 个顶层调用全部为 `taskspace_exec`，无普通 Tool 逃逸；最终 `root -> explore -> fix -> verify -> finish` 为 5 节点/4 边、无环、唯一终点并完整闭合。一次 `apply_patch` 的嵌套字符串含裸换行，触发 JSON syntax reject；随后两次 L2 `work` 选择尚有未完成直接父节点的 Waiting 后继，均被完整预检以零副作用拒绝，Agent 下一请求改用 L4 `update_and_work` 恢复。L2/L4 schema 分支当前只冻结形状，适用前提仍主要位于全局说明和示例，不能证明选择行为已收敛。该线性 Map 也未实际覆盖原计划的 fork/join 与运行中 Map 调整 | 按“结构异常即停”约束暂停 Run C，不重跑 B。LS-09 保持未通过；先决定最小合同修复与针对性复验，不晋升缓存基线 |
| LS-09 branch applicability repair | 2026-08-13 | `sequence_schema.rs`、catalog declaration test；TaskSpace Exec 72/72；zero-base PASS；缓存敏感面门禁 PASS | L1～L8 的 `anyOf` 分支均获得自包含适用条件；L2 明确旧 Tool outcome 不完成 owner，L4 明确只有前置 Map update 解锁本批 owner、同批 Tool outcome 不解锁后继。未增加 Runtime 决策、拒绝或状态转移 | 离线实现完整；Run B 仍是修复前历史证据。获得新的真实运行预算并复验之前，不宣称 waiting 误选频率或成本改善；不自动恢复 Run C |
| LS-09 Run C | 2026-08-13 | [`44-ls09-run-c-result.md`](44-ls09-run-c-result.md)；`WAR-20260813-053410-CACHE-REGRESSION-93CFAC19`；`provider-web-search-probe × map-request × repeat=1`；12 Provider requests；334,942 input / 291,584 cached / 43,358 uncached / 15,517 output；USD 0.0112313152 | 修复后的 L2/L4 在 Map 初始化后被正确采用，未复现 Waiting 误选；业务文件、公开验证和隐藏 oracle 通过。但 Agent 在 Hosted `web_search` 的 Exec 归属声明上连续 7 次试探，耗尽请求上限，未执行最终校验、Map 闭合和回复。根因是 Agent 可见合同只给出 `tool + node_ids` 结构，未说明它是同响应 Hosted output 的逐项归属声明、参数不在 Exec 内、失败 output 也必须声明 | 正式验收未通过，不重试、不晋升缓存基线；Hosted action 最小操作合同继续归入 I03，离线修复后另行申请最小复验预算 |
| LS-09 Hosted contract repair | 2026-08-13 | 最终 Provider-visible Hosted variant 与 catalog test；TaskSpace Exec 72/72 | 在统一 `tools[]` 内补回同响应已执行 output、无原生 input、逐项按序覆盖、失败项/action subtype 也声明、始终逐字使用原生 ToolSpec 名五项合同；Runtime 对账、DAG、状态转移和 Provider 执行均未改变 | 缓存敏感面门禁和提交后，按用户批准执行 `provider-web-search-probe × map-request × repeat=1`，零自动重试 |
| LS-09 Hosted contract revalidation | 2026-08-13 | [`45-ls09-hosted-contract-revalidation-result.md`](45-ls09-hosted-contract-revalidation-result.md)；`WAR-20260813-061928-CACHE-REGRESSION-04087B3B`；12 Provider requests；367,309 input / 318,976 cached / 48,333 uncached / 13,674 output；USD 0.0114884728 | 新合同已进入真实 wire，但 Agent 仍把 Hosted action 当作带输入的 client 执行请求，并在错误响应轮次补登记。业务文件未生成，Map 未闭合。证据表明缺口不再只是文字遗漏，而是统一 `tools[]` 同时承载执行前 client 请求和执行后 Hosted 凭据，生命周期语义冲突 | LS-09 未通过，不重试、不晋升缓存基线；先由用户确认 Hosted 绑定继续逐 output item，还是改为逻辑 Provider Tool 调用粒度，再重写后续实施计划 |
| LS-09 logical Hosted repair | 2026-08-13 | 用户确认 Provider 内部 `search/open_page` 不得拆分；TaskSpace Exec 73/73 | 同一 response scope 的同种 Hosted capability 聚合为一个逻辑 action；Agent 只声明一次 `node_ids`；Map action identity 改用 outer call + Tool index；schema、日志和反馈删除 Provider internal ID/output index | 完成缓存门禁和当前二进制构建后，按批准执行一次 `provider-web-search-probe × map-request × repeat=1`，零自动重试 |
| LS-09 logical Hosted revalidation | 2026-08-13 | [`46-ls09-logical-hosted-revalidation-result.md`](46-ls09-logical-hosted-revalidation-result.md)；`WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE`；12 Provider requests；301,975 input / 264,064 cached / 37,911 uncached / 10,772 output；USD 0.0090630792 | 逻辑聚合通过：同响应 `search + open_page` 只产生 `actual=[web_search]`，一次 `find_in_page + web_search` 声明只返回一个 Hosted result，无 Provider internal ID/index/subtype。端到端未通过：Agent 当轮漏声明后，下一请求补声明已不属于原 response scope，现合同无合法恢复路径 | 不重试、不晋升缓存基线。LS-09/I03 只剩跨响应 Hosted 归属恢复的产品边界待决策；禁止 Runtime 自动绑定或默认 Root |
| LS-09 Hosted execution-direction repair | 2026-08-13 | 原始 rollout requests 1～3/6；TaskSpace Exec 74/74 | Hosted action 必填 `execution: "already_executed"` 且不允许 `input`；Tool description 新增同响应归属示例；`actual/declared` 调试结构改为“本响应已执行但未归属/本响应未执行但已归属”的忠实反馈。Provider outcome、逻辑聚合、Map 和 client Tool 路径不变 | 离线实现完成；运行缓存门禁。真实复验需专用预算，不在未验证前引入跨响应 pending |
| LS-09 Hosted execution-direction revalidation | 2026-08-13 | [`47-ls09-hosted-execution-direction-result.md`](47-ls09-hosted-execution-direction-result.md)；`WAR-20260813-220517-CACHE-REGRESSION-ED5FF5CE`；12 Provider requests；302,780 input / 254,720 cached / 48,060 uncached / 7,710 output；USD 0.009600416 | Agent 已能准确解释并成功使用 `already_executed`，同响应真实搜索与归属对账成功，业务、oracle 和 Map 闭合通过；但仍有一次提前登记，以及一次真实搜索漏登后下一响应补登失败。方向判别是有效修复但不是完整根因 | 保留轻量结构，不引入 Runtime 自动绑定、默认 Root 或 pending；I03 继续 verifying，后续样本继续观察同响应漏登频率 |
| LS-09 same-response pairing repeat=5 | 2026-08-14 | `806b29780`；[`48-ls09-same-response-pairing-repeat5-result.md`](48-ls09-same-response-pairing-repeat5-result.md)；5 runs / 60 requests；1,947,752 input / 1,772,544 cached / 175,208 uncached / 51,739 output；USD 0.0439791632 | 同响应双写合同可被 Agent 理解并完成 8 次对账，但五轮均有协议或序列错误，公开验证仅 2/5。后续纠偏确认内部 action 映射不是缺口；真实缺口是两个独立顶层 item 缺少结构性耦合，以及 Hosted 归属后同批完成 owner 的合法顺序缺失 | I03 继续 verifying；分别设计和验证两个单变量修复，不晋升缓存基线，不引入自动绑定或跨响应 pending |

## 5.1 PA：Provider Action 待归属恢复

2026-08-14 用户确认废弃同响应双写，改为 Runtime 按 Provider 原生调用边界持久化待归属 Action，并在下一次请求要求
Agent 通过 `taskspace_exec` 完成节点归属。该变更由 `00-product-contract.md` 的 PD5、PD8 管辖，替换 LS-09 后续所有
“继续强化同响应配对”方向。历史 LS-09 结果只保留为失败证据，不再是活动合同。

| ID | Objective | Location / Target | Concrete Action | Resulting Behavior / Benefit | Side Effects | Verification / Stop | Status |
|---|---|---|---|---|---|---|---|
| PA-00 | 冻结新归属合同 | `00-product-contract.md`、本计划、README | 删除活动合同中的同响应双写与跨响应禁止，确认 Provider 事实、Agent 归属、Runtime 硬门和结束条件 | 后续实现只有一条产品权威，不在旧协议上叠加 pending | Complexity: 产品合同切换；Reach: PA 全部单元 | PD5/PD8 可追溯；活动文档无相反要求 | verified |
| PA-01 | 建立持久化待归属事实 | canonical SQLite Store、TaskSpace session state | 新增最小 pending action 表/操作：稳定 action identity、原生 Tool 名、机械 outcome、Provider 原生关联；不保存 input/output | Provider Action 不依赖 outer Exec 存活，重启后仍可恢复 | Complexity: 一个关系表和窄 Store API；Reach: TaskSpace Store，不影响 Standard | round-trip、幂等、重启、blank-map fixture；若需要后台 worker 或复制结果则停止 | verified |
| PA-02 | 接入 Provider response 采集 | response completion lifecycle、Hosted classifier | 在 Provider 原生响应完成边界幂等入队；同 response 的同一原生 ToolSpec 内部 items 聚合为一个逻辑 Action，不拆 search/open 等内部步骤 | 已发生事实不会因 Agent 漏写或 outer Exec 拒绝而丢失 | Complexity: 移动 Hosted 事实消费者；Reach: TaskSpace response lifecycle | 0/1/N、重复投递、failed/cancelled、无 outer Exec；Standard 0-diff | verified |
| PA-03 | 暴露待归属事实 | TaskSpace request/context constructor、outer result | 队列非空时在下一请求尾部暴露 `action_id/tool/outcome`；不复制结果、不修改稳定 base/schema 前缀 | Agent 获得完成归属所需事实且缓存影响局限于动态尾部 | Complexity: 一个机械事实块；Reach: 三种 projection 模式 | exact projection、无结果重复、缓存 source gate | not-started |
| PA-04 | 替换 Exec 归属协议 | `sequence_schema.rs`、`plan.rs`、`protocol.rs` | 从 `tools[]` 删除 Hosted/`already_executed`；新增 `assign_pending_actions[{action_id,node_ids}]` 固定前缀和有证据的初始化归属、纯归属场景 | Provider 归属不再伪装成 Tool 执行，仍由同一个 Exec 管理 | Complexity: schema/parser 净替换；Reach: TaskSpace final wire/cache | 正反 schema、decode、无旧字段残留；不得新增 generic escape | not-started |
| PA-05 | 建立归属硬门和原子结算 | preflight、handler、canonical Store | 除 `read_map` 外，队列非空要求完整覆盖；校验 ID、重复、多节点和 Work node；Node action 写入与出队同事务；队列非空禁止 finish/end | Runtime 只维护事实和底线，Agent 独占节点选择权 | Complexity: 一个动态硬门和组合事务；Reach: Map revision/finish | partial/wrong/duplicate/multi-node/completed-node/finish/restart；零自动绑定 | not-started |
| PA-06 | 删除旧双写链并收敛反馈 | response scope、preflight error、result、tests、active docs | 删除同响应 actual/declared reconciler、Hosted `tools[]` result、配对错误和提示；反馈只列待归属事实与硬规则 | 不再同时维护两套归属路径，也不诱导 Agent 模拟 Provider Tool | Complexity: 预期净删除；Reach: observer/fixtures/docs | active-symbol audit、TaskSpace tests、observer fixtures | not-started |
| PA-07 | 完成离线与真实验收 | workspace/cache gates、Docker benchmark、run ledger | 先完成 focused/core/Standard/final-wire/cache 门禁，再执行 `provider-web-search-probe × map-request × repeat=3` | 验证归属稳定性、业务结果、请求/token/cache 成本和无新协议异常 | Complexity: tests only；Reach: 真实 API 成本 | 三轮均无双写错误、无漏绑、Map 闭合；预算/账本完整；失败即停并归因 | not-started |

### PA Pre-Phase Plan Rebase Gate

| Before Unit | Evidence Reviewed | Delta | User Approval | Gate |
|---|---|---|---|---|
| PA-01 | LS-09 repeat=5、native-contract 修复与 2026-08-14 用户决策 | material：同响应配对改为跨请求持久化归属 | user-approved-plan-direct: “开始执行，提前批准预算，持续执行到完成” | ready |

## 6. 证据校准

| Date / Evidence | New Fact | Prior Conclusion | Validity Change | Downstream Change | Plan Validity / Next |
|---|---|---|---|---|---|
| 2026-08-06 / `protocol/src/taskspace.rs` 静态检查 | `taskspace-canonical-map-v3` 仍在 Map 顶层分别保存 `action_records`、`result_refs`、`evidence_refs`、`completion_records`、`block_records`；action 再用 `node_id` 反向关联节点 | “B0 后 canonical Action Map 可原样作为 NX-01 的事实底座” | 当时 qualified；已被下一条最简 Map 决策进一步 invalidated | 原 NX-00A～NX-00G 不再执行，后续以 MM-00～MM-10 为准 | invalidated / 读取下一条 |
| 2026-08-06 / 用户确认最简 Map | Node 必须直接展示 goal/state/content/parents/children/actions；Agent 只声明 parents，Runtime 机械反算 children；无 edges、Map ref、语义分类模块或 handoff condition | “把 v3 ledger 搬入节点并保留 result/evidence refs” | invalidated：旧 NX-00A～NX-04 未执行部分不再代表目标模型 | 以 MM-00～MM-10 重建 Map，再执行 EX/OB/VA；旧代码失效或无消费者即删除 | valid / 先完成 MM-00 后执行 MM-01 |
| 2026-08-06 / 用户确认净删除原则 | 旧设计不得改名、残留、暂留或以 dormant/compatibility 形式保留；无生产消费者代码也必须删除 | “可先保留 replay/detail-fold 等基础，后续再判断” | invalidated：keep 必须由新模型的当前生产责任证明，未来可能需要不是理由 | MM-01 建删除清单；MM-02～MM-10 每单元同步净删除并以零残留门禁收口 | valid / MM-01 |
| 2026-08-07 / 用户确认低延迟结算与关系化 Store | 整图 JSON 覆盖不适合逐 Tool 结算；合法 Exec 应在完整预检后立即执行，每项结果完成即写回唯一 Map | “按整批 Tool 完成后统一结算以减少 revision” | invalidated：revision 增长不得以反馈延迟为代价；整图重写也不得成为临时生产方案 | EX-05 先独立证明原生 dispatch；MS-01～MS-03 随后建立关系化 Store 与逐项 outcome 结算，再继续 Hosted/反馈/注册 | valid / EX-05 |
