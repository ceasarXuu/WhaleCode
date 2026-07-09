# R5 TaskSpace 简洁模型分阶段收敛计划

> 本计划从 `docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md` 派生。
> R5 的每个 phase 都必须证明：拆掉过度设计没有引入负向收益，且 TaskSpace 更接近
> `standard` 自然上下文的图化/状态机化再组织。

## 1.1 元数据

```text
Created: 2026-07-09
Updated: 2026-07-10
Version: v0.0.5 build-R5
Status: In Progress - R5-E4 completed; R5-F is ready
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, action_map runtime, taskspace_control,
  context projection, provider-visible context, benchmark harness
Related Links:
  docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md
  docs/v0.0.5/build-R5/02-r5-phase-a-current-state-inventory.md
  docs/v0.0.5/build-R5/03-r5-phase-b-node-event-contract.md
  docs/v0.0.5/build-R5/04-r5-phase-c0-cadence-parity.md
  docs/v0.0.5/build-R5/05-r5-phase-c-thin-projection-action-sequence.md
  docs/v0.0.5/build-R5/06-r5-phase-c1-native-tool-loop-boundary.md
  docs/v0.0.5/build-R5/07-r5-phase-c-exposure-followup-plan.md
  docs/v0.0.5/build-R5/09-r5-phase-d-ledger-deactivation.md
  coe/2026-07-10-01-54-r5-normal-progress-budget-hard-stop.md
  coe/2026-07-10-05-03-r5-stale-active-projection-accumulation.md
  docs/v0.0.5/build-R4/10-r4-request-convergence-engineering-plan.md
Risk Level: High
Plan Type: Full
```

## 1.2 问题定义

当前 TaskSpace 的 active path 已经包含过多 runtime 语义层：

```text
problem_ledger
cognitive_state
success_criteria / fact_sources / output_contracts
facts / decisions / result_validity / adoption
projection coverage inference
critical_artifact_evidence / dependency_read_evidence
next-valid-actions / recovery guidance
semantic gate recovery messages
```

这些结构让 runtime 从“map 状态账本”变成“半语义控制器”。R5 要通过分阶段收敛，逐步把
TaskSpace 拉回三个职责：

1. map 管理。
2. node-local 上下文归档。
3. 状态机硬规则底线。

## 1.3 目标和非目标

目标：

| Goal | Expected Benefit | Verification |
|---|---|---|
| 建立最小 TaskSpace 状态模型 | 减少 runtime 语义所有权 | schema/代码路径审计能解释每个字段的状态机用途 |
| 恢复接近 standard 的工具执行节奏 | 避免 TaskSpace 把普通工作拆成大量 provider requests | paired sample 对比 request/tool cadence，不在 implement 前 hard stop |
| 移除普通请求次数 hard stop | 避免 runtime 截断正常、有进展的 Agent 执行并污染收益判断 | route profile 只产生 observability；超过 profile 后仍可继续采样；中断样本不得计入 utility |
| 收敛 provider-visible projection | Agent 看到忠实上下文而非策略提示 | payload diff 证明 projection 只含 map、node context、events、refs |
| 拆除 active 语义账本控制路径 | 避免局部文本被固化为 truth | `initial_*` 不再生成强事实/合同/决策控制 |
| 保留工具反馈忠实性 | 不因为简化丢失 failure semantics | targeted samples 检查 stderr/exit/path/ref 可见 |
| 防止负向收益 | 简化后不明显劣于 R4 正向样本 | paired metrics 对比 correctness、tool count、request count、tokens |

非目标：

```text
不在 R5 追求新的 benchmark 分数峰值。
不把失败样本通过 runtime 禁止动作来做 pass。
不要求一次性大爆炸改完所有代码；每个 phase 可以独立切断一类 active path。
不得为了历史数据或旧 schema 增加 runtime 兼容层、legacy adapter 或双写路径。
不重写整个 Codex upstream substrate。
```

## 1.4 约束和假设

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| TaskSpace 当前核心代码集中在 `action_map/runtime.rs` 和 `taskspace_control.rs` | R5-A 代码审计 | 先补模块边界图，再进入拆除 |
| R4 正向样本可作为回归防线 | R5-A 基线复用或重跑 | 选取更小 targeted baseline，不进入移除 phase |
| 简洁模型能保留当前运行必要 replay/debug refs | R5-B event/ref schema 验证 | 修正 NodeEvent 直接 schema，不增加旧结果结构兼容层 |
| Agent 能承担语义判断 | paired sample 观察 | 优先检查上下文透传，不回退到 runtime 策略控制 |

## 1.5 Phase 总览

| Phase | Theme | Main Output | Exit Gate |
|---|---|---|---|
| R5-A | Current-state inventory and baseline | 过度设计清单、活跃代码路径、R4 正负样本基线 | 每个复杂结构有 owner、active path、保留/降级/删除候选 |
| R5-B | Minimal map/event contract | 直接 NodeEvent 契约和写入路径 | 工具结果可按 node 忠实归档并可 ref 读取 |
| R5-C0 | Execution cadence parity | budget lifecycle 修复和 action-contract patch 语法归一化 | `count-call-stack` standard/R5 单样本均 solved，不再停在 implement 首轮或 malformed patch 归一化点 |
| R5-C | Projection thin mode and action sequence carrier | active projection 改为 map skeleton + current node + events/refs；action sequence 承载 Agent 明确多动作 | provider-visible diff 无策略提示、无语义重写；`count-call-stack` standard/R5 均 solved |
| R5-C1 | Native tool loop and projection boundary | DeepSeek 默认 native tools、机械空 map 初始化、删除 action-class projection 残留 | `count-call-stack` standard/R5 均 solved；rollout 无 `allowed action classes` / `hard action-class constraints` |
| R5-D | Semantic residue inventory and ledger deactivation | D0 审计 provider-visible 语义残留；D1 降级 `initial_*`；D2 降级 `problem_ledger/cognitive_state` active 控制权 | 局部任务文本和旧文案不再变成 canonical truth 或策略约束 |
| R5-E | Runtime hard-baseline pruning and projection uniqueness | E0 移除普通请求次数 hard stop 并修正中断/完成语义；E1 清除策略性 recovery/sentinel text；E2 建 hard-gate classifier；E3 审计 fallback；E4 active projection latest-only 替换 | route profile 不再终止正常执行；拒绝只保留硬底线；每个 provider payload 恰好一份最新 active projection |
| R5-F | Dead code cleanup and code split | 删除旧结构、模块拆分、移除兼容分支 | 生产路径不依赖旧语义控制，代码边界清楚 |
| R5-G | Regression and benefit gate | 正向/负向样本对照、成本和语义传递报告 | 不引入明确负收益，失败可解释 |
| R5-H | Closeout | R5 收口报告和后续 backlog | 文档、测试、代码、证据一致 |

### 1.5.1 Phase 验收和工程收益矩阵

| Phase | 验收标准 | 工程收益 | 度量 / 验证方法 |
|---|---|---|---|
| R5-A | 所有 active 复杂结构被标记为 `keep/thin/deactivate/delete/unknown`，且 `unknown` 有后续诊断 | 降低架构不确定性，避免盲删或继续堆补丁 | 结构清单覆盖 `state/projection/gate/tool feedback`；`unknown` 不允许进入删除 phase |
| R5-B | ordinary tool success/failure 都能归档为 node-local event，并保留 excerpt/ref | 提升反馈可追踪性，后续 projection 不必重新解释工具语义 | fixture 中 tool result -> node event attribution 覆盖率 100%；raw/ref 可恢复 |
| R5-C0 | 历史上修复 fresh-node/feedback grace 误扣和 patch 双 `End Patch`；R5-D 证明该措施只延后 profile hard stop，未修正抽象 | 保留 patch 归一化修复；预算 grace 作为 R5-E0 待删除技术债，不再视为最终收益 | patch normalization focused test 有效；budget lifecycle 结论由 R5-E0 重新验收 |
| R5-C | active projection 只含 map skeleton、current node、events、refs、hard status；routing prompt 不再 model-visible 注入策略 | 减少上下文污染和策略注入，让 Agent 直接面对忠实反馈；一次响应可承载多个 Agent 明确动作 | `projection_strategy_hint_count=0`；provider-visible payload diff 通过；`count-call-stack` R5-C live sample solved |
| R5-C1 | DeepSeek native tools 成为默认路径；TaskSpace 可机械初始化空 map；projection 不再暴露 node kind -> action class 合同 | 进一步贴近 standard tool loop，同时消除“runtime 不拦但 projection 还暗示不能做”的边界错位 | native alias/unit tests 通过；`count-call-stack` paired run both_success；文本扫描无旧 action-class contract |
| R5-D | 先完成 provider-visible semantic residue inventory，再让 `initial_*`、ledger、cognitive state 退出 active canonical truth | 防止旧文案或任务文本局部细节被 runtime 固化放大 | `state_machine_allowed_actions`、validation/recovery/sentinel/spawn 文案完成分类；H203/H204 path case 中 `/app` 不再由 projection/ledger 强化；state_commit 不要求 facts/decisions/adoption |
| R5-E | 普通 route/profile 请求计数不再 hard stop；真实中断不伪装 Agent 完成；保留 gate 只含硬底线；active projection 机械替换旧快照 | 清晰 runtime 边界，避免正常执行被截断、旧状态冲突和 projection 二次累积 | `profile_budget_hard_stop_count=0`；completion/validation 分离；`active_projection_count=1`；stale projection omission 可审计；rollout/payload scan 无策略文案 |
| R5-F | active path 不依赖旧 semantic ledger，模块边界测试通过 | 提升可维护性，降低 `runtime.rs` 混合职责继续扩张风险 | call/import 审计证明 projection 不调用 cognitive coverage helper；cargo check/test 通过 |
| R5-G | targeted paired runs 无明确 correctness 回退，成本无无解释放大；只有 Agent 生命周期完整且未被 runtime 中断的样本可进入收益统计 | 用未污染样本证明简化降低干扰且保留收益 | Agent completion、external validation、tool/model request、tokens、wall time、feedback completeness 分项对比报告 |
| R5-H | closeout 列出已删/降级/保留结构和后续删除条件，git clean | 形成可交接的架构边界和后续路线，避免 R5 结论再次散落 | closeout 文档、证据索引、clean git、保留复杂结构 owner/exit condition |

## 1.6 Phase R5-A：当前结构盘点和基线

目标：

1. 列出所有 active TaskSpace 语义结构和 runtime 决策点。
2. 区分“状态机必要字段”和“语义控制字段”。
3. 建立 R5 拆除前的 targeted baseline。

实现项：

| Item | Output |
|---|---|
| state field inventory | `TaskState/ActionMapInstance/MapNode/NodeResult/ledger/cognitive` 字段用途表 |
| projection inventory | active projection sections、裁剪规则、heuristics 列表 |
| gate inventory | tool gate、finish gate、validation gate、recovery message 分类 |
| baseline samples | R4 正向样本、H203/H204 负向样本、simple smoke 样本 |

退出门禁：

```text
每个复杂结构被标记为 keep / thin / deactivate / delete / unknown。
所有 unknown 都有下一步诊断，不允许进入删除 phase。
baseline 至少覆盖：simple success、tool failure feedback、H203/H204 path case、large output ref。
```

负收益防线：

```text
R5-A 不改生产行为，只产出审计和基线；任何代码修改仅限诊断日志或测试脚手架。
```

## 1.7 Phase R5-B：最小 map/event contract

目标：

把 TaskSpace 的目标结构明确为 map + node + node event。先建立新 contract，再迁移 projection。

最小 contract：

```text
TaskSpaceMapV2:
  task
  nodes
  edges
  node_events
  refs
```

实现项：

| Item | Expected Behavior |
|---|---|
| NodeEvent model | 工具调用、工具结果、Agent node summary、blocker、finish 都作为 node-local event |
| raw ref contract | 大输出或长历史不丢，只转为 raw_ref + bounded excerpt |
| attribution invariant | 每个 ordinary tool result 必须归属到一个 node 或明确硬失败 |
| snapshot/export | viewer/debug 能读取 map 和 node events |

退出门禁：

```text
direct tool success/failure 都能落到 node event。
event 有 raw_ref 或 visible_excerpt。
不需要 facts/decisions/fact_sources/output_contracts 也能解释工具反馈归属。
```

负收益防线：

```text
直接建立 NodeEvent 写入路径；旧 NodeResult/TaskSpaceTraceEvent 只能作为被替换对象，
不得引入兼容后端、双写或历史读取分支。
```

## 1.8 Phase R5-C0：执行节奏和 request cadence 收敛

目标：

让 TaskSpace 保持“必须使用、不可绕过的状态机/map 工具”定位，但执行节奏回到接近
standard 的自然工具循环。TaskSpace 不应把每个小动作都强制拆成独立 provider request，
也不应让状态机生命周期本身消耗掉简单任务的全部 request budget。

新增发现：

```text
count-call-stack R5-B:
  inspect 内连续 list/read/read/read/read/read
  第 7 次 provider request 才 finish_node -> implement
  implement node_request_count=0/2 时 hard stop

standard:
  同类任务表现为连续工具循环，能在一次自然工作流中读文件、patch、验证。
  当前 standard artifacts 对 provider request 的可观测性不足，不能只用
  request-summary 的 model_request_count=1 下定论；C0 必须补齐同口径日志。
```

设计原则：

```text
TaskSpace action 是状态机工具调用，不是 runtime 语义决策。
允许 Agent 在一个自然执行回合中推进多个合法 action。
runtime 只逐个校验硬规则、执行工具、记录 NodeEvent、返回忠实结果。
不得通过“少读文件”“必须进入实现”等提示词或语义 gate 解决 request 放大。
TaskSpace route/profile budget 只能观测；用户显式取消/绝对预算、provider/进程不可恢复故障等
外部严重异常才可终止。C0 当时采用 grace 延后 hard stop 的方向已由 R5-D 证据推翻，R5-E0 删除。
```

候选实现路径：

| Option | Description | Pros | Risks | Decision Gate |
|---|---|---|---|---|
| native tool-loop carrier | 将 TaskSpace control 和 ordinary tools 统一放回 provider/tool loop，由 runtime 在每次 tool call 前做硬状态机校验 | 最接近 standard；工具结果可自然连续返回 | DeepSeek/tool transport、cache 前缀和现有 action-contract 兼容成本需验证 | spike 证明同一 turn 内多工具结果可稳定进入 Agent 上下文 |
| action sequence envelope | 允许单个 provider response 携带 `actions: []`，runtime 顺序执行已知合法 action，遇到需看结果的步骤停止并返回反馈 | 改动集中；保留 action-contract 形态 | Agent 无法基于前一个工具结果动态决定同 envelope 后续动作；可能只适合无依赖动作 | 只用于无依赖状态机动作，不替代 tool loop |
| budget lifecycle rebase | provider request budget 从“每个 TaskSpace 小动作”改为“模型推理回合”，状态机 bookkeeping/tool execution 不额外放大 budget | 快速缓解 hard stop | 若没有 cadence 修复，仍会慢且贵 | 只能作为 C0 辅助，不可单独关闭 C0 |

实现项：

| Item | Expected Behavior |
|---|---|
| standard telemetry parity | standard 和 taskspace 都能输出同口径 provider request、tool call、tool-result feedback 日志 |
| action cadence audit | 每个 sample 记录 tool calls / provider requests / state-machine actions 三类计数 |
| carrier spike | 验证 native tool-loop 或 action sequence envelope 哪个能最少改动恢复连续执行 |
| budget gate adjustment | C0 历史措施只让新 implement 节点越过首轮 cliff；R5-E0 将删除普通 profile hard stop 及配套 grace |
| feedback invariant | 多 action/多 tool 结果仍逐条归档为 node-local `NodeEvent`，失败语义不丢 |

退出门禁：

```text
`count-call-stack` R5 当前阶段至少进入 implement 并执行一次合法 edit tool。
同口径日志显示 R5 不再是一小步一 provider request 的强制节奏。
standard/R4/R5 对比记录包含 provider request、tool call、state-machine action 三个维度。
没有新增“指导 Agent 少读/必须改”的 runtime 语义提示。
状态机硬约束仍生效：无 active map/node/lease、协议非法、权限/安全仍被拒绝并忠实反馈；
只有用户显式取消/绝对预算、provider/进程不可恢复故障等有明确来源的严重外部资源异常可中断。
```

负收益防线：

```text
如果 native tool-loop 路径出现具体可复现的工具 ABI 或反馈透传失败，先按能力层/反馈层修复；
不得预设 native tool-loop 不稳定，也不得回退到 runtime 语义约束。
request cadence 放大若仍存在，作为 R5-G 成本项解释和优化，不阻塞已验证的 correctness gate。
若多 action 执行导致反馈丢失，回退到单 action，但保留 NodeEvent 证据和问题记录。
```

## 1.9 Phase R5-C：Projection thin mode and action sequence carrier

目标：

将 active projection 收敛为薄视图，只暴露 map skeleton、当前 node、node-local recent events、refs。

允许内容：

```text
task id/title/objective/status
node id/kind/objective/status/dependencies
current node recent events
tool feedback excerpt/ref
hard state-machine status/errors
omission audit
```

禁止内容：

```text
next action strategy
coverage inference
critical artifact recommendation
validation rework strategy
semantic acceptance/adoption
path correction instruction
```

退出门禁：

```text
provider-visible payload diff 证明：thin projection 不包含策略性 next-action hints。
工具失败的 exit/path/stderr/ref 仍可见。
H203/H204 中 `/app` 不再因 projection 被强化为 canonical truth。
```

Phase C 实现补充：

```text
1. active projection 已收敛为 map skeleton、current node、node-local recent events、result refs、
   omission audit 和预算字段。
2. 支持 taskspace-action-sequence-v1，一次 provider response 可以携带多个 Agent 明确动作；
   runtime 逐个执行并只做硬状态机校验，遇到失败反馈停止本 sequence。
3. action-contract parse/reject、状态机拒绝、工具失败均保留为 node-local feedback/event。
4. benchmark routing 决策保持 artifact/report-only，不再通过 prompt 注入策略。
5. bootstrap developer context 改为 thin bootstrap，只保留 start/route 入口和硬状态机边界。
```

Phase C live evidence：

```text
RunDir: target/r5cphase6/count-call-stack/20260709-183144-389
standard current: solved, 15135ms, 10 tools
R5-C current: solved, 45228ms, 10 tools
R4-D historical: solved, 154525ms, 11 tools
right rollout_trace.model_request_count: 8
agent_messages: 8
agent_actions: 15
multi_action_messages: 4
old routing/compact prompt hits: 0
```

负收益防线：

```text
若 thin projection 导致 Agent 看不到工具失败细节，停止拆除，优先修 event/ref 透传。
```

### 1.9.1 Phase R5-C1：Native tool loop and projection boundary

C1 是 R5-C 后的边界补丁，不新增 runtime 智能层：

```text
1. DeepSeek 默认 transport 回到 native tools；显式 action_contract 配置才走 action-contract。
2. Runtime 允许做语义无关机械空 map 初始化，且 projection 明确 objective/node plan pending。
3. 删除 active projection 的 action-class contract 文案：
   hard action-class constraints
   Current node contract
   allowed action classes
4. 删除 NodeContract.allowed_actions，只保留仍被使用的机械 split hint。
```

C1 根因和修复证据：

```text
CoE: coe/2026-07-09-21-50-r5-native-tool-loop-agent-no-patch.md
Doc: docs/v0.0.5/build-R5/06-r5-phase-c1-native-tool-loop-boundary.md
Failing before fix: target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
Passing after fix: target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052
```

退出状态：

```text
standard: solved
taskspace: solved
failure_taxonomy: none
engineering_unclean: False
rollout no matches: allowed action classes / hard action-class constraints / Current node contract
```

## 1.10 Phase R5-D：语义账本降级

目标：

把 Phase C/C1 暴露出的 provider-visible 语义残留先盘清，再把
`problem_ledger`、`cognitive_state`、`facts`、`decisions`、`fact_sources`、
`output_contracts` 从 active runtime 控制路径降级。

执行拆分：

| Subphase | Scope | Exit Gate |
|---|---|---|
| R5-D0 | Provider-visible semantic residue inventory | 扫描并分类所有 `state_machine_allowed_actions`、validation/recovery/sentinel/spawn 文案；每条归为 hard baseline、mechanical status、semantic residue、debug-only |
| R5-D1 | `start_task initial_*` 不再自动提升为 canonical truth | H203/H204 中局部 `/app` 文本不再进入 fact/source coverage authority |
| R5-D2 | `problem_ledger/cognitive_state` 从 active projection/gate 移出 | 普通工作不依赖 facts/decisions/adoption 继续推进 |

处理策略：

| Structure | R5 Direction |
|---|---|
| `success_criteria` | 仅作为 Agent-authored node/task note，不能作为 runtime 语义 gate |
| `fact_sources` | 降级为 node note 或 event tag，不再做 coverage 控制 |
| `output_contracts` | 降级为 task note，不再强制 closeout |
| `facts/decisions` | 移出 runtime active projection，保留为 Agent 摘要事件 |
| `result_validity/adoption` | 不再表达语义信任链，仅保留可选 review note |

退出门禁：

```text
provider-visible 文案残留完成分类，明显越界项进入删除清单或已删除。
start_task initial_* 不再把任务文本细节提升成 active canonical truth。
state_commit 不再要求 Agent 维护 facts/decisions/adoption 才能继续普通工作。
closeout 不依赖 runtime 的 accepted semantic facts。
```

D0 分类白名单：

```text
允许保留：
- no active map/node/lease
- invalid node state
- protocol/schema parse failure
- permission/sandbox/security/explicit external resource failure
- output ref/crop explanation

必须删除或降级：
- 告诉 Agent 该 read/search/edit/test/final
- validation_needs_test 这类策略状态
- rejected_by_state_baseline: list_files/search/read_file/apply_patch 这类语义动作禁止
- coverage/fact_source/rework_target 作为 model-visible 指令
```

负收益防线：

```text
如果某个 benchmark 依赖 output artifact contract，先把它表达为 node objective/event note，
不能回退到 runtime output_contract 语义 gate。
```

## 1.11 Phase R5-E：runtime gate 修剪

目标：

先移除会截断正常执行并污染收益判断的普通请求次数 hard stop，再保留硬状态机底线、
删除或降级语义干预 gate，并把所有 model-visible recovery/sentinel 文案纳入同一边界审计。

R5-E0 是 R5-E 的强制入口门：E0 未完成前，后续 sample 只能用于诊断，不能用于声明
TaskSpace 的工具数、耗时、token 或完成率收益。

R5-E0 进入条件：

| Entry Criterion | Check Method | Evidence / Output |
|---|---|---|
| hard stop 根因已确认 | 审阅 COE H-001/H-002 evidence gate | `coe/2026-07-10-01-54-r5-normal-progress-budget-hard-stop.md` |
| 污染样本可复核 | 保留 R5-D rollout、last message、metrics | 成功 edit -> 7/6 hard stop -> runtime task_complete 事件链 |
| 外部严重中断能力边界已盘点 | 审计 Codex substrate 的用户取消、显式限额、provider/进程错误、benchmark timeout | 能力清单；未知项不得被 route/profile hard stop 代替 |

硬底线允许：

```text
无 task/node 归属时拒绝 ordinary tools。
节点状态非法时拒绝绑定或完成。
工具调用/结果配对非法时拒绝或报错。
权限、沙箱、安全、协议规则非法时拒绝。
输出过大时转 ref，但必须暴露 ref 和裁剪说明。
用户显式取消、用户显式配置的绝对预算、provider/进程不可恢复故障等可验证的外部严重异常可以中断。
```

不属于资源硬底线：

```text
TaskSpace route/profile 推导出的普通 request count。
根据 node kind 分配的 2/3/4 次经验请求额度。
为了控制 benchmark 成本而设置的正常执行截断。
通过 post-budget grace、fresh-node grace 等例外延后上述截断。
```

越界 gate 候选：

```text
阻止 Agent 继续验证/读取/编辑的语义策略 gate。
自动创建 rework node 并指示下一步策略的 gate。
根据 validation failure 推断必须如何修的 gate。
根据 coverage/fact_source 判断 Agent 必须读什么的 gate。
达到 route/profile 请求次数后停止正常 provider sampling 的 gate。
将 runtime 中断文本覆盖为 last_agent_message 并发出正常 task_complete 的出口。
```

执行拆分：

| Subphase | Scope | Exit Gate |
|---|---|---|
| R5-E0 | 移除普通 route/profile request-count pre-dispatch hard stop；删除随之失去意义的 grace 决策分支；拆分 Agent completion、sampling interruption 和 external validation | 超过 profile 的正常、有进展请求继续执行；中断不生成 Agent final/正常 completion；被中断样本不进入 utility |
| R5-E1 | 移除 model-visible 策略性 recovery/sentinel text 和 next-valid-actions | blocked message 只含 hard reason 和机械状态，不含下一步策略 |
| R5-E2 | 为剩余拒绝建立 hard-gate classifier | 每个拒绝都可归类为状态机、协议、权限、安全或资源底线 |
| R5-E3 | action-contract fallback audit | action-contract 仅作为显式 fallback，不恢复默认语义策略层 |
| R5-E4 | 修复 provider history 中 stale active projection 累积；只保留 latest projection，保留当前 tool/gate feedback；补 uniqueness scanner | 每个 TaskSpace provider payload 恰好一份最新 active projection；旧 running 与新 completed 状态不再并存；同一样本 token 增长显著下降或残差有独立证据 |

R5-E4 为 2026-07-10 插入并已关闭的阻断门。`target/r5e-phase-e-final-clean/...` 虽证明 hard stop
已退场，但其 14 份 active projection 使成本与重复 finish 行为 `projection-tainted`，只保留为
修复前证据。修复后的 `target/r5e4-projection-latest-only/.../20260710-051931-572` 在 9 个
provider request 中均只有一份 active projection，standard/R5 均 solved，可以进入 R5-F。

R5-E0 实施边界：

| Work Item | Production Behavior | Verification Evidence |
|---|---|---|
| profile budget 降级 | `max_rollout_model_requests` / `max_model_requests_per_node` 只用于 trace、告警和成本分析，不参与正常 pre-dispatch 拒绝 | focused test 在 `request_count > profile` 后仍允许下一次模型请求 |
| grace 逻辑退场 | `post_budget_grace`、fresh-node/feedback grace 不再为错误 hard stop 提供例外；不保留兼容分支 | active call graph 和测试证明正常执行不依赖 grace 才能继续 |
| emergency boundary | 不新增 TaskSpace 自主推断的异常策略；优先复用 Codex substrate 已有取消、显式限额、provider/进程错误。若必须保留 TaskSpace emergency stop，触发源必须显式、语义无关且独立可测 | 每个 emergency source 有稳定 `interruption_source` 和对应负例；普通 route count 永远不能触发 |
| completion 语义 | sampling 被外部严重异常中断时保留 Agent 原始最后消息，记录 `interrupted/incomplete`，不伪装 `task_complete` | rollout 事件顺序测试和 last-message fixture |
| benchmark 资格 | external validator 通过只表示 patch 结果有效，不自动表示 Agent 完整完成 | report 同时输出 Agent completion、interruption、external validation 和 utility eligibility |

R5-E0 日志和分类：

| Change Link | Success Signal | Failure Signal | Required Fields | Consumer |
|---|---|---|---|---|
| profile budget observation | `budget_action=observe` | profile counter 被用于 `allowed=false` | `request_count/profile_limit/route_mode` | benchmark/runtime audit |
| explicit interruption | `sampling_interrupted` | interruption source 缺失或来自 route profile | `interruption_source/reason/turn_id/task_id/node_id` | CLI/runtime audit |
| Agent lifecycle | `agent_completion_status=complete` | `interrupted` / `incomplete` | `completion_source/last_agent_message_source` | benchmark/report |
| validation lifecycle | `external_validation_status=passed|failed|skipped` | validation 与 Agent completion 混为单字段 | `validator_source/exit_code/run_id` | benchmark/report |

R5-E0 独立验收：

```text
1. 单测：verification_first 超过 6 次、node 超过 profile hint 后仍可继续正常 sampling。
2. 单测：普通 profile count 不产生 TaskSpaceProviderBudgetHardStopV1。
3. 单测：显式外部严重中断若存在，只记录 interrupted，不覆盖 Agent message，不发正常完成。
4. harness 测试：external validation passed + agent interrupted 不能归类为普通 solved/utility eligible。
5. live sample：count-call-stack 至少跑 standard/R5 各一次；无 profile hard stop，Agent 是否完成按真实结果记录。
6. 历史 `target/r5d-ledger-deactivation/.../pair-001/right` 标记为 budget-interrupted/benefit-tainted，
   不再用于证明 R5-D 比 standard 更快或工具调用更少。
```

R5-E0 收益验证：

| Benefit Hypothesis | Baseline | Target | Measurement | Pass / Fail Threshold |
|---|---|---|---|---|
| 正常执行不再被 profile 截断 | R5-D 在成功 edit 后 7/6 hard stop | profile-triggered hard stop 为 0 | focused continuation tests + `count-call-stack` rollout | 任一 route/profile count 导致 `allowed=false` 即失败 |
| 评估不再把中断误判为收益 | R5-D `business_success=true`，但 Agent 未验证/收尾 | completion/interruption/external validation 独立，interrupted utility eligibility 为 false | harness fixture + pair report fields | 任一 interrupted sample 进入普通 solved utility 即失败 |
| 不通过新语义策略控制成本 | 旧实现依赖 hard stop/grace | 无新增 action guidance、semantic gate 或 node-kind request cap | code review + provider-visible scan | 出现新增策略提示或语义 cap 即失败 |

R5-E0 审查、回退和下一门禁：

| Gate Condition | Evidence | Fallback | Proceed Decision |
|---|---|---|---|
| production path 不再调用 profile-count hard reject | call graph、focused tests | 暂停 E1，修复残留；不加兼容分支 | complete 后 proceed |
| interruption/completion/validation 分类完整 | harness tests、sample report | 标记样本 ambiguous/utility-ineligible；不得猜测完成 | complete 后 proceed |
| 成本或循环异常可由外部底线停止并被忠实记录 | substrate audit、timeout/cancel smoke | 暂停该样本并诊断上下文；不得恢复低 profile stop | complete 后 proceed |
| E0 代码和计划接受边界审查 | focused code review；可选用户授权后的对抗性审查 | 修复 finding 后重新过 E0 gate | complete 后 proceed to E1 |

退出门禁：

```text
普通 route/profile request count 不再进入 hard gate。
所有保留 gate 都能归类为状态机/协议/权限/安全/有明确来源的严重外部资源底线。
所有语义 gate 要么删除，要么变成忠实 event/note，不阻止 Agent 动作。
blocked message 不包含策略性纠错指令。
rollout/payload scan 不含 next_valid_actions、validation_needs_test、rejected_by_state_baseline 语义动作列表。
Agent completion、sampling interruption、external validation 三种状态不可互相覆盖。
每个 provider payload 中 active projection 数量必须等于 1；旧 projection 必须以 stale replacement reason 被排除。
```

负收益防线：

```text
删除 gate 后若出现循环，优先检查上下文 event/ref 是否丢失或扭曲；
不得第一反应重新加 runtime 语义约束。
若成本异常，先依赖用户显式限额、provider/进程底线和观测告警；不得恢复低 profile hard stop。
```

## 1.12 Phase R5-F：死代码清理和模块拆分

目标：

把旧的大型 runtime 混合逻辑拆成清晰模块，并删除不再被 active path 使用的旧结构。
R5 不做历史数据兼容，不保留 legacy adapter。

建议边界：

```text
map_state.rs        只管理 task/map/node/edge/status
node_events.rs     只管理事件、工具反馈、raw refs
state_machine.rs   只做硬规则校验
projection.rs      只渲染薄上下文和 omission audit
```

退出门禁：

```text
active provider path 不依赖旧 semantic ledger。
projection 代码不能调用语义 gate/cognitive coverage helper。
单元测试覆盖模块边界。
不存在 legacy read/compat adapter 或双写路径。
```

负收益防线：

```text
拆文件不能改变行为；行为变化必须已经在 R5-B/C/D/E 对应 phase 验证。
```

## 1.13 Phase R5-G：回归和收益门禁

目标：

证明简化没有带来明确负收益，并记录真实收益/代价。

Phase C 后，R5-G 必须把 Agent lifecycle、correctness、semantic cleanliness、request cadence
分开验收。R5-E0 未完成或 `agent_completion_status != complete` 的样本只能作为诊断证据：

| Dimension | Gate | Acceptable Result |
|---|---|---|
| Agent lifecycle integrity | completion/interruption/event-source audit | Agent final、runtime interruption、external validation 独立记录；interrupted sample 不进入 utility |
| correctness | targeted samples standard/R5 对照 | R5 无明确 correctness 回退；失败先归因上下文/反馈 |
| semantic cleanliness | provider-visible scan | 无策略提示、无 action-class contract、无 old recovery hints |
| feedback fidelity | tool result event/ref audit | stdout/stderr/exit/path/ref 可恢复，不主观摘要成策略 |
| request cadence | request/tool/action metrics | 记录真实放大来源；不要求一次性解决，但必须解释 |
| cost regression | token/projection metrics | projection tokens 不因重复旧结构无解释增长 |

优先样本：

```text
count-call-stack:
  native tool loop、projection boundary、patch + validation。

sqlite-db-truncate / H203 path case:
  局部路径文本不被 canonical truth 放大。

large-output-ref-smoke:
  ref/crop 忠实传递，不回退到语义摘要。
```

样本矩阵：

| Sample Class | Purpose |
|---|---|
| simple single-file success | 确认 TaskSpace 不拖累简单任务 |
| tool failure feedback | 确认失败语义进入 Agent 上下文 |
| H203/H204 path case | 确认局部路径文本不再被固化放大 |
| large output/ref | 确认裁剪透明且成本受控 |
| one multi-node task | 确认 map 图式组织仍有价值 |

指标：

```text
business_success
public_validation_exit_code
agent_completion_status
agent_completion_source
sampling_interrupted
interruption_source
external_validation_status
utility_eligible
tool_call_count
model_request_count
input_tokens / cached_tokens
wall_time_ms
provider-visible feedback completeness
projection_strategy_hint_count
semantic_gate_block_count
raw_ref_recoverability
```

退出门禁：

```text
projection_strategy_hint_count = 0。
semantic_gate_block_count 只包含硬底线分类。
工具失败反馈完整性不低于 R4。
无明确 correctness 回退；若有回退，必须能证明不是上下文丢失/扭曲。
所有收益统计样本 `agent_completion_status = complete` 且 `sampling_interrupted = false`。
`business_success = true` 不能单独满足 Agent 完整完成或 utility eligibility。
```

## 1.14 Phase R5-H：收口

目标：

完成 R5 closeout，明确剩余复杂度和后续版本入口。

交付物：

```text
R5 closeout 文档
最终架构边界图
已删除/降级结构列表
保留复杂结构列表和保留理由
测试和 benchmark 证据索引
后续 R6 backlog 或回到正常 v0.0.5 release path 的建议
```

退出门禁：

```text
文档、代码、测试、样本证据一致。
没有未提交改动。
所有保留的旧结构都有 owner 和删除条件。
```

## 1.15 Phase 依赖和门禁矩阵

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| R5-A | 静态审计、baseline artifact | 不依赖 R5-B contract | `02-r5-phase-a-current-state-inventory.md` | 100% 完成 | proceed to R5-B |
| R5-B | unit/fixture 证明 node event 归档 | 不依赖 thin projection 或 C0 cadence | event/ref 测试和 snapshot | 100% 完成 | proceed to R5-C0 |
| R5-C0 | 同口径 request/tool cadence 日志、focused paired sample | 不依赖 R5-C thin projection | cadence report、budget cliff 不复现 | 100% 完成 | pause |
| R5-C | provider-visible payload diff、action sequence live sample | 不依赖 ledger 删除 | thin projection diff、反馈完整性测试、`count-call-stack` paired run | 100% 完成 | proceed to R5-D |
| R5-C1 | native tool-loop focused sample、projection boundary scan | 不依赖 ledger 删除 | native alias tests、`count-call-stack` paired run、旧 action-class contract 扫描 | 100% 完成 | proceed to R5-D |
| R5-D | D0 provider-visible residue inventory、initial/state_commit 降级测试 | 不依赖 gate pruning | 语义残留分类表、ledger 非 active path 证据；D 阶段 live sample 因 profile hard stop 仅保留诊断资格 | 100% 完成 | proceed to R5-E0，不进入 benefit claim |
| R5-E0 | profile-over-limit focused tests、interruption/completion harness fixtures、`count-call-stack` live sample | 不依赖 E1/E2 文案清理或 R5-G | profile count 不再 hard stop；中断不伪装完成；样本资格分类正确 | 100% 完成 | proceed to R5-E1 |
| R5-E1/E2/E3 | gate 分类测试、payload scan、负例 | 不依赖模块拆分 | 仅硬底线 gate 列表；model-visible recovery/sentinel 无策略指令 | 100% 完成 | proceed to R5-E4 |
| R5-E4 | 双 projection provider-history 测试、exact payload uniqueness scan、同一样本重跑 | 不依赖模块拆分或语义压缩 | latest-only projection；当前 feedback pair 保留；token/payload 增长重新计量 | 100% 完成 | proceed to R5-F |
| R5-F | 模块边界测试、cargo check | 不依赖 benchmark 总跑 | active path import/call graph | 100% 完成 | pause |
| R5-G | targeted paired runs | 不依赖 closeout | 指标报告、失败分类 | 100% 完成 | pause |
| R5-H | closeout review | 无 | closeout 文档和 clean git | 完成 | pause |

### 1.15.1 每阶段样本验证规则

每个 R5 phase 必须选择 1 到 2 个适合本阶段改动的 sample，各执行 1 次，并横向记录：

```text
standard 当前版本
R4 历史基线或同样本重跑
R5 当前阶段版本
```

单次样本只作为 E1 诊断证据，不计入 utility aggregate，也不得把偶然成功写成收益定论。
在 R5-E0 完成前，任何出现 `TaskSpaceProviderBudgetHardStopV1`、runtime 生成 last message、
或 Agent 未完成但由外部 validator 判定成功的样本，一律标记为 `benefit-tainted`，不得比较
tool count、wall time、token 或 success-rate 收益。
每次记录必须包含 scenario、命令、run dir、pair report、standard/R4/R5 outcome、
Agent completion、sampling interruption、external validation、utility eligibility、tool count、
provider request count、state-machine action count、wall time、失败分类、
以及 feedback/event/ref 是否忠实透传。
若 R5 失败，第一优先级检查上下文语义是否丢失、扭曲、过度结构化或被预算/裁剪截断；
不得为了让样本通过新增 runtime 语义约束。

## 1.16 Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| R5-A inventory | 明确旧结构用途和拆除候选 | `core/src/action_map/*`, `tools/handlers/taskspace_control.rs` | docs/CoE | `02-r5-phase-a-current-state-inventory.md` | baseline artifact paths | none | landed |
| R5-B node events | 工具反馈忠实归档到 node | `NodeEvent` direct path | ordinary tools under TaskSpace | direct success/failure fixtures | node_event trace/ref | none | landed |
| R5-C0 cadence parity | patch 归一化不制造 malformed patch；预算 grace 只暂时越过 implement 首轮 cliff，不能阻止后续 profile hard stop | budget lifecycle accounting + action-contract patch normalization | whale exec taskspace mode | cadence focused tests, patch normalization test | provider request/tool/action metrics；R5-D hard stop 反证 | budget grace blocks final completion until E0 | landed, budget portion superseded by R5-E0 |
| R5-C thin projection and action sequence | model-visible 只含 map/node/events/refs；单次 response 可承载多个 Agent 明确动作；routing prompt report-only | projection renderer, action-contract parser/executor, benchmark routing prompt | provider request, whale exec taskspace mode | active_projection, taskspace_action_contract, routing harness tests | `target/r5cphase6/count-call-stack/20260709-183144-389` | none | landed |
| R5-C1 native tool loop boundary | DeepSeek 默认 native tools；机械空 map；projection 不再暴露 action-class contract | session turn transport, action_map runtime projection, tools router/parallel alias | whale exec taskspace mode | native alias tests, active_projection, taskspace_action_contract | `target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052` | none | landed |
| R5-D0 provider-visible residue inventory | provider-visible 旧语义标签和策略提示完成首轮清理；semantic ledger 仍待 D1/D2 降级 | projection/recovery/action-contract/gate recovery text | provider-visible context, ordinary tool feedback | `taskspace_action_contract`, `gate_recovery`, `active_projection`, D0 forbidden scan | `target/r5d0-semantic-residue-clean/count-call-stack/20260709-232508-447` | legacy/test-only `next_valid_actions` parser/helper remains internal | landed |
| R5-D1/D2 ledger deactivation | `initial_*`、`problem_ledger/cognitive_state` 不控制 active path | start_task/projection/closeout/final gate | taskspace_control, provider-visible context | `start_task_`, active projection, D2 closeout/final boundary tests, `taskspace_action_contract`, `gate_recovery` | `target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050` forbidden scan 无命中；TaskSpace side 被 profile hard stop，收益证据 tainted | legacy validation/rework tests still contain old semantic-control assertions | landed |
| R5-E0 request hard-stop removal | route/profile 请求计数只观测；真实中断不伪装 Agent completion；外部验证不覆盖 Agent 生命周期 | provider pre-dispatch gate、turn completion path、benchmark classifier | whale exec TaskSpace mode、paired report | profile-over-limit continuation、interruption semantics、harness eligibility tests | `target/r5e-phase-e-final-clean/count-call-stack/20260710-043411-389`：13 requests 后 Agent complete、无 hard stop、map closed | no compatibility/grace fallback | landed |
| R5-E1/E2/E3 hard-baseline pruning | 只保留硬底线拒绝；model-visible recovery/sentinel 不含策略指令；action-contract 不重解释 Agent 动作 | state machine gate path, sentinel/recovery renderer, native control parser, action-contract fallback | ordinary tool preflight, payload construction | gate classification、raw feedback、no-auto-rework、forbidden scan tests | E0-E3 live run forbidden marker 0；旧 scanner 的 replacement 结论已由 E4 废弃并重验 | 旧未调用 semantic helpers/tests 留给 R5-F 物理删除 | landed |
| R5-E4 active projection uniqueness | provider history 只保留 latest active projection；旧快照不与最新 map 状态并存；scanner 拒绝 projection count != 1 | provider-visible history composer、exact payload scanner、benchmark extractor/report | every TaskSpace provider request | 双 projection regression、current feedback pair、scanner uniqueness tests | 修复后 `target/r5e4-projection-latest-only/.../20260710-051931-572`：9/9 requests projection count=1；input 100365，wall 23649ms，均较污染样本显著下降 | 不做 projection 语义压缩或兼容分支 | landed |
| R5-F module split | map/event/gate/projection 边界清晰 | action_map modules | whale exec --taskspace | cargo check/test | trace fields stable | none | planned |
| R5-G benefit gate | 简化无明确负收益 | benchmark harness | targeted samples | paired report | metrics json/report | none | planned |

## 1.17 Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| node event record | recorded | `node_event.recorded` | `node_event.rejected` | `reason` | `task_id/map_id/node_id/event_id/call_id` | info/warn | runtime/debug |
| action cadence | request_started / action_executed | `taskspace.cadence.action_sequence_completed` | `taskspace.cadence.request_amplified` | `reason` | `provider_request_id/task_id/map_id/node_id/action_id` | info/warn | benchmark/R5 owner |
| raw ref creation | ref_created | `raw_ref.created` | `raw_ref.failed` | `error_code` | `event_id/ref_id/call_id` | info/error | runtime/debug |
| projection render | rendered | `projection.thin.rendered` | `projection.thin.failed` | `reason` | `projection_id/task_id/map_id/node_id` | info/error | benchmark/debug |
| profile budget | observed | `taskspace.profile_budget_observed` | `taskspace.profile_budget_became_control_gate` | `reason` | `provider_request_id/task_id/map_id/node_id` | info/error | benchmark/R5 owner |
| sampling interruption | interrupted | `taskspace.sampling_interrupted` | interruption source missing / route profile source | `interruption_source` | `turn_id/task_id/map_id/node_id` | warn/error | CLI/runtime audit |
| completion classification | classified | Agent/runtime/validator states independently recorded | `completion_state_ambiguous` | `reason` | `run_id/turn_id` | info/error | benchmark/report |
| hard gate reject | rejected | n/a | `state_machine.hard_reject` | `rule_id` | `task_id/map_id/node_id/call_id` | warn | Agent/debug |
| old semantic path removal | removed | `taskspace.old_semantic_path_removed` | `taskspace.old_semantic_path_still_active` | `old_path` | `task_id/map_id` | warn/error | R5 owner |

## 1.18 风险

| Risk | Probability | Impact | Trigger Signal | Mitigation | Fallback |
|---|---:|---:|---|---|---|
| 简化后 Agent 漏掉重要约束 | Medium | High | validation failure 增加 | 先查 event/ref 透传和 node objective | 恢复为 node note，不恢复 semantic gate |
| 删除 ledger 破坏旧测试 | High | Medium | fixture 大量失败 | 区分旧设计断言和新边界断言，删除或重写旧设计测试 | 回退本 phase commit 后重拆，不加兼容层 |
| projection 太薄导致反馈不可见 | Medium | High | Agent 重复低级错误 | 增加忠实 excerpt/ref，不加策略提示 | 回退 R5-C，不回退 R5-D/E |
| request cadence 观测口径不一致 | High | Medium | standard `provider_request_hook_coverage=0` | C0 先补同口径 telemetry，再比较收益 | 只报告 tool/action cadence，不声明 request parity |
| 为降低 request 放大而引入语义 batch | Medium | High | runtime 开始排序/合并/跳过 Agent action | batch 只执行 Agent 明确给出的合法 action | 回退 batch，保留 native tool-loop 方案 |
| gate pruning 放大循环 | Medium | Medium | repeated same action 增加 | 先诊断上下文是否丢失/扭曲 | 依赖用户取消、显式绝对预算、provider/进程底线，不加 TaskSpace 语义纠错 |
| 移除低请求 hard stop 后 benchmark 成本增长 | Medium | Medium | request/token/wall time 持续增长且无 completion | 使用现有用户取消、显式 benchmark timeout、provider/进程底线和观测告警；先诊断上下文反馈 | 停止该次 benchmark 并标记 interrupted，不恢复 route/profile hard stop |
| 历史收益结论被污染 | High | High | 样本有 hard stop/runtime last message 但仍标 solved | R5-E0 前统一 quarantine，重跑后再声明收益 | 撤销对应 benefit claim，只保留 patch correctness 事实 |
| 模块拆分引入行为回归 | Medium | Medium | cargo/test failure | 行为变化与拆文件分 phase | revert 单 phase commit |

## 1.19 第一批执行建议

R5-A 已按不改 runtime 的方式完成：

1. 已建立 active 语义结构清单。
2. 已从 R4 H203/H204、large-output、simple success 选出最小 baseline。
3. 已标出必须先保留的 replay/debug refs。
4. R5-B 的最小 `NodeEvent` contract 需要直接实现，不做兼容 overlay。

R5-B 已完成最小 `NodeEvent` 直接路径，并在 `count-call-stack` 中暴露 request cadence
blocker。R5-C0 已关闭预算 lifecycle blocker：fresh executable node 首轮请求与 post-budget
feedback grace 已分账，action-contract patch trailing-only End 归一化缺陷已修复。R5-C 已完成
thin projection、action sequence carrier、runtime feedback event、routing prompt report-only 和 thin
bootstrap 收敛；`count-call-stack` 单样本复验 standard/R5 均 solved。R5-C1 继续把默认路径切到
DeepSeek native tools，补齐 `exec_command`/`read_file` alias，允许 runtime 机械空 map 初始化，
并删除 active projection 中残留的 action-class contract；`count-call-stack` 复验 standard/R5
均 solved。Phase C/C1 暴露的问题已被单独归档到
`docs/v0.0.5/build-R5/07-r5-phase-c-exposure-followup-plan.md`。R5-D0 已完成首轮
provider-visible semantic residue inventory 和明显越界文案清理，记录在
`docs/v0.0.5/build-R5/08-r5-phase-d0-semantic-residue-inventory.md`；`count-call-stack`
D0 样本 standard/R5 均 solved，forbidden scan 无命中。R5-D1/D2 已完成，记录在
`docs/v0.0.5/build-R5/09-r5-phase-d-ledger-deactivation.md`：`initial_*` 不再提升为
canonical truth，ledger/cognitive state 不再控制 active projection、closeout、final response
或 broad delegation strategy；Phase D `count-call-stack` 的 patch correctness 和 forbidden scan
仍有效，但 TaskSpace side 在成功 edit 后被 profile hard stop，Agent 未自行验证/收尾，因此
tool ratio 0.79、wall ratio 0.66 和 harness solved 不再作为收益证据。R5-E0 至 E3 已移除普通
请求次数 hard stop、拆分生命周期并收清策略性 gate/recovery；E4 进一步修复 active projection
追加而非替换的问题。最终 `count-call-stack` 单样本 standard/R5 均 solved，R5 的 9 个 provider
payload 均只有一份最新 projection，输入 token 由污染样本的 269093 降至 100365，wall time
由 46971ms 降至 23649ms。下一步进入 R5-F，物理删除旧语义控制死代码和过时测试。

## 1.20 R5-A/B 后计划校准

| Finding | Plan Adjustment |
|---|---|
| `initial_*` 会把局部任务文本提升为结构化 fact/source/contract | R5-D 拆出 D1，先关闭 canonical truth 提升 |
| active projection 混合 ledger、coverage、tool feedback、strategy hints | R5-C 已完成 thin projection，并把 routing prompt 改为 report-only；下一步 R5-D 继续处理 ledger/cognitive state 写入和 canonical truth |
| `NodeResult/TaskSpaceTraceEvent` 暴露了正确的 node-event 方向，但不应作为兼容层保留 | R5-B 直接实现最小 NodeEvent，并切断旧结构 active 依赖 |
| R5-B live sample 显示 TaskSpace 一小步一 provider request，`verification_first` 在 implement 前 hard stop；R5-D 又在成功 edit 后复现 7/6 hard stop | R5-C0 只延后了 cliff，没有修正抽象；R5-E0 移除普通 route/profile 请求 hard stop 和 grace 补丁，并隔离被中断样本；request cadence 继续作为观测项，不转成 runtime 约束 |
| R4 large-output/ref 是正向收益 | R5-B/C 必须保留 raw_ref/excerpt，不和语义 gate 一起删除 |
| gate 消息含策略性纠错 | R5-E0 先清除会污染执行/评估的普通请求 hard stop；E1 再清 model-visible guidance；E2 建立 hard-gate classifier |
| active projection 名为 replacement 但历史 composer 实际追加全部快照 | R5-E4 只按 item identity 机械保留最新 projection，当前 tool/gate feedback 原样保留；scanner 强制 `active_projection_count=1`，不做语义压缩或重写 |
