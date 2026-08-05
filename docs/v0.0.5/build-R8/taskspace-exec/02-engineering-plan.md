# TaskSpace Exec 正式工程计划

- Created: 2026-08-05
- Revised: 2026-08-06 after `se-good-plan` review
- Status: Phase A direction-supported / downstream plan revised / Phase B1 plan-ready
- Plan validity: valid-with-qualifications
- Risk depth: Full
- Product contract: [`00-product-contract.md`](00-product-contract.md)
- Upstream evidence: [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md)
- Issue dependency review: [`03-global-issue-prerequisite-review.md`](03-global-issue-prerequisite-review.md)
- Integrates: I10 能力身份、I06 统一 admission、I01/I02/I05 结果合同、I07 新协议观测
- Excludes: projection 三模式重构、Map 压缩、旧数据兼容、Provider Tool proxy

## 1. 问题与最小建设方向

当前生产 TaskSpace 使用 `taskspace_control + sibling client calls` 表达动作序列和节点归属，导致 Tool schema 侵入、事后
配对、重复反馈和 admission 分散。主方案以一个 Function Call 形态的 `taskspace_exec` 取代旧顶层动作协议，但继续复用
Codex ToolSpec、ToolRouter、权限、sandbox、hook、业务 handler 和 canonical Action Map。

最小必要增量只有：

1. 一个由原 ToolSpec 快照机械派生的结构化 `taskspace_exec` declaration；
2. 一个只活在当前 Provider response 内的 envelope；
3. 一个先完成硬规则 admission、再调用原 Router 的 outer executor；
4. Provider-hosted fact 与 Agent 节点声明的机械核对和 canonical 持久化；
5. 一份不改写原结果的 outer result，以及可复算关联日志。

不建立第二 Router、第二 Map、Session 全局暂存、Provider proxy、旁路 binding database、reasoning parser、旧协议兼容层或
Runtime 语义决策器。

## 2. 目标数据流

```text
TaskSpace provider request
  -> one structured Function Tool: taskspace_exec
  -> provider-native hosted capability descriptors

provider response
  -> hosted output items 0..N                         -- provider 已执行
  -> exactly one taskspace_exec Function Call         -- Agent 声明动作和节点归属
  -> response.completed

existing response coordinator
  -> freeze one response-local envelope
  -> attach Runtime-owned protocol/catalog/call identities
  -> parse structured Agent arguments
  -> preflight TaskSpace hard rules and canonical Map candidate
  -> reconcile hosted facts with Agent-declared node sets
  -> dispatch admitted client/map calls through the original ToolRouter
  -> persist canonical facts and return one outer FunctionCallOutput
```

## 3. 已冻结不变量

1. Standard 的 ToolSpec、provider payload、dispatch 和结果保持结构等价。
2. TaskSpace 顶层只暴露 `taskspace_exec + provider-native hosted capabilities`。
3. 一次 Provider response 只允许一个 outer `taskspace_exec`；内部不得递归调用 `taskspace_exec` 或原 `exec`。
4. Agent 决定实际 Tool、数量、原生参数、数组顺序和节点归属；Runtime 不预生成、不补全、不重排。
5. 普通 client call 由 Agent 声明单个 `node_id`；Hosted fact 由 Agent 声明非空 `node_ids[]`；Map call 不声明外层 owner。
6. 协议版本、能力快照身份和内部调用身份由 Runtime 从 request-local ToolSpec、outer `call_id` 和数组位置机械维护，不要求
   Agent 回显。
7. 普通 Tool schema、参数、handler、权限、sandbox、hook 和原生结果对 TaskSpace 无感。
8. Client Tool 合同只暴露一次：Standard 在顶层，TaskSpace 在 `taskspace_exec` 内；Hosted 完整合同只在 provider 顶层。
9. Tool declaration 只由确定排序的 ToolSpec 快照和协议版本生成，不含 Map、node、plan、Provider output 或 Session 数据。
10. Agent 合同、Hosted reconciliation 或 Map admission 失败时，尚未发生的 client/map 零执行、Map 零提交。
11. Provider Tool outcome、client Tool outcome 和节点生命周期互不推导。
12. Hosted 漏绑、错配、歧义、非法节点或 Provider 身份缺失/重复使整个响应不被接受；不得默认 Root 或未绑定池。
13. 每个 Tool 结果只进入 Agent context 一次，不新增 developer/system factual carrier。
14. 每个实施单元同时增加该边界的成功、拒绝与 reason code 日志；不得等最终观测阶段再补基本诊断。
15. 不为实验数据或旧 wire 增加兼容分支。

## 4. 当前证据状态

这里分开记录验证、候选 artifact 和生产集成，避免把隔离原型误报为已落地功能。

| ID | Evidence / Artifact | Validation State | Artifact State | Production State | Current Decision |
|---|---|---|---|---|---|
| TX-00 | request/usage identity aggregation | direction-supported | implemented | runtime-verified | 保留，继续作为成本事实源 |
| TX-01 | 当前生产、Codex main 和删除 seam 盘点 | direction-supported | verified discovery | not-applicable | 后续优先复用已定位入口 |
| TX-02 | ToolSpec catalog 与 capability identity 候选 | direction-supported | prototype-verified | not-started | identity 保留为 Runtime metadata，不进入 Agent 参数 |
| TX-03 | source-only typed plan decoder | direction-rejected for production | prototype-verified | not-started | 只保留历史证据，TX-06E 删除候选代码 |
| TX-04 | 结构、序列、单 Patch 纯预检 | direction-supported | prototype-verified | not-started | 迁移到结构化参数，不复制 Map 规则 |
| TX-05 | Hosted 逐项多节点声明与机械核对 | direction-supported | prototype-verified | not-started | Provider identity 由 Runtime 读取；持久化表示仍待 TX-12A |

Phase A 证明的是“存在值得正式投资且不违反产品约束的路径”，没有证明结构化 carrier、response lifecycle、Map、Store、
Router、反馈或生产行为已经集成。

## 5. 实施前与阶段内验证

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
|---|---|---|---|---|---|---|---|
| V-B1 | DeepSeek 能在最终结构化 schema 下生成合法 1/N client calls，而不是只在 source-only 候选下工作 | 是否继续投入 response/Map/Store 全链路 | TX-06B/C 完成后，先用 fixture 验证 wire，再申请一次 `deepseek-v4-flash` 无副作用 shape probe | Enough: outer Function Call 可解析，Tool variant、原生参数和 node metadata 保真；Not proven: 任务质量、成本、Hosted 全链路和长期稳定性 | Budget: 1 sample × 1 arm × repeat 1，最多 2 个 Provider request；启动前登记 ledger 并单独说明 token/费用/时间上限；Allowed: 未接生产 schema 和 disposable harness；Forbidden: 扩大 repeat、生产切换 | 首个结构性失败即停；证据归档，probe 不进入生产 runtime | planned |
| V-B2 | canonical persistence 能以一份 Provider fact 表达多个节点引用，且无需旁路事实源 | TX-12B 的数据模型和 API | 静态源码/SQLite schema/rollout codec 盘点，加最小内存 fixture；不先建新表或迁移 | Enough: 找到可复用关系结构，或证明必须扩展 canonical event schema；Not proven: 完整 replay/compaction 正确性 | Budget: 1 个 discovery 单元，零 API；Allowed: 文档、fixture、未注册类型草图；Forbidden: 生产 schema/default 修改 | 得出唯一表示方案即停；无可行单事实模型则回到产品决策 | planned |

DeepSeek 官方 strict tool schema 当前要求 beta endpoint，而 Whale 当前生产 Tool 使用 `strict:false`。首版结构化
`taskspace_exec` 继续把 JSON Schema 作为 Agent 生成合同，并由 Runtime parser/preflight 承担最终硬校验；启用 beta strict
属于独立 provider 路由决策，不在本计划中暗中切换。

## 6. 工作单元

所有未来单元使用 Plan Authoring 状态；只有实际实施后才另行记录 `implemented/integrated/runtime-verified`。

| ID | Objective | Axis | Location / Target | Concrete Action | Resulting Behavior / Benefit | Side Effects | Verification / Safe Stop | Plan Status |
|---|---|---|---|---|---|---|---|---|
| TX-06A | 建立唯一中性 ToolSpec 投影 | internal | `codex-tools` code-mode conversion helper | 从现有 code-mode helper 中复用或中性抽取 Function/Freeform/Namespace 到内部定义的转换，不加入 TaskSpace 字段 | Code Mode 与 TaskSpace 共用一份能力转换，避免第二 registry | Complexity: 一个共享 helper；Reach/Cost: code-mode tests 受影响，Standard wire 不变 | code-mode exact snapshots；若必须复制 registry 或修改普通 ToolSpec，停止 | planned |
| TX-06B | 生成最终结构化 outer schema | API | `taskspace_exec/catalog.rs`、`plan.rs` | 从 TX-06A 输出生成 Tool-specific call variants、`calls[]` 和 `hosted_bindings[]`；Agent 参数只含动作、原生输入和节点声明 | Provider schema直接表达合法结构，Runtime仍做硬校验；移除 Agent 对 `version/capability_id/item_id` 的机械回显 | Complexity: 一个静态 Function schema builder；Reach/Cost: TaskSpace declaration 体积和缓存指纹会改变 | schema fixtures；client 1/N、hosted-only `calls=0`、完全空计划拒绝、不同顺序；若必须回到 source 字符串，停止 | planned |
| TX-06C | 固定 Runtime 能力身份和单次暴露 | cache/API | request-local catalog、TaskSpace projection fixture | 从同一 ToolSpec 快照生成 Runtime-only identity、outer declaration 和 nested dispatch catalog；验证 TaskSpace 顶层 client 零暴露但不切生产默认 | 声明、执行、日志和缓存引用同一能力快照；原 Tool 合同不重复 | Complexity: Runtime metadata 增加；Reach/Cost: 静态 payload 测量增加，生产入口尚不切换 | 重复构建逐字一致、catalog/Router 一一对应、Standard exact equality、Tool 合同/TaskSpace metadata/序列化体积分拆、缓存门禁 | planned |
| TX-06D | 验证最终 schema 的 Provider 兼容性 | validation | V-B1 disposable harness、run ledger | 执行 V-B1，不运行完整软件工程样本 | 在投入 response/Map/Store 前验证最高风险外部假设 | Complexity: 不增加生产代码；Reach/Cost: 一次获批 API 成本 | 原始 response、usage 和 schema 指纹可复算；失败则暂停 B1 后续 | planned |
| TX-06E | 退役 source-only 候选 | cleanup | `taskspace_exec/decoder.rs`、source schema/tests | TX-06D 支持继续后，删除 `taskspace.plan(<JSON>)` decoder、`source:string` declaration和只服务该候选的 tests | 只剩结构化协议，不形成两个 carrier | Complexity: 净删除候选；Reach/Cost: Phase A 历史文档保留，生产无影响 | `rg` 不存在 active source decoder；TX-06B～D 全绿后执行，可单独回退 | planned |
| TX-07A | 保留 Provider 原始响应顺序身份 | response lifecycle | Responses SSE decoder | 在 decoded output item 上保留原始 `output_index`，不以 done 顺序替代 | Hosted facts 可按 provider response 顺序唯一恢复 | Complexity: 一个 wire metadata 字段；Reach/Cost: decoder fixtures 变化 | 乱序 done、重复/缺失 index、Standard 0-diff；无法无损保留时停止 | planned |
| TX-07B | 建立 response-local envelope | response lifecycle | `session/turn.rs` 附近的 response coordinator | 收集 0..N Hosted items 和唯一 outer call，在 `response.completed` 冻结并立即消费 | executor 获得同响应完整事实，不建 Session 全局状态或重放重建 | Complexity: 一个局部值对象；Reach/Cost: response loop、abort/cancel fixtures | 0/1/N Hosted、缺/多 outer、abort/cancel；若需跨响应存活，停止 | planned |
| TX-08 | 建立 outer response executor | internal | `taskspace_exec/response_executor.rs`、现有 sequence入口 | 消费 TX-07B envelope，附加 Runtime-owned identities，解析一次，拒绝递归/多 outer，并保留原 outer `call_id` | `taskspace_exec` 获得唯一 Runtime 入口和原生 output pairing | Complexity: 一个 executor，不注册第二 Router；Reach/Cost: sequence/cancel 路径受影响 | decode、pairing、timeout、零 nested dispatch；若普通 handler 必须读取 envelope，停止 | planned |
| TX-09A | 固定各 Map action 的 canonical 接缝 | discovery | `session/taskspace_response.rs`、`ActionMapRuntimeState::prepare_response_for_main`、原 control handler | 逐项映射 initialize/execute/reopen/read/finish 的纯检查、提交和 reservation 边界，记录可直接复用入口 | 后续不臆造 `map_admission` 抽象，也不复制状态规则 | Complexity: 只读盘点和 fixtures；Reach/Cost: 零生产影响 | 每种 action 有唯一入口和副作用时点；存在缺口则把缺口拆成实施单元 | planned |
| TX-09B | 接入非终态 Map preparation | state | TX-09A 确认的现有 Session/ActionMap API | 将结构化 map calls 和 client node declarations 转为现有 `ActionMapDeclaredCall`/prepared reservations | init/execute/reopen 与 client reservations 在 dispatch 前通过 canonical validator | Complexity: 一个窄 adapter；Reach/Cost: Map revision/reservation tests 扩大 | stale、unknown/not-ready node、非法 graph、零提交拒绝；若需复制规则，停止 | planned |
| TX-09C | 接入 read 与 terminal Map 边界 | state | 原 `taskspace_control` handler、canonical finish path | 按 TX-09A 证据连接 read-only 和 `finish_map`，保持 finish 显式且位于最终边界 | read 不伪造事务；终态仍由 Agent 显式关闭且不自动推导 | Complexity: 复用不同既有 action seam；Reach/Cost: finish/read fixtures 变化 | read-only、work+finish、过早 finish、reopen 后 finish；无法先验证再提交时停止讨论 | planned |
| TX-10 | 机械执行 admitted client calls | internal | `taskspace_exec/dispatch.rs`、`ToolRouter`、nested call adapter | 用 outer call ID 与数组位置生成内部 call identity，还原原生 ToolCall 并交给原 Router | Function/Freeform/MCP/Namespace/ToolSearch/LocalShell 保持原权限、hook 和结果 | Complexity: 一个 invocation adapter；Reach/Cost: 并行、取消和全部 Tool 类型测试扩大 | 类型矩阵、单 Patch、独立并行、结果依赖拆批、plan 外调用零执行；任一 Tool 需改原生 args 时停止 | planned |
| TX-11 | 原子核对 Hosted facts | provider | `provider_reconcile.rs`、TX-07B envelope、TX-09B/C admission | 按 output index、数量、Tool 类型、真实 Provider ID 和 Agent `node_ids[]` 生成完整 relation set | 只有完整归属的 Provider facts 可进入 canonical persistence，不重执行、不猜配 | Complexity: 扩展 reconciler；Reach/Cost: Web/Image 和整批失败反馈增加 | 0/1/N、多 owner、failed、缺/重复 ID/index/node、prelude-created node；任一 finding 返回零 accepted relations | planned |
| TX-12A | 冻结单事实多节点持久化表示 | data discovery | V-B2、Action Map store、Event Store、rollout codec、SQLite state | 执行 V-B2，选择唯一 canonical 表示并写明 identity、atomicity、replay 和 compaction 责任 | 消除“现有 Store 已支持 node set”的错误前提，避免旁路账本 | Complexity: 设计结论；Reach/Cost: 决定 TX-12B/C 影响面 | 证据必须指向现有类型/schema；无可行表示则暂停 B2 | planned |
| TX-12B | 实施 canonical Hosted relation persistence | data | TX-12A 选定的现有 canonical store/schema | 保存一份 Provider raw fact 和 Agent 声明的节点引用集合，按 Provider ID 原子查重并拒绝冲突集合 | 同一事实不复制，多个节点可稳定引用 | Complexity: 扩展一个既有数据模型；Reach/Cost: codec/schema/store tests 变化，无新数据库 | same-set 幂等、conflicting-set reject、非法/空 owner reject；不得默认 Root/unbound | planned |
| TX-12C | 完成 restore/replay/compaction 闭环 | data | rollout reconstruction、SQLite hydration、compaction | 让 TX-12B 的 fact/relation set 经 restart、resume、fork 和 compaction 后保持一致 | Map 和事实始终固化存在，不依赖重放重建临时 envelope | Complexity: 扩展恢复测试；Reach/Cost: session/state tests 增加 | Web/Image round-trip、SQLite restore、resume/fork/compaction identity；失败不增加兼容旁路 | planned |
| TX-13 | 返回唯一无损 outer result | feedback | `taskspace_exec/result.rs`、outer FunctionCallOutput | 组合内部原生结果、Hosted settlement、失败来源和唯一 final revision，不摘要或重写结果 | Agent 一次获得完整可继续事实，承接 I01/I02/I05 | Complexity: 一个 result builder；Reach/Cost: final wire/context snapshots变化 | 每个结果一次、原文/结构保真、错误类型不互换、Standard 0-diff；任一结果丢失则停止 | planned |
| TX-14A | 建立新协议关联重算 | observability | provider/response/dispatch/store trace consumer | 消费各实施单元已经产生的 stage events，按 request/response/outer/internal/provider/node identity 重算一条链路 | 失败可定位到生成、解码、Map、Provider、dispatch、store或反馈 | Complexity: 一个聚合器；Reach/Cost: trace schema consumer增加 | 合成 trace 重算、local reject provider delta=0、无重复 request/token；不得另建事实源 | planned |
| TX-14B | 更新 benchmark/report 消费 | observability tooling | TaskSpace benchmark parser、performance report | 从 TX-14A canonical facts 输出 request/token/cache/time/cost 和 Map/exec 明细 | 新旧协议对比不依赖脆弱日志文本 | Complexity: 更新工具消费端；Reach/Cost: benchmark snapshots变化，runtime不变 | fixture report 与原始 trace逐 ID 对账；失败不修改 runtime 语义 | planned |
| TX-15 | 只切换 TaskSpace Tool projection | integration/cache | request Tool projection、provider builder | 将 TaskSpace model-visible Tool list 从旧 sibling 集合替换为 `taskspace_exec + native hosted`；不在本单元新增 TX-07～14 逻辑 | 生产生成入口切到已集成路径，Standard 不变 | Complexity: 一个 mode projection 替换；Reach/Cost: 所有 TaskSpace 请求和缓存指纹一次变化 | 缓存门禁、payload snapshots、Standard exact equality、每份 Tool 合同恰好一次；阻断后申请专项预算 | planned |
| TX-16A | 删除旧 sibling 执行路径 | cleanup | `sequence.rs`、preflight、response pairing | 删除旧 control manifest、sibling admission/executor 和 receipt 生成分支 | Runtime 只有一个 TaskSpace 动作协议 | Complexity: 净删除执行分支；Reach/Cost: sequence tests 迁移 | `rg` 删除清单、core/tool tests；TX-15 未通过不执行 | planned |
| TX-16B | 删除旧 schema/prompt/context 路径 | cleanup | `taskspace_tool`、旧 context carrier、prompt/observer | 删除 schema 入侵、旧完整合同复述和额外 factual carrier | Agent 只看到新协议的一份合同和一份结果 | Complexity: 净删除输入/反馈分支；Reach/Cost: prompt/context snapshots变化 | payload/context snapshots、缓存门禁、无旧 wire；可单独回退 | planned |
| TX-16C | 清理旧测试与工具解析器 | cleanup tooling | sequence fixtures、benchmark parsers、历史 compatibility helpers | 删除只验证旧 wire 的 active tests/parser，保留文档证据但不保留运行时兼容 | CI 和性能工具只维护当前协议 | Complexity: 净删除和迁移 fixtures；Reach/Cost: CI/test inventory变化 | `rg` 无 active old protocol reference；历史 docs 不作为失败项 | planned |
| TX-17A | 验收 schema、暴露和 Standard 0-diff | test | Tool projection/provider payload tests | 验证 final schema、唯一暴露、确定性 declaration、Standard exact equality | 证明协议入口和缓存静态面正确 | Complexity: fixtures；Reach/Cost: CI时间增加，零 API | 指定 snapshots 全绿；失败停止后续矩阵 | planned |
| TX-17B | 验收 response、Map 和 dispatch | test | session/action_map/router integration tests | 覆盖初始化并工作、多节点 calls、完成并继续、read、finish、单 Patch、取消 | 证明硬规则和 client 副作用边界 | Complexity: integration fixtures；Reach/Cost: CI时间增加 | 非法输入零 client/map 副作用，合法结果完整 | planned |
| TX-17C | 验收 Hosted、Store 和恢复 | test | provider/store/replay integration tests | 覆盖同响应多 Hosted、多 owner、失败状态、冲突、restart/replay/compaction | 证明事实唯一且节点引用可恢复 | Complexity: store fixtures；Reach/Cost: CI时间增加 | 任一错绑整批拒绝，same-set replay 幂等 | planned |
| TX-17D | 验收结果、日志和缓存门禁 | test/observability | final wire、trace、cache gate | 验证一次结果、reason code、逐 ID 重算和预期静态指纹变化 | 真实运行前证明反馈与测量可信 | Complexity: contract fixtures；Reach/Cost: CI时间增加 | I01/I02/I05/I07 验收矩阵全绿；失败不启动真实 run | planned |
| TX-18A | 形成真实运行专项预算 | validation planning | run ledger、Docker benchmark plan | 基于 TX-17A～D trace 预估并向用户申请 `2 samples × 2 arms × repeat 1 = 4` 个 arm-runs 的模型、请求、token、费用、耗时、停止与重试预算 | 满足全局成本授权，不用模糊“各跑一次”绕过计数 | Complexity: 只写预算；Reach/Cost: 零 API | 用户明确批准前保持 planned；默认模型 `deepseek-v4-flash`、不允许自动重试 | planned |
| TX-18B | 验证真实行为、缓存和成本 | validation | Docker benchmark、run ledger | 仅按 TX-18A 获批矩阵执行 Standard/TaskSpace 简单和复杂样本 | 判断正确实现是否形成真实产品收益 | Complexity: 不增加生产代码；Reach/Cost: 产生获批 API 成本和时间 | 逐 request trace、Map、token/cache/time/cost；异常先归因，不扩大 repeat | planned |
| TX-19 | 重评 R8 问题全集 | planning | `01-r8-known-issues.md` | 只依据新生产 trace 更新 I01～I10 的关闭、改方案或优先级 | R8 回到唯一证据化问题队列 | Complexity: 文档状态变化；Reach/Cost: 改变后续顺序 | 每项有新协议证据；无 E2/E3 证据保持 open | planned |

## 7. 阶段与停点

### Phase B1：结构化承载与响应入口

- Entry: Phase A direction-supported；本轮计划复审已完成。
- Units: TX-06A～TX-08，其中 TX-06D 是进入 response/Map 广泛建设前的有界外部验证，TX-06E 只在该验证支持继续后清理候选。
- Exit: 最终结构化 schema、唯一 ToolSpec 投影、Runtime-owned identities、response-local envelope 和 outer executor 均有
  确定性证据；source-only 候选已删除；Provider shape probe 支持继续投资。
- Stop: 必须复制 registry、双重暴露 Tool 合同、修改普通 Tool schema、回到 source-only、建立 Session 全局 envelope，或
  V-B1 首次结构性失败。

### Phase B2：Map、执行与 Hosted 持久化

- Entry: B1 reconciliation 结论允许继续。
- Units: TX-09A～TX-12C。
- Exit: 原 canonical Map admission、原 Router dispatch、Hosted relation persistence 和 restore/replay 形成离线闭环。
- Stop: 需要复制 Map 规则、修改普通 Tool args、用语义猜配 Hosted、复制 Provider fact、增加旁路数据库，或 V-B2 无法找到
  单事实多引用模型。

### Phase B3：反馈与可观测性

- Entry: B2 reconciliation 结论允许继续。
- Units: TX-13～TX-14B。
- Exit: outer result 只传一次，trace 和报告能从 Provider request 逐 ID 重算到内部结果、节点和 Hosted fact。
- Stop: 任一原生 Tool 结果丢失、摘要替代事实、错误类型互换或 observer 成为第二事实源。

### Phase C：生产切换与清理

- Entry: B1～B3 全绿并完成 reconciliation；缓存敏感面已被门禁识别。
- Units: TX-15、TX-16A～TX-16C。
- Exit: TX-15 只切 projection；TaskSpace 生产只剩 `taskspace_exec`，Standard 0-diff，旧协议无 active code。
- Stop: TX-15 需要同时新增前序业务逻辑，或删除旧路径前新入口尚未通过确定性测试。

### Phase D：确定性验收、获批真实验证与问题重排

- Entry: Phase C reconciliation 允许继续。
- Units: TX-17A～TX-19。
- Exit: 正确性、真实质量、缓存、成本和 R8 问题状态均有可复算证据。
- Stop: 任一 TX-17A～D 矩阵失败则不申请真实运行；TX-18A 未获明确批准则不执行 TX-18B。

## 8. 阶段证据重排

| Phase / Review | New Evidence | Affected Assumption | Conclusion Update | Downstream Plan Change | Plan Validity | Next Action |
|---|---|---|---|---|---|---|
| Phase A | Function outer、Runtime Provider ID、逐项多节点声明和纯预检方向有证据；source-only 真实 probe 失败 | source-only 能否直接生产、Agent 是否应复制传输身份 | `qualified:` 结构化 Function 值得实施；source-only 不生产；Provider/Runtime 身份不由 Agent 回显 | TX-06 改为结构化 schema 并增加候选清理 | valid-with-qualifications | revise-complete |
| 2026-08-06 plan review | `version/capability_id/item_id` 属于 Runtime；Map/client/Hosted 绑定形态不同；Event Store 只有单 owner；原 TX-06/TX-15/TX-16/TX-17 过粗 | Phase B 是否可按旧单元直接执行 | `needs-revalidation:` Provider final schema兼容性与 Hosted 多节点持久化仍未知；其余方向继续成立 | 拆分 TX-06/07/09/12/14/16/17/18，增加 V-B1/V-B2，TX-15 收窄为 projection-only | valid-with-qualifications | continue TX-06A |

每完成 B1、B2、B3 或 C，必须新增一行，使用 `current/qualified/superseded/invalidated/needs-revalidation` 记录结论，并明确
下游单元继续、修改、拆分、删除、重排、暂停或停止。`needs-revision` 或 `invalidated` 不得原样进入下一阶段。

## 9. 执行约束

- 每个 TX 单元单独实现、测试、提交和推送；不得把相邻单元提前混入。
- 每个代码单元同时建设该边界的日志和 reason code；TX-14A/B 只做跨边界重算与报告，不补救此前不可观测实现。
- `preflight.rs` 和 `provider_reconcile.rs` 已接近 500 行；实施扩展前先把同文件 tests 机械迁出，行为不变并单独提交。
- 任何 Tool declaration、base instruction、context、projection 或 provider payload 变化先执行缓存门禁。
- 真实 Whale Agent run 必须先写 planned ledger；超过 3 个 `sample × arm × repeat` 必须按 TX-18A 申请专项预算。
- 验证失败不得靠自动重试覆盖；预算外重试必须重新登记和申请。
- 旧候选代码只能在替代路径通过后删除，不保留兼容暗线；历史文档保留为证据，不作为 active code 残留。
- 涉及 Provider route、canonical 多节点持久化模型或新的状态语义时暂停实施，提交证据和代价给用户决策。
