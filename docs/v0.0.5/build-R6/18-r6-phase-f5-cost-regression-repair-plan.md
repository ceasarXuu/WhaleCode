# R6 Phase F5 工具合同与成本回归修复计划

- Created: 2026-07-17
- Updated: 2026-07-17
- Version: 1.2
- Status: F5.0-F5.0b Complete / F5.0c Pending / F5.1-F5.3 Blocked
- Owner / Responsible: WhaleCode core runtime / TaskSpace
- Related Systems: `taskspace_control` schema、ToolRouter、Rooted DAG runtime、provider request composer、Docker benchmark
- Related Links: `01-r6-phased-implementation-plan.md`、`16-r6-phase-f-context-cost-plan.md`、
  `17-r6-phase-f-result.md`、`coe/2026-07-16-18-52-r6-phase-f-context-cost.md`
- Risk Level: High
- Plan Type: Full

## 1. 背景与状态修正

Phase F 的 F0-F4 已完成上下文分区观测、Map 当前状态单 owner、反馈保真、projection epoch 和严格参数解析，
但端到端成本门失败。Phase F 不能作为健康 R6-B0 进入 Phase G。

Phase E 到 Phase F final 的 R6 绝对回归为：

| Sample | Metric | Phase E | Phase F final | Change |
|---|---:|---:|---:|---:|
| simple | requests | 28 | 43 | +53.6% |
| simple | input | 213,502 | 428,351 | +100.6% |
| simple | uncached input | 20,862 | 66,367 | +218.1% |
| complex | requests | 43 | 58 | +34.9% |
| complex | input | 536,250 | 863,940 | +61.1% |
| complex | uncached input | 46,394 | 98,116 | +111.5% |

输入增量约 55% 来自更多 provider request，约 45% 来自每 request 更重。F5 必须同时处理两部分。

## 2. 已确认根因与证据门

| ID | 根因 | 状态 | F5 处理 |
|---|---|---|---|
| H-007 | 完整 lifecycle schema 固定为 26,628 B/request，但 named/auto/named choice break 仍存在 | confirmed | F5.1 |
| H-008 | bootstrap 合并进完整 schema 后降低合同显著性并生成非法 `finish.goal` | refuted；正式 A/B/C=6/5/6 | 不再作为 F5.1 修复依据 |
| H-009 | R6 丢失 R5 的非终态完成交接 carrier；F3 依赖 sibling calls，实际采用 0/6 | confirmed | F5.2 |
| H-010 | F0-F4 没有 Phase E 端到端成本收益门 | confirmed | F5.0/F5.3 |
| H-011 | Finish 使用对象类型诱发模型补齐 `goal` | refuted；E 对象 0/6 错误 | 不实施标量化修复 |
| H-012 | `finish`/`node_id` 与普通节点共享命名束诱发字段泛化 | confirmed；D=5/6、E=0/6 | F5.0c 切换 E 合同 |

F5 不重新调查已排除方向：projection count 始终为 1、semantic rewrite 为 0、F1 `map_state` 去重是净收益、
F3.5 epoch identity 不构成主要 provider-visible input。

## 3. 冻结的设计原则

1. `taskspace_control` 仍是唯一公开状态机工具，不增加 action-frame、planner 或第二个控制协议层。
2. 工具能力面只由 canonical hard state 机械派生；不按任务语义、关键词或 Runtime 策略隐藏工具。
3. 空 Map 必须初始化、无 binding 不能执行 ordinary tool、terminal 必须显式闭合，属于硬状态底线。
4. Agent 明确声明 complete、next binding、graph mutation、ordinary actions 和 final summary；Runtime 不选择 next。
5. 非终态完成不得成为无意义的 provider 流程断点；能继续、等待或结束必须在同一 control carrier 中明确区分。
6. complete 后不得继续使用旧 lease；后续 ordinary action 只能在 Agent 声明并成功建立的新 lease 下执行。
7. 状态交接原子提交，后续 ordinary action 失败不回滚已经提交的状态，但必须忠实返回失败和 skipped tail。
8. 不保留旧 `transition_node(complete)` 的 model-visible 兼容形态；实验产品直接切换并删除旧分支。
9. 不通过减少 Work 节点、Runtime 自动合并 Map、projection 语义裁剪或关闭 thinking 换取成本收益。
10. 每次只落一个策略，完成 simple/complex 各一次诊断后暂停汇报；未过本阶段门不得进入下一阶段。

### 3.1 复杂度与风险评估

本计划属于高风险工具合同重构：变更跨 provider schema、状态事务、ToolRouter、terminal 和 replay，但不迁移用户数据，
每个 phase 可按独立提交完整回退。最大风险不是数据兼容，而是 schema 已承诺的序列与 Runtime 实际事务不一致。

### 3.2 替代方案与取舍

| 方案 | 决策 | 原因 |
|---|---|---|
| 保留完整 immutable schema，只继续调缓存 | Rejected | choice break 仍存在，terminal 明确放大 uncached input |
| 用系统提示词鼓励 complete 后继续 | Rejected | R5 已证明 sibling call 存在性不能由单个 tool schema 约束 |
| Runtime 自动选择 next 并合并请求 | Rejected | 越过状态机底线，替 Agent 做规划 |
| 强制减少 Work 节点 | Rejected | 三个 Work 可能忠实表达读取、修复、验证，成本不能靠 Map 坍缩解决 |
| 整体恢复 R5 状态模型 | Rejected | R6 Rooted DAG 是唯一领域模型；只迁移已验证行为不变量 |
| 新增通用 action-frame/sequence 工具 | Rejected | 增加平行协议层；应演进现有 `taskspace_control` |

## 4. 外部依据

1. [DeepSeek Function Calling](https://api-docs.deepseek.com/guides/function_calling/)：strict mode 位于 beta endpoint，
   且服务端只接受其支持的 JSON Schema 子集；F5 不把 `strict=true` 或 `required+thinking` 当成未经验证的前提。
2. [JSON Schema Boolean Combination](https://json-schema.org/understanding-json-schema/reference/combining)：
   `anyOf` 表示至少一个分支匹配，`oneOf` 要求且仅要求一个分支匹配；F5 使用单值 action discriminator、独立
   required 和 `additionalProperties=false` 让行为形态互斥，不使用宽对象加提示词补约束。
3. [JSON Schema `$defs` 与 `$ref`](https://tour.json-schema.org/content/06-Combining-Subschemas/01-Reusing-and-Referencing-with-defs-and-ref)：
   公共结构通过本地定义复用，避免复制 ordinary tool schema；但不得以无约束 arguments 信封替代真实参数校验。
4. [OpenAI Chat Completions Tool Reference](https://platform.openai.com/docs/api-reference/chat/create)：strict tool schema
   只支持 JSON Schema 子集；F5 必须以当前 DeepSeek production probe 和本地 typed parser 为最终证据。

## 5. 目标与非目标

### 5.1 目标

1. provider-visible tools 与 hard state 一致，不再暴露当前必然被 Runtime 拒绝的顶层能力。
2. 恢复 R5 已验证的不变量：standalone nonterminal complete 在 model-visible schema 中不可表达。
3. complete 后的 next binding/end/wait 与后续动作由 Agent 在同一 control call 中声明。
4. 以证据支持的工具合同消除首次 bootstrap 参数错误；不得把 bootstrap-only 或描述强化直接计为修复。
5. simple/complex correctness、Map closure、terminal/replay、反馈保真保持 100%。
6. F5 final 的 requests、input、uncached input 和 1/5 cached-weighted input 均不劣于 Phase E R6 基线。

### 5.2 非目标

1. 不进入 Phase G 的节点详情折叠、长 Map 压缩或骨架超限方案。
2. 不要求 R6 在 F5 内低于 Standard；F5 只负责消除相对 Phase E 的明确回归。
3. 不解析 reasoning，不增加策略提示词，不由 Runtime 推断 Agent 下一步。
4. 不要求多个具有真实结果依赖的 ordinary actions 在同一 provider request 中预先声明。
5. 不修改 Hook；当前根因与 Hook 无关。

## 6. 依赖与假设

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek production function schema 行为 | third-party | H-008/H-011 已反证、H-012 confirmed | E 合同需要生产闭环验证 | F5.0c deterministic + live |
| Phase E/F4 frozen artifacts | data | Ready | provider 时间漂移影响历史绝对值 | 正式矩阵同时运行 current Standard，并轮换顺序 |
| Rooted DAG candidate/replay | system | Ready | composite handoff 可能产生部分状态 | F5.2 candidate preflight + fault injection |
| ToolRouter/permissions/sandbox | system | Ready | nested continuation 绕过原生能力链 | 强制复用现有 router/runtime |
| Docker paired harness | environment | Ready | 本机状态污染成本数据 | 所有 live gate 使用统一 Docker |

## 7. 总体技术设计

### 7.1 Hard-state 对齐的工具面

| Canonical state | Provider-visible surface | 允许 Agent 决策 |
|---|---|---|
| Bootstrap / empty Map | bootstrap-only `taskspace_control` | 声明完整初始 DAG、initial binding、continuation |
| Active binding | ordinary tools + active `taskspace_control` | 工作、改图、完成交接、阻塞、读取 Map |
| No binding / ready frontier | active `taskspace_control` | bind、rework、mutate、read/expand |
| Finish ready | terminal-capable `taskspace_control` | `finish_end`、rework、mutate、read/expand |

工具面只根据空 Map、binding 和 Finish ready 等机械状态变化，不根据 Runtime 对任务的语义判断变化。

### 7.2 非终态完成交接合同

F5.2 在设计门中冻结最小 discriminated variants，必须覆盖继续与结束两类结果：

```text
complete_then_continue
  -> complete Agent 指定的 current node
  -> apply Agent 声明的必要 graph mutation（可选）
  -> bind Agent 指定的 next node
  -> execute required non-empty continuation under the new lease

complete_then_end
  -> complete Agent 指定的 current node
  -> validate Finish ready
  -> atomically close Root + Finish and persist exact final summary
```

最终字段名由 F5.2 provider probe 冻结，但行为不变量不能退回 sibling-call 约定。只有确定性 branch/join fixture 证明
存在“当前节点必须完成、但没有可绑定 successor 且其他依赖仍运行”的自然状态时，才允许增加机械
`complete_then_wait` 变体；否则该变体不进入 schema。它不得接受策略性 reason，也不得触发无意义 recovery request。

### 7.3 事务边界

```text
parse complete carrier
  -> clone canonical ActionMap candidate
  -> validate complete + optional mutation + next binding/end/wait as one state plan
  -> persist ordered canonical state events atomically
  -> install candidate
  -> execute ordinary continuation through existing ToolRouter
  -> preserve raw results/output refs; first failure stops and marks tail skipped
```

state plan 失败时 revision、lease、node status、event store 均不变化。ordinary continuation 失败时已提交 handoff 不回滚，
反馈必须区分 `state_commit=true` 与 nested tool failure。

## 8. 分阶段执行

### Phase F5.0：重开门禁与 Bootstrap 因果 A/B

#### Result

Complete。正式 A/B/C 的 `finish.goal` 为 6/6、5/6、6/6，且唯一字段错误路径相同；H-008 被反证。结果见
`19-r6-phase-f5-0-bootstrap-ab-result.md`。bootstrap-only schema 只保留 H-007 的 hard-state/cost 价值，不能再用于
解释初始化正确性。

#### Objective

冻结 Phase E/F4 基线和 F5 outcome gate，完成 H-008 的同版本因果隔离；本阶段不修改 production 默认行为。

#### Entry Criteria

- COE H-007/H-009/H-010 confirmed；H-008 有 6/6 runtime evidence。
- Phase E/F4 artifacts 可重放，Docker/provider credential preflight 正常。

#### Implementation Tasks

1. 在 benchmark probe 中从同一生产 schema builder 构造三臂，不新增 production feature flag：
   - A：当前 full lifecycle schema + 当前通用 description；
   - B：bootstrap-only schema + 当前通用 description；
   - C：bootstrap-only schema + 明确 `Finish=node_id only` 的机械 description。
2. simple/complex 每臂各 3 次，固定模型、prompt、temperature、tool choice 和轮换顺序。
3. 记录首次 tool name、raw arguments、finish/root 字段、schema bytes、thinking、parse verdict 和 provider request。
4. 冻结 F5 final 成本公式：total、mean、median、request2+ cache、terminal uncached、`uncached + cached/5`。
5. 增加 R5 行为迁移清单：`finish_then_actions` 的工具合同必须映射到 R6 handoff，不以领域类型已迁移代替能力迁移。

#### Validation

| Type | Passing Standard |
|---|---|
| Diagnostic | B/C 至少一臂把首次 `finish.goal` 从 A 的高复现率降到 <=1/6，且无新字段错误 |
| Attribution | 明确区分 support、refutation 与 inconclusive，不以单次随机合法计为收益 |
| Refutation | A 高复现且 B/C 均 >=5/6 时，反证 schema breadth 与 description salience |
| Safety | probe 不改 production 默认 schema，不记录 secret/完整业务正文 |
| Baseline | Phase E/F4 指标与原 artifact 对账 100% |

#### Exit Criteria

- H-008 confirmed 或 refuted，并有对应 raw probe artifact；不允许以“模型随机”无证据结束。
- F5 outcome gate、比较臂和停止条件冻结。

#### Risks And Fallback

| Risk | Impact | Trigger | Fallback |
|---|---|---|---|
| provider 波动导致三臂都不稳定 | 无法归因 | 每臂错误分布无显著差异 | 增加重复，不进入 F5.1 bootstrap 修复 |
| probe schema 与生产 builder 漂移 | 假结论 | hash/bytes 无法对账 | probe 失败并修复观测，不修改 production |

#### Gate To Next Phase

F5.0 已暂停汇报。下一步进入 F5.0b；不得把 B/C 的成本下降解释成初始化正确性收益。

### Phase F5.0b：Finish Identity Wire Shape 因果隔离

#### Result

Complete。D/E/F identity error 为 5/6、0/6、1/6，公共字段错误均为 0；E 是获胜臂。H-011 被反证，
H-012 confirmed。结果见 `20-r6-phase-f5-0b-finish-identity-result.md`。

#### Objective

只验证 Finish identity 的 JSON wire shape 是否导致 `goal` 泛化；不修改 production，不增加提示词或 Runtime 纠错。

#### Experiment

1. 从 F5.0 的 bootstrap-only schema 机械派生三臂，除 Finish identity 线形态外保持 description、prompt、模型、
   `temperature` 和 named tool choice 相同：
   - D：当前 `finish: { node_id }`；
   - E：`finish_identity: { id }`，保留对象但改变 identity 命名束；
   - F：`finish_identity: string`，保持 E 的外层字段，只把对象改为标量 Finish ID。
2. simple/complex 每臂各 3 次，沿用 F5.0 轮换、脱敏、hash、cache 与 parse 观测。
3. 不把“标量天然无法容纳 goal”直接算成功；同时检查 action、Root、initial Work、edges、continuation 是否新增错误。

#### Exit Criteria

- D 必须高复现当前错误；E/F 至少一臂将非法字段或类型错误降到 <=1/6，且无其他字段回归，才能确认
  identity wire contract 根因，并按对象/命名/标量结果拆分假设。
- 三臂都失败则保持 investigating，暂停并重新建假设，不进入生产修复。
- 完成后暂停汇报。

### Phase F5.0c：Finish Identity 合同切换

#### Objective

将 E 臂 `finish_identity: { id }` 一次性切换到生产 schema、typed parser、mapping、event/replay 和 observer；
不保留旧 `finish: { node_id }` 兼容分支。canonical domain 内部仍保存 Finish node ID，Runtime 不解释或改写 ID。

#### Entry Criteria

- F5.0b 18/18 请求有效，E=6/6 strict-valid，common error=0；
- H-012 evidence gate satisfied；E schema 仅比 D 增加 8 bytes；
- 回滚点为 `63470f124`，无用户数据迁移要求。

#### Validation

- schema/parser 正反 fixture、terminal/replay、malformed feedback 全部通过；
- simple/complex 各 1 次，首次初始化错误为 0/2，Map/Root/Finish/continuation 100%；
- 只验证初始化正确性与该项 schema 成本，完成后暂停，不与 F5.1 工具面收益合并报告。

### Phase F5.1：Hard-state 对齐 Tool Surface

#### Objective

删除负收益的 immutable full-lifecycle 暴露，让工具面与 hard state 一致，同时保留 thinking 和 Agent 决策权。

#### Entry Criteria

- F5.0c 完成，E wire contract 已通过 deterministic 与 live 初始化验证。
- bootstrap-only 只作为 hard-state 对齐，不承担 H-008 已反证的正确性归因。
- F4 commit 和回滚点已记录。

#### Implementation Tasks

1. 恢复 bootstrap/active/no-binding/terminal 的机械 registry plan；删除 F2 永久暴露全部 13 tools 的路径。
2. bootstrap 顶层只暴露 bootstrap control；ordinary continuation 仍通过同一 ToolRouter 执行。
3. no-binding/terminal 不暴露必然触发 hard-state reject 的普通顶层工具。
4. `update_plan` 在 TaskSpace 全生命周期继续隐藏。
5. 不启用 `required+thinking`；保留 provider 已验证可用的 named/auto 选择。
6. schema builder 保持单一 owner；阶段形态复用 `$defs`，不复制普通工具参数合同。
7. 删除 immutable contract 专属测试和错误文案，不留兼容分支。

#### Deliverables And Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime Evidence | Status |
|---|---|---|---|---|---|
| state tool surface | `session/turn.rs` registry/visibility | provider prompt build | mode matrix | tools hash/count/bytes | planned |
| bootstrap contract | `tools/taskspace_tool.rs` | named bootstrap request | schema positive/negative | first-call args | planned |
| terminal surface | same registry plan | Finish-ready request | terminal/rework fixtures | terminal tools bytes | planned |

#### Validation

- deterministic state/tool visibility matrix 100%。
- simple/complex 各 1 次 Standard/F4/F5.1；public/hidden、Map、terminal、replay 100%。
- model-visible ordinary tool hard-state reject：bootstrap=0，terminal=0。
- tools bytes：bootstrap/terminal 均低于 F4 的 26,628 B；terminal uncached 低于 F4 同样本。
- 若 input/request 任一样本高于 F4，暂停，不进入 F5.2。

#### Risks And Fallback

| Risk | Impact | Trigger | Fallback |
|---|---|---|---|
| tools shape break 增加 | cache 下降 | weighted input 高于 F4 | 整体回退 F5.1，不保留双 schema |
| terminal 无法 rework/read | Agent 被错误限制 | 合法 control variant 不可见 | 修正同一 active control schema后重测 |

#### Gate To Next Phase

完成单样本收益报告后暂停；correctness、hard-state alignment、input 三门全部通过才进入 F5.2。

### Phase F5.2：恢复 Agent 声明的 Complete Handoff Carrier

#### Objective

在 Rooted DAG 上恢复 R5 schema-first 不变量，消除 standalone nonterminal complete 和 sibling-call 依赖。

#### Entry Criteria

- F5.1 100% 完成；工具能力面已经与 hard state 一致。
- R5 `finish_then_actions` 与 R6 transition/revision/lease 的迁移映射通过设计评审。

#### Implementation Tasks

1. 先做最小 provider schema probe，冻结 `complete_then_continue/end` 的 discriminated 参数形态；
   `complete_then_wait` 受 branch/join 必要性证据门控制。
2. 从 active model-visible schema 删除 standalone `transition_node(complete)`；不保留 parser alias。
3. 在 Rooted DAG domain 增加 Agent-declared handoff candidate/preflight，不在 session loop 组合语义。
4. complete + graph mutation + next bind 的 state plan 原子提交；stale revision 或非法 next 零部分提交。
5. continuation 只在新 lease 下通过现有 ToolRouter 执行；保留权限、沙箱、取消、单 patch 和 output-ref 行为。
6. final Work 使用 `complete_then_end` 直接进入 Phase E terminal durable envelope，不新增 provider request。
7. 用并发 DAG fixture 判断是否需要 `complete_then_wait`；没有自然必要性证据则不实现。
8. 删除 F3 “complete 后依赖 sibling calls”的 description、测试和 cadence 假设。

#### Logging And Observability

| Change Link | Key State | Success Signal | Failure Signal | Reason Field | Correlation | Level |
|---|---|---|---|---|---|---|
| schema parse | variant/ids/revision | `taskspace.handoff_parsed` | existing arguments rejected | `error.code/path` | outer call id | debug/warn |
| state preflight | candidate revisions | `taskspace.handoff_preflight_passed` | zero-commit reject | `violation_code` | call/map/revision | info/warn |
| state commit | complete/next/end | `taskspace.handoff_committed` | persistence failure | `failure_stage` | call/map/revision | info/error |
| continuation | new lease/action index | existing tool result | nested failure/skipped | native reason | parent/derived call id | existing levels |

日志不记录工具参数正文、patch、summary、API key 或完整输出。

#### Validation

| Type | Passing Standard |
|---|---|
| Schema | standalone complete 不可表达；continue/end 正负例全部通过；wait 按必要性证据明确实现或移除 |
| State | stale/illegal next/persistence crash 均 `partial_commit=0` |
| Router | nested ordinary result 与独立原生调用反馈等价，权限/沙箱不绕过 |
| Terminal | final Work + end 只产生一个 carrier，Root/Finish/replay hash 一致 |
| Benefit | eligible handoff adoption=100%，standalone nonterminal complete=0 |
| Live | simple/complex 各 1 次，F5.2 request/input 不高于 F5.1 |

#### Risks And Fallback

| Risk | Impact | Trigger | Fallback |
|---|---|---|---|
| composite state 部分提交 | Map 损坏 | fault injection hash/revision 不一致 | 回退 F5.2 整组提交 |
| provider 不生成 discriminated carrier | 重试增加 | probe/live 参数错误 | 停留 schema probe，不加提示词或 Runtime fallback |
| wait 形态被滥用 | 无效停顿 | 有 ready successor 仍 wait | hard-state reject，反馈机械事实 |

#### Gate To Next Phase

全部 deterministic、fault injection 和两个 live sample 通过后暂停；不得用 F5.3 补证 F5.2。

### Phase F5.3：正式成本门与 Phase F 重新收口

#### Objective

证明修复消除了明确回归，再冻结新的 R6-B0；未达到 Phase E 成本门则继续停留 F5。

#### Entry Criteria

- F5.0-F5.2 独立门全部通过。
- production 无诊断 flag、旧 schema alias、双 runtime 或 silent fallback。

#### Formal Matrix

统一 Docker、同 model/prompt/validator/oracle，轮换顺序，每个样本每臂 3 次：

| Sample | Arms |
|---|---|
| `single-file-fast-fix` | current Standard / frozen Phase E / frozen F4 / F5 candidate |
| `subscription-billing-repair` | current Standard / frozen Phase E / frozen F4 / F5 candidate |
| branch-join | current Standard / frozen F4 / F5 candidate |

冻结臂使用各自 attested binary 和原生协议，不向 production 增加兼容代码。

#### Exit Criteria

```text
public/hidden correctness = 100%
finish_end or complete_then_end adoption = 100%
Root/Finish closed and raw terminal hash = replay hash = 100%
semantic rewrite = 0; projection authoritative section = 1/request
bootstrap first-call finish.goal/root.goal error = 0/6
model-visible hard-state impossible ordinary calls = 0
standalone nonterminal complete = 0
eligible complete handoff adoption = 100%
partial state commit = 0

simple and complex F5:
  total requests <= Phase E
  total input <= Phase E
  total uncached input <= Phase E
  uncached + cached/5 <= Phase E
  request median <= Phase E median
```

任何一项失败都不能把 Phase F 标记完成。branch-join 不要求粗化 Map，验收依赖、frontier、wait/handoff 和状态正确性。

#### Review And Closeout

1. 更新 `17-r6-phase-f-result.md`，区分 F0-F4 机制结果和 F5 outcome 结果。
2. 更新 COE H-007/H-008/H-009/H-010/H-011/H-012，只有 fix-validation evidence 完整才关闭。
3. 运行 forbidden symbol、dead branch、兼容 alias、provider-visible strategy text 扫描。
4. 经用户单独授权后才执行对抗性审查；审查发现问题回到对应 F5 phase。
5. F5 全门通过后才能冻结 R6-B0 并进入 Phase G。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Required | Decision |
|---|---|---|---|---:|---|
| F5.0 | provider A/B + artifact replay | 不依赖 production repair | H-008 verdict + frozen baseline | 100% | pause/report |
| F5.0b | Finish wire-shape provider A/B | 不依赖 production repair | H-011/H-012 verdict | 100% | pause/report |
| F5.0c | Finish identity production switch | F5.0b | typed contract + live init | 100% | pause/report |
| F5.1 | state visibility fixtures + 2 live | 不依赖 handoff | hard-state alignment + tools/input report | 100% | pause/report |
| F5.2 | schema/router/fault + 2 live | 不依赖 formal matrix | handoff adoption + atomicity report | 100% | pause/report |
| F5.3 | 3-sample formal matrix | 不依赖 Phase G | correctness + Phase E cost gate | 100% | close F / enter G |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime Evidence | Mock / Stub | Status |
|---|---|---|---|---|---|---|---|
| bootstrap A/B | 反证 schema/description | benchmark probe only | provider probe | 3-arm fixtures | raw args/verdict | diagnostic-only | completed |
| Finish identity A/B | 归因 wire shape | benchmark probe only | provider probe | 3-arm fixtures | raw args/verdict | diagnostic-only | completed |
| Finish identity contract | `finish_identity: { id }` | tool/args/mapping/replay | control handler | schema/parser/replay | init trace | none | planned |
| state tool surface | 只暴露 hard-state 合法能力 | session/tool registry | provider prompt | mode matrix | tools hash/bytes | none | planned |
| handoff schema | standalone complete 不可表达 | taskspace tool/args | tool call | schema/parser | adoption trace | none | planned |
| handoff domain | complete+next 原子 | rooted DAG runtime | control handler | property/fault | revision/hash | none | planned |
| native continuation | 新 lease 下执行 | ToolRouter/sequence | nested call | permission/sandbox | call lineage | none | planned |
| benefit gate | 不劣于 Phase E | benchmark observer | Docker runner | report selftest | formal matrix | none | planned |

## 11. 回退策略

1. 每个 F5 phase 独立 commit；phase 门失败整体回退该 phase，不跨 phase 摘取半套 schema/runtime。
2. 不创建长期 feature flag、兼容 parser 或双写 Event Store。
3. F5.2 schema 与 domain/runtime 必须同组回退，禁止 model-visible 承诺与执行能力不一致。
4. 回退后恢复上一个 attested binary，并重跑该 phase 的 simple/complex smoke，确认 workspace 和 replay 未污染。
5. Phase G 在 F5.3 通过前保持 blocked，不使用压缩策略掩盖回归。

## 12. 权限、安全与发布边界

1. nested continuation 必须复用现有 ToolRouter、approval、sandbox、cancellation 和 output-ref，不新增旁路执行器。
2. schema probe 和日志只记录 tool/schema hash、字段路径、字节和 verdict，不记录 API key、命令正文、patch 或文件正文。
3. F5 不创建长期 feature flag；诊断探针在 F5.0 结束后只保留可复用 benchmark，不进入 production prompt path。
4. 每个生产 phase 完成后构建 attested `whale` binary；旧 attested binary 是回退载体，不保留运行时兼容分支。
5. 本产品当前为实验性 CLI，不做线上流量 canary；Docker formal 是默认路径切换前的 release gate。

## 13. 开放问题

| Question | Resolution Phase | Blocking Rule |
|---|---|---|
| bootstrap 错误是否由 schema breadth 或 description salience 引起 | F5.0 | 已反证，不进入生产修复 |
| Finish identity 的对象线形态是否诱发 `goal` 泛化 | F5.0b | 已反证；对象不是必要原因 |
| 哪个最小 wire contract 进入生产 | F5.0b | 已冻结 E=`finish_identity: { id }` |
| handoff variant 的最小字段名和 provider 可生成形态 | F5.2 probe | probe/typed parser 不一致不得实现 Runtime |
| branch/join 是否自然需要 `complete_then_wait` | F5.2 fixture | 无必要性证据不增加变体 |
| state handoff 是否能复用现有 candidate transaction 而不扩展平行 reducer | F5.2 design | 不能复用则暂停并重新评审架构 |

## 14. 决策记录

| Decision | Status | Reason |
|---|---|---|
| 重开 Phase F，新增 F5 | Accepted | F0-F4 机制完成但 outcome gate 失败 |
| H-008 schema breadth/description 归因 | Rejected | A/B/C=6/5/6，未达到任一支持门 |
| Finish identity wire shape 单独建证据门 | Accepted | 不把未确认结构假设并入 F5.1 |
| E 对象命名束进入 F5.0c | Accepted | E=6/6，F 标量仍有 1/6 类型错误 |
| 删除 immutable full lifecycle 暴露 | Planned | 残留 choice break 下为明确负收益且工具面与 hard state 矛盾 |
| 恢复 schema-first complete handoff | Planned | R5 已验证能力在 R6 迁移中丢失 |
| 不恢复 R5 旧数据模型或字段 | Accepted | 只迁移行为不变量，R6 Rooted DAG 保持唯一领域模型 |
| 不通过 sibling tool calls 表达 handoff | Accepted | 单个 function schema 无法约束兄弟调用存在 |
| 不用 Runtime 自动选择 next | Accepted | Agent 完整声明，Runtime 只做机械事务 |
| 不提前压缩 projection | Accepted | 当前回归来自工具合同和请求路径，不是 Map 超限 |

## 15. 计划质量检查

- [x] 已反证 H-008/H-011 与已确认 H-012、待实施修复项分开。
- [x] 每次只改一个策略，并要求阶段内独立收益证据。
- [x] 保留 Agent 决策权与 Runtime 硬状态边界。
- [x] 没有兼容分支、自动推进、语义裁剪或提示词补洞。
- [x] correctness、成本、缓存、Map、terminal、日志均有量化门。
- [x] Phase E 是明确 outcome baseline，Phase G 不能补证。
- [x] 生产代码、测试、日志、Docker 和回退路径均映射完整。
