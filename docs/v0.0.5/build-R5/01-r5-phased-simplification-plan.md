# R5 TaskSpace 简洁模型分阶段收敛计划

> 本计划从 `docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md` 派生。
> R5 的每个 phase 都必须证明：拆掉过度设计没有引入负向收益，且 TaskSpace 更接近
> `standard` 自然上下文的图化/状态机化再组织。

## 1.1 元数据

```text
Created: 2026-07-09
Updated: 2026-07-09
Version: v0.0.5 build-R5
Status: In Progress - R5-C0 implemented and validated
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, action_map runtime, taskspace_control,
  context projection, provider-visible context, benchmark harness
Related Links:
  docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md
  docs/v0.0.5/build-R5/02-r5-phase-a-current-state-inventory.md
  docs/v0.0.5/build-R5/03-r5-phase-b-node-event-contract.md
  docs/v0.0.5/build-R5/04-r5-phase-c0-cadence-parity.md
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
| R5-C | Projection thin mode | active projection 改为 map skeleton + current node + events/refs | provider-visible diff 无策略提示、无语义重写 |
| R5-D | Semantic ledger deactivation | D1 降级 `initial_*`，D2 降级 `problem_ledger/cognitive_state` active 控制权 | 局部任务文本不再变成 canonical truth |
| R5-E | Runtime gate pruning | E1 清除策略性提示，E2 建 hard-gate classifier 并删除/降级语义 gate | 拒绝只保留硬状态机/协议/安全底线 |
| R5-F | Dead code cleanup and code split | 删除旧结构、模块拆分、移除兼容分支 | 生产路径不依赖旧语义控制，代码边界清楚 |
| R5-G | Regression and benefit gate | 正向/负向样本对照、成本和语义传递报告 | 不引入明确负收益，失败可解释 |
| R5-H | Closeout | R5 收口报告和后续 backlog | 文档、测试、代码、证据一致 |

### 1.5.1 Phase 验收和工程收益矩阵

| Phase | 验收标准 | 工程收益 | 度量 / 验证方法 |
|---|---|---|---|
| R5-A | 所有 active 复杂结构被标记为 `keep/thin/deactivate/delete/unknown`，且 `unknown` 有后续诊断 | 降低架构不确定性，避免盲删或继续堆补丁 | 结构清单覆盖 `state/projection/gate/tool feedback`；`unknown` 不允许进入删除 phase |
| R5-B | ordinary tool success/failure 都能归档为 node-local event，并保留 excerpt/ref | 提升反馈可追踪性，后续 projection 不必重新解释工具语义 | fixture 中 tool result -> node event attribution 覆盖率 100%；raw/ref 可恢复 |
| R5-C0 | 预算模型不再把状态机生命周期和反馈交付窗口互相误扣；action-contract patch 归一化不再制造双 `End Patch` | 降低 request lifecycle cliff，避免简单任务在 implement 前或 patch 归一化处失败 | `count-call-stack` R5 已进入 implement、执行 edit，并在单样本中与 standard 均 solved；standard/R5 request cadence 有同口径日志 |
| R5-C | active projection 只含 map skeleton、current node、events、refs、hard status | 减少上下文污染和策略注入，让 Agent 直接面对忠实反馈 | `projection_strategy_hint_count=0`；provider-visible payload diff 通过 |
| R5-D | `initial_*`、ledger、cognitive state 不再作为 active canonical truth 或语义 gate | 防止任务文本局部细节被 runtime 固化放大 | H203/H204 path case 中 `/app` 不再由 projection/ledger 强化；state_commit 不要求 facts/decisions/adoption |
| R5-E | 保留 gate 都能归类为状态机、协议、权限、安全或资源底线 | 清晰 runtime 边界，减少 Agent 被 runtime 纠错/引导 | `semantic_gate_block_count=0`；hard gate 分类测试通过 |
| R5-F | active path 不依赖旧 semantic ledger，模块边界测试通过 | 提升可维护性，降低 `runtime.rs` 混合职责继续扩张风险 | call/import 审计证明 projection 不调用 cognitive coverage helper；cargo check/test 通过 |
| R5-G | targeted paired runs 无明确 correctness 回退，成本无无解释放大 | 用样本证明简化不是单纯删功能，而是降低干扰且保留收益 | business success、tool/model request、tokens、wall time、feedback completeness 对比报告 |
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
provider budget 保护资源底线，但不能把已完成的状态转移截断成不可执行半成品。
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
| budget gate adjustment | hard stop 不在新 implement 节点 `node_request_count=0` 时立即截断已完成状态转移 |
| feedback invariant | 多 action/多 tool 结果仍逐条归档为 node-local `NodeEvent`，失败语义不丢 |

退出门禁：

```text
`count-call-stack` R5 当前阶段至少进入 implement 并执行一次合法 edit tool。
同口径日志显示 R5 不再是一小步一 provider request 的强制节奏。
standard/R4/R5 对比记录包含 provider request、tool call、state-machine action 三个维度。
没有新增“指导 Agent 少读/必须改”的 runtime 语义提示。
状态机硬约束仍生效：inspect 不能直接 edit，非法 action 仍被拒绝并忠实反馈。
```

负收益防线：

```text
如果 native tool-loop 难以稳定，先保留 action-contract 并修 budget lifecycle；
但不得把 C0 标记为完成，直到 request cadence 放大有可验证收敛。
若多 action 执行导致反馈丢失，回退到单 action，但保留 NodeEvent 证据和问题记录。
```

## 1.9 Phase R5-C：Projection thin mode

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

负收益防线：

```text
若 thin projection 导致 Agent 看不到工具失败细节，停止拆除，优先修 event/ref 透传。
```

## 1.10 Phase R5-D：语义账本降级

目标：

把 `problem_ledger`、`cognitive_state`、`facts`、`decisions`、`fact_sources`、`output_contracts` 从
active runtime 控制路径降级。

执行拆分：

| Subphase | Scope | Exit Gate |
|---|---|---|
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
start_task initial_* 不再把任务文本细节提升成 active canonical truth。
state_commit 不再要求 Agent 维护 facts/decisions/adoption 才能继续普通工作。
closeout 不依赖 runtime 的 accepted semantic facts。
```

负收益防线：

```text
如果某个 benchmark 依赖 output artifact contract，先把它表达为 node objective/event note，
不能回退到 runtime output_contract 语义 gate。
```

## 1.11 Phase R5-E：runtime gate 修剪

目标：

保留硬状态机底线，删除或降级语义干预 gate。

硬底线允许：

```text
无 task/node 归属时拒绝 ordinary tools。
节点状态非法时拒绝绑定或完成。
工具调用/结果配对非法时拒绝或报错。
权限、沙箱、安全、协议规则非法时拒绝。
输出过大时转 ref，但必须暴露 ref 和裁剪说明。
```

越界 gate 候选：

```text
阻止 Agent 继续验证/读取/编辑的语义策略 gate。
自动创建 rework node 并指示下一步策略的 gate。
根据 validation failure 推断必须如何修的 gate。
根据 coverage/fact_source 判断 Agent 必须读什么的 gate。
```

执行拆分：

| Subphase | Scope | Exit Gate |
|---|---|---|
| R5-E1 | 移除 model-visible 策略性 recovery text 和 next-valid-actions | blocked message 只含 hard reason，不含下一步策略 |
| R5-E2 | 为剩余拒绝建立 hard-gate classifier | 每个拒绝都可归类为状态机、协议、权限、安全或资源底线 |

退出门禁：

```text
所有保留 gate 都能归类为状态机/协议/权限/安全/资源底线。
所有语义 gate 要么删除，要么变成忠实 event/note，不阻止 Agent 动作。
blocked message 不包含策略性纠错指令。
```

负收益防线：

```text
删除 gate 后若出现循环，优先检查上下文 event/ref 是否丢失或扭曲；
不得第一反应重新加 runtime 语义约束。
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
| R5-C | provider-visible payload diff | 不依赖 ledger 删除 | thin projection diff、反馈完整性测试 | 100% 完成 | pause |
| R5-D | initial/state_commit 降级测试 | 不依赖 gate pruning | ledger 非 active path 证据 | 100% 完成 | pause |
| R5-E | gate 分类测试和负例 | 不依赖模块拆分 | 仅硬底线 gate 列表 | 100% 完成 | pause |
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
每次记录必须包含 scenario、命令、run dir、pair report、standard/R4/R5 outcome、
tool count、provider request count、state-machine action count、wall time、失败分类、
以及 feedback/event/ref 是否忠实透传。
若 R5 失败，第一优先级检查上下文语义是否丢失、扭曲、过度结构化或被预算/裁剪截断；
不得为了让样本通过新增 runtime 语义约束。

## 1.16 Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| R5-A inventory | 明确旧结构用途和拆除候选 | `core/src/action_map/*`, `tools/handlers/taskspace_control.rs` | docs/CoE | `02-r5-phase-a-current-state-inventory.md` | baseline artifact paths | none | landed |
| R5-B node events | 工具反馈忠实归档到 node | `NodeEvent` direct path | ordinary tools under TaskSpace | direct success/failure fixtures | node_event trace/ref | none | landed |
| R5-C0 cadence parity | TaskSpace 不因一小步一请求在 implement 前或 patch 归一化处 hard stop | budget lifecycle accounting + action-contract patch normalization | whale exec taskspace mode | cadence focused tests, patch normalization test | provider request/tool/action metrics, `count-call-stack` paired report | none | landed |
| R5-C thin projection | model-visible 只含 map/node/events/refs | projection renderer | provider request | payload snapshot/diff | omission audit | none | planned |
| R5-D ledger deactivation | semantic ledger 不控制 active path | state_commit/start_task handling | taskspace_control | initial_* and state_commit tests | state update traces | none | planned |
| R5-E gate pruning | 只保留硬底线拒绝 | state machine gate path | ordinary tool preflight | gate classification tests | blocked reason taxonomy | none | planned |
| R5-F module split | map/event/gate/projection 边界清晰 | action_map modules | whale exec --taskspace | cargo check/test | trace fields stable | none | planned |
| R5-G benefit gate | 简化无明确负收益 | benchmark harness | targeted samples | paired report | metrics json/report | none | planned |

## 1.17 Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| node event record | recorded | `node_event.recorded` | `node_event.rejected` | `reason` | `task_id/map_id/node_id/event_id/call_id` | info/warn | runtime/debug |
| action cadence | request_started / action_executed | `taskspace.cadence.action_sequence_completed` | `taskspace.cadence.request_amplified` | `reason` | `provider_request_id/task_id/map_id/node_id/action_id` | info/warn | benchmark/R5 owner |
| raw ref creation | ref_created | `raw_ref.created` | `raw_ref.failed` | `error_code` | `event_id/ref_id/call_id` | info/error | runtime/debug |
| projection render | rendered | `projection.thin.rendered` | `projection.thin.failed` | `reason` | `projection_id/task_id/map_id/node_id` | info/error | benchmark/debug |
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
| gate pruning 放大循环 | Medium | Medium | repeated same action 增加 | 先诊断上下文是否丢失/扭曲 | 加硬资源底线，不加语义纠错 |
| 模块拆分引入行为回归 | Medium | Medium | cargo/test failure | 行为变化与拆文件分 phase | revert 单 phase commit |

## 1.19 第一批执行建议

R5-A 已按不改 runtime 的方式完成：

1. 已建立 active 语义结构清单。
2. 已从 R4 H203/H204、large-output、simple success 选出最小 baseline。
3. 已标出必须先保留的 replay/debug refs。
4. R5-B 的最小 `NodeEvent` contract 需要直接实现，不做兼容 overlay。

R5-B 已完成最小 `NodeEvent` 直接路径，并在 `count-call-stack` 中暴露 request cadence
blocker。R5-C0 已关闭该 blocker：fresh executable node 首轮请求与 post-budget feedback
grace 已分账，action-contract patch trailing-only End 归一化缺陷已修复，单样本复验
standard/R5 均 solved。下一步进入 R5-C thin projection，但仍需继续跟踪 action-contract
一步一请求的结构性成本。

## 1.20 R5-A/B 后计划校准

| Finding | Plan Adjustment |
|---|---|
| `initial_*` 会把局部任务文本提升为结构化 fact/source/contract | R5-D 拆出 D1，先关闭 canonical truth 提升 |
| active projection 混合 ledger、coverage、tool feedback、strategy hints | R5-C 做 thin projection，但必须排在 C0 cadence 收敛之后 |
| `NodeResult/TaskSpaceTraceEvent` 暴露了正确的 node-event 方向，但不应作为兼容层保留 | R5-B 直接实现最小 NodeEvent，并切断旧结构 active 依赖 |
| R5-B live sample 显示 TaskSpace 一小步一 provider request，`verification_first` 在 implement 前 hard stop | R5-C0 已先修复 budget lifecycle cliff 和 patch 归一化缺陷；完整 standard-like tool loop 仍留给后续 Phase C/E 评估 |
| R4 large-output/ref 是正向收益 | R5-B/C 必须保留 raw_ref/excerpt，不和语义 gate 一起删除 |
| gate 消息含策略性纠错 | R5-E 先清 model-visible guidance，再建立 hard-gate classifier |
