# R5 TaskSpace 简洁模型分阶段收敛计划

> 本计划从 `docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md` 派生。
> R5 的每个 phase 都必须证明：拆掉过度设计没有引入负向收益，且 TaskSpace 更接近
> `standard` 自然上下文的图化/状态机化再组织。

## 1.1 元数据

```text
Created: 2026-07-09
Updated: 2026-07-09
Version: v0.0.5 build-R5
Status: Draft
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, action_map runtime, taskspace_control,
  context projection, provider-visible context, benchmark harness
Related Links:
  docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md
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
| 收敛 provider-visible projection | Agent 看到忠实上下文而非策略提示 | payload diff 证明 projection 只含 map、node context、events、refs |
| 拆除 active 语义账本控制路径 | 避免局部文本被固化为 truth | `initial_*` 不再生成强事实/合同/决策控制 |
| 保留工具反馈忠实性 | 不因为简化丢失 failure semantics | targeted samples 检查 stderr/exit/path/ref 可见 |
| 防止负向收益 | 简化后不明显劣于 R4 正向样本 | paired metrics 对比 correctness、tool count、request count、tokens |

非目标：

```text
不在 R5 追求新的 benchmark 分数峰值。
不把失败样本通过 runtime 禁止动作来做 pass。
不要求一次性删除所有旧结构；必须先证明替代路径稳定。
不重写整个 Codex upstream substrate。
```

## 1.4 约束和假设

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| TaskSpace 当前核心代码集中在 `action_map/runtime.rs` 和 `taskspace_control.rs` | R5-A 代码审计 | 先补模块边界图，再进入拆除 |
| R4 正向样本可作为回归防线 | R5-A 基线复用或重跑 | 选取更小 targeted baseline，不进入移除 phase |
| 简洁模型能保留必要 replay/debug refs | R5-B event/ref schema 验证 | 暂缓删除旧结果结构，只做 projection 降级 |
| Agent 能承担语义判断 | paired sample 观察 | 优先检查上下文透传，不回退到 runtime 策略控制 |

## 1.5 Phase 总览

| Phase | Theme | Main Output | Exit Gate |
|---|---|---|---|
| R5-A | Current-state inventory and baseline | 过度设计清单、活跃代码路径、R4 正负样本基线 | 每个复杂结构有 owner、active path、保留/降级/删除候选 |
| R5-B | Minimal map/event contract | 最小 TaskSpaceMap / NodeEvent 契约 | 工具结果可按 node 忠实归档并可 ref 读取 |
| R5-C | Projection thin mode | active projection 改为 map skeleton + current node + events/refs | provider-visible diff 无策略提示、无语义重写 |
| R5-D | Semantic ledger deactivation | 降级 `problem_ledger/cognitive_state/initial_*` active 控制权 | 局部任务文本不再变成 canonical truth |
| R5-E | Runtime gate pruning | 删除/降级越界语义 gate 和 next-action guidance | 拒绝只保留硬状态机/协议/安全底线 |
| R5-F | Compatibility cleanup and code split | 旧结构隔离、模块拆分、兼容读路径 | 生产路径不依赖旧语义控制，代码边界清楚 |
| R5-G | Regression and benefit gate | 正向/负向样本对照、成本和语义传递报告 | 不引入明确负收益，失败可解释 |
| R5-H | Closeout | R5 收口报告和后续 backlog | 文档、测试、代码、证据一致 |

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
旧 NodeResult 可暂时作为 event 后端，不能先删再补。
```

## 1.8 Phase R5-C：Projection thin mode

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

## 1.9 Phase R5-D：语义账本降级

目标：

把 `problem_ledger`、`cognitive_state`、`facts`、`decisions`、`fact_sources`、`output_contracts` 从
active runtime 控制路径降级。

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

## 1.10 Phase R5-E：runtime gate 修剪

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

## 1.11 Phase R5-F：兼容清理和模块拆分

目标：

把旧的大型 runtime 混合逻辑拆成清晰模块，避免再次把 projection、gate、ledger、工具反馈缠在一起。

建议边界：

```text
map_state.rs        只管理 task/map/node/edge/status
node_events.rs     只管理事件、工具反馈、raw refs
state_machine.rs   只做硬规则校验
projection.rs      只渲染薄上下文和 omission audit
legacy_ledger.rs   只读兼容旧数据，不参与 active 控制
```

退出门禁：

```text
active provider path 不依赖 legacy semantic ledger。
projection 代码不能调用语义 gate/cognitive coverage helper。
单元测试覆盖模块边界。
```

负收益防线：

```text
拆文件不能改变行为；行为变化必须已经在 R5-B/C/D/E 对应 phase 验证。
```

## 1.12 Phase R5-G：回归和收益门禁

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

## 1.13 Phase R5-H：收口

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

## 1.14 Phase 依赖和门禁矩阵

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| R5-A | 静态审计、baseline artifact | 不依赖 R5-B contract | 结构清单、样本基线 | 100% 完成或记录 residual risk | pause |
| R5-B | unit/fixture 证明 node event 归档 | 不依赖 thin projection | event/ref 测试和 snapshot | 100% 完成 | pause |
| R5-C | provider-visible payload diff | 不依赖 ledger 删除 | thin projection diff、反馈完整性测试 | 100% 完成 | pause |
| R5-D | initial/state_commit 降级测试 | 不依赖 gate pruning | ledger 非 active path 证据 | 100% 完成 | pause |
| R5-E | gate 分类测试和负例 | 不依赖模块拆分 | 仅硬底线 gate 列表 | 100% 完成 | pause |
| R5-F | 模块边界测试、cargo check | 不依赖 benchmark 总跑 | active path import/call graph | 100% 完成 | pause |
| R5-G | targeted paired runs | 不依赖 closeout | 指标报告、失败分类 | 100% 完成 | pause |
| R5-H | closeout review | 无 | closeout 文档和 clean git | 完成 | pause |

## 1.15 Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| R5-A inventory | 明确旧结构用途和拆除候选 | `core/src/action_map/*`, `tools/handlers/taskspace_control.rs` | docs/CoE | audit doc + baseline commands | baseline artifact paths | none | planned |
| R5-B node events | 工具反馈忠实归档到 node | new/refactored node event path | ordinary tools under TaskSpace | direct success/failure fixtures | node_event trace/ref | none | planned |
| R5-C thin projection | model-visible 只含 map/node/events/refs | projection renderer | provider request | payload snapshot/diff | omission audit | none | planned |
| R5-D ledger deactivation | semantic ledger 不控制 active path | state_commit/start_task handling | taskspace_control | initial_* and state_commit tests | state update traces | legacy read only | planned |
| R5-E gate pruning | 只保留硬底线拒绝 | state machine gate path | ordinary tool preflight | gate classification tests | blocked reason taxonomy | none | planned |
| R5-F module split | map/event/gate/projection 边界清晰 | action_map modules | whale exec --taskspace | cargo check/test | trace fields stable | none | planned |
| R5-G benefit gate | 简化无明确负收益 | benchmark harness | targeted samples | paired report | metrics json/report | none | planned |

## 1.16 Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| node event record | recorded | `node_event.recorded` | `node_event.rejected` | `reason` | `task_id/map_id/node_id/event_id/call_id` | info/warn | runtime/debug |
| raw ref creation | ref_created | `raw_ref.created` | `raw_ref.failed` | `error_code` | `event_id/ref_id/call_id` | info/error | runtime/debug |
| projection render | rendered | `projection.thin.rendered` | `projection.thin.failed` | `reason` | `projection_id/task_id/map_id/node_id` | info/error | benchmark/debug |
| hard gate reject | rejected | n/a | `state_machine.hard_reject` | `rule_id` | `task_id/map_id/node_id/call_id` | warn | Agent/debug |
| legacy ledger fallback | fallback_used | `legacy_ledger.read_compat` | `legacy_ledger.active_dependency` | `legacy_field` | `task_id/map_id` | warn/error | R5 owner |

## 1.17 风险

| Risk | Probability | Impact | Trigger Signal | Mitigation | Fallback |
|---|---:|---:|---|---|---|
| 简化后 Agent 漏掉重要约束 | Medium | High | validation failure 增加 | 先查 event/ref 透传和 node objective | 恢复为 node note，不恢复 semantic gate |
| 删除 ledger 破坏旧测试 | High | Medium | fixture 大量失败 | 分阶段降级，先兼容读 | 保留 legacy adapter |
| projection 太薄导致反馈不可见 | Medium | High | Agent 重复低级错误 | 增加忠实 excerpt/ref，不加策略提示 | 回退 R5-C，不回退 R5-D/E |
| gate pruning 放大循环 | Medium | Medium | repeated same action 增加 | 先诊断上下文是否丢失/扭曲 | 加硬资源底线，不加语义纠错 |
| 模块拆分引入行为回归 | Medium | Medium | cargo/test failure | 行为变化与拆文件分 phase | revert 单 phase commit |

## 1.18 第一批执行建议

建议从 R5-A 开始，不直接改 runtime：

1. 建立 active 语义结构清单。
2. 从 R4 H203/H204、large-output、simple success 选最小 baseline。
3. 标出必须先保留的 replay/debug refs。
4. 产出 R5-B 的最小 `NodeEvent` contract 细案。

只有 R5-A 关闭后，才进入 R5-B 的生产代码修改。
