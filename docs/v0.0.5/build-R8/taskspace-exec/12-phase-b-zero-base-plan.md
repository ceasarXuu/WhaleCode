# Phase B 零基线重建计划

- Created: 2026-08-06
- Status: Active / Phase B0 verified / Phase B1A next
- Supersedes: [`02-engineering-plan.md`](02-engineering-plan.md) 中 TX-06B 之后的兼容迁移顺序
- Completed foundation: TX-06A (`54fc781fc`)
- Paid Whale Agent run: 本阶段删除与离线建设不需要

## 1. 决策

TaskSpace Exec 不再从当前 sibling 协议渐进迁移。先删除旧协议对 schema、parser、handler、sequence、context、response
和 feedback 的影响，再从 Standard 原生链与 canonical Action Map 原语建设新协议。

不做：

1. 旧 `taskspace_control.actions[]` 到新 calls 的 adapter；
2. 旧、新两套 Agent-visible schema 或 parser；
3. 为保持旧 TaskSpace 可运行而增加 feature branch、fallback 或补字段逻辑；
4. 把旧 sibling tests 改名后继续作为新合同；
5. 从旧 handler 参数反推新的 Map Tool 合同。

旧 TaskSpace rollout、专用对话事件和未发布数据不提供迁移、读取或 fallback；零基线只接受 Standard
原生历史格式与独立 canonical Action Map Store。

## 2. 零基线边界

### 2.1 可复用白名单

| Foundation | Reuse Boundary |
|---|---|
| Standard ToolSpec / Tool registry plan | Tool 事实、静态 schema、能力排序；TaskSpace 不修改普通 Tool |
| Standard ToolRouter / ToolCall | 原生权限、sandbox、hook、handler 和结果执行 |
| Provider response lifecycle | 原始 Function Call、Hosted output item、call ID 和完成边界 |
| Canonical Action Map | 只复用 DAG、revision、状态转换、Store、restore/replay 机制；现有顶层 action/result/evidence/lifecycle 平行账本不能原样复用，须先完成节点所有权重置 |
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
| Phase A prototype | `source:string` exec、Agent 回显 version/capability/item ID 的 decoder/preflight |
| Tests / prompts | 以旧合法序列、旧字段或旧反馈为正确答案的 active fixture 与说明 |

历史文档和 benchmark evidence 可保留，但不能被编译、注册、加载或作为新测试 fixture 输入。

### 2.3 参考依据

以下资料只用于校验所有权和实现边界，不意味着引入新的 DDD 框架、CQRS 服务或 Event Store：

1. [Microsoft：Design a microservice domain model](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/microservice-domain-model)
   将 aggregate root 定义为一致性入口。对应本计划：Map transaction 是唯一写入口，节点子事实不能被旁路 ledger 独立修改。
2. [Microsoft：Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
   区分权威事实流与可再生成的 projection/snapshot。对应本计划：projection 和临时索引只从 canonical Map 派生，不成为
   第二事实源；本项目不因此新增 Event Store。
3. [Serde：Container attributes](https://serde.rs/container-attrs.html)
   明确 `deny_unknown_fields` 会拒绝未知字段。对应本计划：schema v4 直接拒绝 v3 顶层 ledger，不以忽略字段实现静默兼容。

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
| NX-00A | 冻结节点所有权合同 | `00-product-contract.md`、canonical Map schema design | 明确 action、result/evidence reference、completion/block 和 terminal 的唯一物理归属，禁止 canonical 顶层平行账本 | Map 结构直接表达“哪个节点拥有哪次工作”；Exec 不再依赖 Map 外 binding store | Complexity: 收窄一个既有数据模型；Reach: NX-00B～NX-03 | 合同覆盖普通 client、Hosted 多节点引用、生命周期正交与唯一性；仍有双重权威即停止 | planned |
| NX-00B | 重塑 protocol canonical schema | `protocol/src/taskspace.rs` / `TaskSpaceCanonicalMap`、node/action records | 将 action 及其 result/evidence refs 嵌入 owner node，将 completion/block 嵌入对应节点，将终态记录归入 Finish；删除顶层五类平行 ledger 并升级 schema version | 序列化 Map 自身即可恢复节点完整事实，不再依靠跨表 join 解释归属 | Complexity: 一次破坏性 schema 简化；Reach: protocol callers 与 fixtures；不增加迁移 | protocol round-trip、unknown-field rejection、旧 v3 fixture 明确拒绝 | planned |
| NX-00C | 改造 transaction 与硬规则 | `core/src/action_map/rooted_dag/` / events、transactions、transitions、invariants | 所有动作和引用只经 owner node 修改；用临时派生索引检查全局 ID、DAG 和状态规则；移除顶层 ledger 读写 | Agent 声明的节点归属直接落到节点；Tool 结果仍不自动推进节点状态 | Complexity: 重写单一事实写路径、删除平行 mutation；Reach: replay/property tests | transaction、invariant、replay tests；同输入重放得到同一 canonical bytes | planned |
| NX-00D | 改造持久化与恢复 | `state/src/runtime/taskspace_map_codec.rs`、`core/src/session/taskspace_store.rs` | SQLite 只保存和恢复新 canonical Map；删除 v3 读取、迁移、兼容和 fallback；schema mismatch 输出机械诊断 | Map 继续是全局唯一持久事实，不靠聊天重放重建，也不保留旧实验数据债务 | Complexity: 删除兼容面、升级存储合同；Reach: 新 Map 数据与 restore | store round-trip、restart/hydration、revision/hash tests；旧 schema 输入失败且不静默降级 | planned |
| NX-00E | 改造 projection 与 snapshot | `core/src/action_map/runtime/` / projection、snapshot、state | projection 从节点聚合读取，保持完整 DAG 全局视图；禁止再投影一份顶层 action/result/evidence 权威副本 | Agent 看到的 Map 与持久事实同构，不因 projection 再制造平行语义 | Complexity: 删除旧字段遍历；Reach: always/append/request 三种模式 | 三模式 deterministic snapshot、全局骨架与近端详情 fixtures；同一事实只出现一次 | planned |
| NX-00F | 改造外部读取面 | protocol session snapshot、CLI debug、Viewer、导出脚本 | 所有消费方改读 node-owned shape，删除兼容 alias 和旧字段拼装 | 调试、可视化和导出不会把已删除的顶层账本重新塑造成产品事实 | Complexity: 调整消费者、不加 adapter；Reach: CLI/API/Viewer/scripts | protocol/schema、CLI、Viewer 与 export fixtures；仓库 active consumer 无旧 shape | planned |
| NX-00G | 建立结构零残留门禁 | canonical schema tests、`scripts/taskspace-exec/` static audit | 增加结构化 fixture 检查，阻止顶层 ledger、生命周期 action_id 和双写路径回流 | 后续 Exec 实施只能依赖节点所有权模型 | Complexity: 一项静态/确定性门禁；Reach: pre-commit 与后续 Map 改动 | targeted suites、zero-base gate、cache gate；不得只靠字段名 grep 误报节点内合法字段 | planned |
| NX-01 | 建立纯 Map operation contract | 完成 NX-00A～NX-00G 后的 canonical Map transaction 邻接模块 + 新 ToolSpec | 从 node-owned transaction 原语定义 initialize/execute/reopen/read/finish，不含 Tool manifest 或 binding store | Map Tool 只管理同一份 Map；Exec 的节点声明直接提交到 owner node | Complexity: 一个新合同；Reach: Map fixtures | schema/parser/transaction fixtures；出现 sibling、平行 ledger 或兼容字段立即停止 | planned |
| NX-02 | 建立结构化 TaskSpace Exec schema | 新 `taskspace_exec` 模块 | 从 TX-06A 与 NX-01 生成 calls/hosted_bindings Tool-specific variants | Agent 只声明动作、原生输入和节点归属 | Complexity: 一个静态 Function schema；Reach: declaration size | 1/N client、hosted-only、empty reject、确定性字节 | planned |
| NX-03 | 建立 request-local identity 与唯一暴露 | Standard request Tool projection 的 TaskSpace 分支 | 从同一快照生成 Runtime identity、Exec declaration 和 nested catalog | TaskSpace 顶层只有 Exec + Hosted；Standard 0-diff | Complexity: 一个模式投影；Reach: provider payload/cache | payload snapshot、Router 一一对应、cache gate | planned |
| NX-04 | 验证 DeepSeek 最终 schema | disposable shape probe + ledger | 获批后运行 1 sample × 1 arm × repeat 1，最多 2 requests | 在 response/Map 大建设前验证最终 Function schema | Complexity: 零生产逻辑；Reach: 付费 API | 首个结构失败即停；启动前另行申请预算 | planned |

NX-00G 通过前不得开始 NX-01；NX-04 通过后再重新设计 response-local envelope、Map admission、Router dispatch、Hosted
persistence、一次反馈和生产验收；
旧计划中的 TX-07～TX-19 不自动继承，必须基于零基线代码重新盘点。

## 4. 阶段门禁

### Phase B0：Zero-Base Reset

- Entry: 用户明确要求不保留过渡方案，从 Standard 零基础建设。
- Units: ZB-01～ZB-07。
- Exit: active code/config/prompt/test 不再包含旧 sibling/control 协议；Standard 构建、Tool wire、sequence、response 与缓存门禁成立。
- Stop: 删除触及 canonical Action Map 数据事实、Store、projection 三模式或 Standard 原生 Tool 行为时立即停下重新划界。
- Clarification: 上述 Store 指 SQLite canonical Action Map Store；ZB-06C 删除的是旧方案的对话 Event Store，不是 Map 事实源。

### Phase B1A：Node-Owned Canonical Map Reset

- Entry: B0 forbidden-symbol audit 和 Standard 回归通过。
- Units: NX-00A～NX-00G。
- Exit: canonical Map 的 action、引用和生命周期事实均只有节点内唯一权威表示；Store、replay、projection 和所有 active consumer
  使用同一结构，旧 v3 数据被明确拒绝且没有兼容路径。
- Stop: 需要增加旁路 binding database、顶层事实 ledger、聊天重放重建 Map、迁移旧实验数据，或无法保持 Tool 结果与节点生命周期正交。

### Phase B1B：Clean Exec Contract And Provider Gate

- Entry: B1A 全部通过，节点所有权结构零残留门禁生效。
- Units: NX-01～NX-04。
- Exit: 纯 Map operation、结构化 Exec、唯一能力暴露均有离线证据，且获批 Provider shape probe 支持继续。
- Stop: 需要恢复旧字段、复制普通 Tool schema、增加兼容 adapter、修改普通 Tool 参数或由 Runtime 推断 Agent 动作。

## 5. 执行记录

| Unit | Date | Evidence | Conclusion | Next |
|---|---|---|---|---|
| TX-06A | 2026-08-06 | `54fc781fc`；tools 154 passed / 1 ignored；TaskSpace Exec 33 passed；cache gate PASS | 中立 ToolSpec projection 保留；旧 TaskSpace prototype integration 不保留 | ZB-01 |
| ZB-01 | 2026-08-06 | `1143706a1`；全局约束、README、active plan 交叉引用 | 旧兼容迁移计划 invalidated；零基线计划成为唯一 active Phase B 计划 | ZB-02 |
| ZB-02 | 2026-08-06 | `2960ea03a`；`cargo check -p codex-core --lib` PASS；`codex-tools` 154 passed / 1 ignored | Phase A source-only prototype 无生产依赖并已从 active code 全部删除 | ZB-03A |
| ZB-03G | 2026-08-06 | `4472c2afa`；cache gate policy-only PASS；Standard 两请求 final-wire tests PASS | 清零期缓存门禁已删除旧 TaskSpace wire 夹具，只验证 Standard；发布保持阻断，NX-03 后重建 TaskSpace 合同 | ZB-03A |
| ZB-03A | 2026-08-06 | `cd327d938`；`codex-tools` 145 passed / 1 ignored；core build、ToolSpec 与 provider visibility unit tests PASS | 旧 control declaration、schema、registry handler 和 active schema fixtures 已删除；未增加替代或兼容入口 | ZB-04A |
| ZB-04A | 2026-08-06 | core build PASS；stream events 15 passed；mailbox、malformed arguments、missing client identity Standard tests 各 1 passed | Tool item 恢复为原生 future；turn 恢复 `FuturesOrdered` 落账；response completion 不再调用旧 sequence | ZB-04B |
| ZB-04B / ZB-03B / ZB-05 | 2026-08-06 | `5228efd80`；旧 sequence、control parser/handler、Provider response gate 静态为零；core build 与 Standard stream tests PASS | Tool response 已回到 Standard 原生调度与反馈路径，无兼容 adapter | ZB-06A |
| ZB-06A / ZB-06B | 2026-08-06 | core build；rooted DAG 19 tests；protocol 3 tests；schema fixtures 4 tests；Viewer 3 tests；core-skills 95 tests；skills 1 test | 旧 Prompt/Skill/fixture 已删除；Map 只保存 action-node 归属，不再保存或等待工具 reservation | ZB-06C |
| ZB-06C | 2026-08-06 | 删除约 2200 行旧 Event Store/codec/checkpoint/test；core/protocol build；历史后端目标测试 2 条；Standard rollout reconstruction 19 条；普通 Tool 错误反馈与 Hosted output 各 1 条；schema fixtures 4 条；Standard final-wire 1 条 | 所有模式只使用 Standard `ContextManager` 与 Standard rollout；模式切换不再搬运聊天历史；canonical SQLite Action Map Store 未改动；不兼容旧专用事件 | ZB-07 |
| ZB-04C / ZB-07 | 2026-08-06 | `scripts/taskspace-exec/check_zero_base.py`；门禁单测 3 条；全仓 active surface PASS；Standard final-wire PASS；cache gate PASS | 旧 control/sibling/sequence/Event Store/reservation 符号在活动表面为零；pre-commit 自动阻止回流；历史 docs 与 benchmark evidence 不误报 | NX-00A |

## 6. 证据校准

| Date / Evidence | New Fact | Prior Conclusion | Validity Change | Downstream Change | Plan Validity / Next |
|---|---|---|---|---|---|
| 2026-08-06 / `protocol/src/taskspace.rs` 静态检查 | `taskspace-canonical-map-v3` 仍在 Map 顶层分别保存 `action_records`、`result_refs`、`evidence_refs`、`completion_records`、`block_records`；action 再用 `node_id` 反向关联节点 | “B0 后 canonical Action Map 可原样作为 NX-01 的事实底座” | invalidated：DAG/Store/replay 机制可复用，但 canonical 数据所有权不可原样复用 | 在 NX-01 前插入 NX-00A～NX-00G；NX-01 改为只依赖 node-owned transaction | valid-with-qualifications / 先执行 NX-00A |
