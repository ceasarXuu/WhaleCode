# R6 显式终结与 Replay 观测收敛设计

- Created: 2026-07-16
- Updated: 2026-07-16
- Version: v1
- Status: Implementing (E2 completed, E3 in progress)
- Owner / Responsible: WhaleCode R6
- Related Systems: TaskSpace control、provider turn loop、ActionMap replay、benchmark observer
- Related Links: `01-r6-phased-implementation-plan.md`、`09-r6-phase-e-finish-boundary-result.md`、
  `11-r6-phase-e2-canonical-replay-result.md`
- Risk Level: High
- Plan Type: Full

## 1. 结论

R6-E-TERM-01 与 R6-E-OBS-01 不分别增加补丁。两者统一按“一个写入权威、一个重放权威”收敛：

1. `ActionMap` 是任务生命周期唯一权威；`taskspace_control.finish_end` 是唯一终结命令。
2. Finish READY 是机械硬状态。此时复用已有 named `taskspace_control` tool choice，Agent 自主选择
   `finish_end`、rework、扩图或读取 Map 信息；Runtime 不替 Agent 选择动作。
3. turn completion 只校验“Map 是否已显式闭合”，不读取最终文本、不推断任务是否完成。异常只记为
   协议错误，不自动闭合、不生成无界 recovery request。
4. production resume 与 offline observer 必须调用同一套 Rust checkpoint + delta replay；PowerShell 只
   构造报告，不再拥有第二套 Map 状态解释器。
5. 现有通用 Hook 沿用既有行为且不在本专项修改；终结和 replay 方案均不依赖 Hook。

目标结构：

```text
Agent intent
  -> one taskspace_control schema
  -> canonical ActionMap transaction
  -> canonical rollout checkpoint/delta
  -> one Rust replay implementation
       -> session resume
       -> offline replay export
            -> observer/report
```

禁止形成 `prompt state`、`observer state` 与 `ActionMap state` 多套平行状态。

## 2. 已证实事实

### 2.1 TERM-01

当前 `finish_end` schema、typed args、handler、Root/Finish 原子事务和 terminal message release 均已存在。
缺口位于 provider hard-state selection：

- 空 Map 时，provider 使用 named `taskspace_control`，并隐藏 ordinary tools；
- Active Map 时，tool choice 无条件回到 `auto`；
- Finish READY 时没有 current Work lease，ordinary tools 本就不合法，但仍被暴露；
- Agent 因而可以返回无 tool call 的 assistant final，turn loop 随即进入通用 completion，而 Map 保持
  Root OPEN、Finish READY。

这不是 `finish_end` 事务缺失，也不是 Agent 最终文本需要语义识别。直接根因是机械 hard state 没有贯通到
provider action surface 和 turn completion。

### 2.2 OBS-01

生产路径已经通过 `apply_snapshot_delta` 对 checkpoint id、base hash、previous hash、result hash 和 JSON
Patch 顺序进行校验。session reconstruction 使用这套能力恢复最终 snapshot。

PowerShell observer 独立扫描 rollout：snapshot replay 只识别 `snapshot_updated`，不处理
`snapshot_delta`；同时它还根据 node/lease/result 等生命周期事件维护另一份可变 Map 状态。因此它在没有
terminal full checkpoint 时可能停留在旧状态或形成混合状态。这是 read model 分叉，不是 Runtime Map 丢失。

## 3. 目标与非目标

### 3.1 目标

| Goal | 可验证结果 | 工程收益 |
|---|---|---|
| 显式终结闭环 | 成功 TaskSpace run 的最终回答只来自成功 `finish_end` | Map、用户终结和 replay 状态一致 |
| 保留 Agent 决策权 | Finish READY 时 Runtime 只要求进入 control tool，不选择具体 action | 不把智能迁入 Runtime |
| 无惩罚式循环 | plain final 不产生 recovery request | 避免请求、token 和缓存成本放大 |
| 单一 replay | resume 与 observer 对同一 rollout 生成相同 snapshot hash | 消除 checkpoint-only 假状态 |
| 可诊断 | 每个 hard-state selection、terminal commit 和 replay failure 有稳定事件 | trace 可直接定位责任层 |

### 3.2 非目标

- 不解析 assistant 文本中的“完成”“测试通过”等语义。
- 不让 Runtime 从 plain final 自动构造 `finish_end`。
- 不新增 `taskspace_finish`、`taskspace_stop` 或第二套 terminal tool。
- 不把 Finish READY 等同于“Agent 必须结束”；Agent 仍可 rework 或扩图。
- 不在 projection 注入 next action、策略提示或纠错文案。
- 不在 PowerShell、TypeScript 或 Viewer 重写 JSON Patch/replay 算法。
- 不为历史实验数据增加 compatibility、migration 或 silent fallback。

## 4. 冻结不变量

| ID | 不变量 |
|---|---|
| T1 | Root 和 Finish 只能由成功的 Agent `finish_end` 在同一 revision 闭合 |
| T2 | terminal summary 字节级来自 Agent tool argument，Runtime 不总结、不润色 |
| T3 | Finish READY 只表示 terminal control frontier，不表示 Runtime 判断任务语义完成 |
| T4 | 非 complete Map 不得产生成功 TaskSpace TurnComplete |
| T5 | provider 异常不得自动提交 Map mutation |
| R1 | rollout 是 replay 权威；observer 是可重建 read model |
| R2 | checkpoint/delta 任一 hash、sequence 或 patch 失败都明确 fatal，不降级到旧 checkpoint |
| R3 | observer final snapshot hash 必须来自 canonical Rust replay proof |

## 5. 终结链路设计

### 5.1 Provider control mode

把当前布尔 `map_requires_initialization` 的选择扩展为由 canonical control state 机械导出的内部枚举：

```text
BootstrapRequired       -> named taskspace_control; only control visible
WorkActive              -> auto; ordinary tools + active taskspace_control visible
TerminalControlRequired -> named taskspace_control; only control visible
```

`TerminalControlRequired` 的判定只能使用：

```text
map.complete == false
finish.status == READY
current_node == none
pending/ready/running/blocked Work == none
```

这些字段已经属于 `ActionMapControlState`。不新增 terminal flag 存储，不从节点名称、goal、工具结果或最终文本
推断状态。该 mode 通过一个无持久化的 `requires_named_taskspace_control()` 从 Map 直接派生，不把
`finish_ready` 塞入 provider budget snapshot，避免预算观测结构成为第二份生命周期状态。

### 5.2 Agent 在 terminal frontier 的自由度

named tool choice 只约束本次响应必须调用现有 `taskspace_control`，不约束具体 variant：

- 接受当前结果：调用 `finish_end(expected_revision, final_summary)`；
- 发现仍需工作：`transition_node(rework)` 或 `mutate_graph`，再显式 `bind` Work 以获得 lease；
- 需要 Map 证据：`expand_nodes` / `read_output_ref`，下一请求仍保持 terminal control mode；
- 非法动作：沿用 typed parser/domain violation 原样拒绝，不给策略性 next action。

Finish READY 时 ordinary tools 没有 Work lease，本来就不具备合法执行条件。隐藏它们是 hard-state capability
projection，不是 Runtime 代替 Agent 思考。

### 5.3 Tool、基础指令与 projection 的分工

| Surface | 保留内容 | 禁止内容 |
|---|---|---|
| tool schema | action、参数、机械效果、`finish_end` 唯一发布 final summary | 工作建议、完成判断、策略解释 |
| TaskSpace 基础指令 | 一条稳定协议：TaskSpace final 只由成功 `finish_end` 发布 | 在多个 prompt 重复同一合同 |
| Map projection | Root/Finish/nodes/edges/frontier/revision 的真实状态 | “下一步应 finish/rework”等行动建议 |
| tool result | committed/rejected、revision、原始 violation code、Map state | Runtime 生成的纠错计划 |

合同 owner 是 tool schema。基础指令只说明会话协议，projection 只展示事实，不再三重复制完整 schema。

### 5.4 Response contract 与 Turn completion gate

provider request 构造时，把 control mode 作为不可变的 turn-local response contract 传给既有 stream/sequence
处理链；它只用于确定本次响应允许发布哪类 item，不持久化、不重新读取 Map，也不形成第二份状态。处于
`TerminalControlRequired` 时，stream adapter 不得把普通 assistant text 发布成 final-answer item 或写入
`last_agent_message`；文本仍原样记录到 history/rollout，并可作为 nonterminal/commentary item 发布。

在 provider tool sequence 完成后、TurnComplete 之前再执行机械 completion check；不引入
新的状态机或 rule engine。成功 `finish_end` 后 active map id 已清空，因此本 turn 的完成凭证不是
`action_map_control_state(None)`，而是 tool sequence 返回的 terminal carrier。carrier 只能由已经提交
`finish_end` 的 handler 产生，并携带 map id、revision 和 Agent 原样 summary；它不是第二份持久状态。

| 状态 | 行为 |
|---|---|
| Standard mode | 保持 Standard 行为 |
| TaskSpace response 带 committed terminal carrier | 发布其 terminal message，允许 TurnComplete |
| Map open，response 已含 tool call/follow-up | 正常进入下一次 provider request |
| TaskSpace response 无 terminal carrier、无 follow-up 且 provider 试图结束 | 记录 terminal protocol anomaly，不提交 mutation，不宣告成功 |

正常路径依赖请求前的 named tool choice，completion gate 只兜底 provider 不遵守 tool choice、状态竞态或实现回归。
兜底不得把 assistant 文本转换成 terminal summary，也不得自动发起反复 provider recovery。异常 turn 明确以
`taskspace_terminal_protocol_violation` 结束并保留 OPEN Map，属于严重机制错误而不是 Agent 任务失败。

只有成功 `finish_end` carrier 的 summary 能发布 final-answer item。异常 fixture 必须同时覆盖 delta、completed
item 和 turn result，证明 provider 即使返回 plain text，也没有 `AgentMessageDelta`、
`ItemCompleted(FinalAnswer)` 或 `last_agent_message` 泄漏到 UI/turn result。

### 5.5 Hook 不在本专项范围

当前失败没有证据指向 Hook，核心修复也不需要 Hook。R6 不修改通用 Hook 的配置、payload、dispatch 顺序、
失败语义或 Stop/AfterAgent 行为，不让 `taskspace_control` 新接入 Hook，也不新增 TaskSpace 专用 hook event/state。
如果未来出现可复现的 Hook 与已提交事务冲突，应作为独立问题采集 trace、确认根因后再设计，不能在本专项中
预防性修改。

## 6. Replay 与 observer 设计

### 6.1 单一 Rust replay API

不能只抽取 JSON Patch 函数，也不能让 offline caller 先筛选 checkpoint/delta。共享边界必须覆盖三层：

1. path-level loader：读取完整 JSONL，返回原始文件 SHA256、parse report 和完整有序 `RolloutItem`；
2. canonical reducer：接收完整 `RolloutItem`，统一处理 turn rollback、compaction 和存活 segment 选择，
   再选择 checkpoint 并应用 delta；
3. restore validator：session resume 与 offline export 共用 snapshot schema、Rooted DAG、lease/binding 不变量
   校验。

把 `session/rollout_reconstruction.rs` 中 Map segment selection 和 checkpoint/delta 重放抽到 `action_map` 的
共享模块；生产恢复不得保留旧私有实现。最终返回：

```text
ReplayedActionMapState {
  rollout_sha256,
  parse_error_count,
  snapshot,
  checkpoint_id,
  base_snapshot_sha256,
  final_snapshot_sha256,
  parsed_checkpoint_count,
  parsed_delta_count,
  surviving_checkpoint_count,
  surviving_delta_count,
  active_checkpoint_id,
  active_chain_applied_delta_count,
  active_chain_last_delta_sequence
}
```

`parsed_*` 统计完整已解析 rollout；`surviving_*` 统计 rollback/compaction 选择后的全部存活 segment；
`active_chain_*` 只统计产生 final snapshot 的最后 checkpoint 链。三种口径不得混用。

session resume 和 offline export 同时调用这套 loader + reducer + validator。`apply_snapshot_delta` 继续是 JSON
Patch 与 hash 校验的唯一实现。`parse_error_count > 0` 视为 rollout 不完整并明确失败，不能丢弃坏行后把较旧
snapshot 当作最终状态。offline export 只有通过 production restore validation 后才能生成 proof。

共享层使用 typed `TaskSpaceReplayError`，至少区分 load/parse、missing checkpoint、sequence gap/order、base
id/hash、previous/result hash、invalid patch、unsupported snapshot schema 和 domain invariant。CLI 和 observer
只序列化稳定 code，不根据错误字符串反向分类。

### 6.2 Offline export

在现有 `whale debug` 命令族中增加机械命令：

```text
whale debug taskspace-replay --rollout <rollout.jsonl> --output <replay-proof.json>
```

命令只做三件事：读取 rollout、调用 canonical replay、序列化 snapshot + proof。成功时通过临时文件 + rename
原子写入；失败时返回非零退出码和不含 partial snapshot 的 JSON error envelope。它不渲染报告、不分析 Agent
行为、不修复损坏数据。benchmark 必须调用本轮 attested Whale binary，避免本机工具版本漂移。

### 6.3 Observer 纵向切换

PowerShell observer 保留 timeline、tool/hook/log 统计和 Markdown/HTML 构造；事件扫描只能产生 timeline 和
counts，不得修改 final-state collection。Map/task/node/edge/lease/result 的最终集合全部从
`replay-proof.json.snapshot` 一次性构造，禁止在旧可变对象上做字段覆盖。

full 与 large-rollout 可以保留不同的 timeline 成本策略，但必须共用同一个 proof consumer/final-state renderer。

删除 checkpoint-only final state 路径。canonical replay 不可用或失败时：

- observer availability 标记 `replay_failed`；
- 输出 checkpoint/delta 数和稳定 error code；
- final Map facts 全部为 unavailable；
- benchmark evidence gate 失败；
- 禁止降级到最后一个 checkpoint、raw control preview 或猜测状态。

Standard/no-TaskSpace rollout 返回 `not_applicable`，不记为 replay failure。

### 6.4 Replay 对账

observer 额外记录但不重新计算以下事实：

- final snapshot hash；
- final rooted DAG revision；
- checkpoint id / delta sequence / applied count；
- terminal event revision（存在时）；
- replay schema version。

`finish_end` 成功时，terminal event revision、snapshot revision 和 observer revision 必须一致；Map 未闭合时，
observer 仍必须展示最新 OPEN/READY 状态，而不是初始化 checkpoint。

## 7. 明确拒绝的方案

| 方案 | 拒绝原因 |
|---|---|
| 只加强 prompt/tool description | 概率性，不能保证 hard terminal contract |
| 新增独立 `taskspace_finish` tool | 与 `taskspace_control` 平行，扩大工具与 handler 分叉 |
| Runtime 用 plain final 自动调用 `finish_end` | 把 Agent 行为改写成 Runtime mutation，违反手动终结 |
| 解析最终文本判断任务完成 | Runtime 越界进入语义和推理层 |
| Finish READY 时强制具体 `finish_end` variant | 剥夺 Agent rework、扩图和读取证据的选择 |
| 每次 turn 额外写 full checkpoint | 掩盖 delta 链缺陷，不能证明 crash/resume replay |
| PowerShell 实现 JSON Patch | 产生第二套 replay 权威和长期漂移 |
| observer 失败时退回旧 checkpoint | 把已知错误状态包装成可用报告 |

## 8. 分阶段实施

### E2：Canonical replay 抽取

**Entry**：现有 production replay 测试全绿，R6-E-OBS-01 fixture 固化。

**任务**：

1. 抽取共享 replay API，不改变 event wire schema。
2. loader 暴露 rollout hash、parse report；parse error 不再被静默丢弃。
3. session reconstruction 切换到共享 segment selection + reducer + restore validation。
4. 增加 offline debug export、typed error、JSON envelope、原子输出与 proof schema。
5. 建立 checkpoint + N delta、multiple checkpoint reset、delta without checkpoint、missing base、gap、reorder、
   hash mismatch、invalid patch、tail parse error、rollback、compaction、fork、unsupported schema 和 DAG/lease
   invariant 测试。

**Exit**：resume 与 offline export 对完整矩阵的 final snapshot hash 和 restore verdict 100% 相同；全部
corruption 返回稳定 typed code 且无 partial snapshot。

### E3：Observer 纵向切换

**Entry**：E2 100% 完成。

**任务**：

1. observer 调用 attested Whale replay command。
2. final-state collection 改为只从 proof snapshot 一次构造。
3. 删除 full/large 两条事件驱动和 checkpoint-only final-state 归并；保留 timeline/count 解析。
4. 回放 R6-E-OBS-01 原始 rollout，并对同一 fixture 强制执行 full 与 large 两种 export policy。

**Exit**：full/large 的 final snapshot hash、revision、node/edge/lease/result 集合完全相同，且 revision=7、Work
全 completed、Finish READY、Root OPEN；人为损坏 delta 时报告不可用而非旧状态。

### E4：Terminal hard-state affordance

**Entry**：E3 让 live trace final state 可被可靠观测。

**任务**：

1. 从 `ActionMapControlState` 派生 provider control mode。
2. Bootstrap 和 Terminal frontier 复用 named `taskspace_control` 与 only-control visibility。
3. tool schema 保持单工具；补一条唯一基础协议说明，不写入 projection。
4. 记录 control mode 选择及其 canonical revision。
5. 单独验证 named choice 关闭 provider thinking 后的 `final_summary` 质量与 rework 选择能力。

**Exit**：Finish READY 请求的 `tool_choice=taskspace_control`，ordinary tools 不暴露；Agent 仍能选择 finish、rework 或扩图。

### E5：Completion gate

**Entry**：E4 deterministic tests 全绿。

**任务**：

1. 将 provider control mode 作为 turn-local response contract 贯通 stream、tool sequence 与 completion。
2. completion 使用 terminal carrier 区分 Standard/no-map 与 TaskSpace committed terminal。
3. named-control assistant text 保留原文但降为 nonterminal；provider 违反 named choice 时发出稳定 protocol
   violation，不 recovery loop。
4. 证明 plain final 不会提交 terminal mutation，也不会泄漏 final-answer UI event。

**Exit**：无 `finish_end` carrier 时无成功 TaskSpace TurnComplete 和 final-answer UI item；异常额外 provider
request=0；Root/Finish/revision 不变。

### E6：原子性与 live 收益门禁

**Entry**：E2-E5 均 100% 完成。

**任务**：

1. 完成 snapshot/delta/resume/fork/crash/corruption 矩阵。
2. simple 与 complex 各跑 Standard、R5 baseline、R6 各3次，允许不同样本/arm 并行。
3. 检查 request、token、wall、cache、tool path、Map 和 replay proof。

**Exit**：6个 R6 runs 的 `finish_end` adoption=100%，plain-final-open-map=0，observer/runtime final hash 一致，
无 terminal recovery request；三臂 public/hidden oracle pass rate 均为100%。单次只作为 smoke，不用于稳定性或
收益结论。

## 9. Phase gate

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Proceed Decision |
|---|---|---|---|---|
| E2 | Rust unit/property + replay matrix | 不依赖 observer 改造 | loader/reducer/validator 双入口同 verdict/hash | 完成后进入 E3 |
| E3 | 旧失败 rollout full/large 离线重放 | 不依赖 terminal 修复 | proof/final collections 对账 | 完成后进入 E4 |
| E4 | provider payload/tool visibility fixture | 不修改通用 Hook | named choice + same tool schema | 完成后进入 E5 |
| E5 | turn-loop integration fixture | 不依赖 live sample | 无假 TurnComplete/无 retry loop | 完成后进入 E6 |
| E6 | crash matrix + 三臂 live trace | 无后续阶段补证 | 完整机器结果 | 通过后关闭 Phase E |

任何阶段未达到 100% 时暂停，不以后一阶段样本倒推当前阶段正确。

## 10. 实现完整性矩阵

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Status |
|---|---|---|---|---|---|
| shared replay | `action_map` + session reconstruction | resume/offline debug | 15 replay + 31 reconstruction tests 等 targeted regression | revision 7 replay proof | completed |
| observer switch | observability exporter | benchmark metrics extractor | R6-E-OBS-01 fixture | observer revision/hash | in progress |
| provider control mode | action map state + session prompt build | provider request | tool choice/visibility tests | control mode event | planned |
| completion gate | stream/sequence/turn completion | provider no-tool end | turn integration | protocol violation event | planned |
| live gate | Docker benchmark | Standard/R5/R6 | business/public/hidden tests | trace + cost report | planned |

## 11. 日志设计

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation | Level | Consumer |
|---|---|---|---|---|---|---|---|
| control mode derive | bootstrap/work/terminal/complete | selected mode + revision | state unavailable | `reason_code` | turn/map/revision | info/error | benchmark/debug |
| provider request | named/auto | expected tool choice | named choice violated | `provider_contract_code` | request/turn/map | info/error | runtime |
| finish transaction | preflight/committed/carrier | Root+Finish same revision | rejection | violation code | call/map/revision | info/warn | Agent/observer |
| turn completion | pending/allowed/rejected | complete=true | open Map stop attempt | `taskspace_terminal_protocol_violation` | turn/map/revision | info/error | CLI/observer |
| replay | checkpoint/delta/final | final hash | sequence/hash/patch failure | `replay_error_code` | rollout/checkpoint/sequence | info/error | observer |
| observer | proof consumed | revision/hash match | proof unavailable | `availability_reason` | run/side/rollout | info/error | report gate |

日志只陈述机械状态和错误码，不写“Agent 应该如何修复”的策略文本。

## 12. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Terminal named choice 改变 provider/cache shape | 单次请求缓存率变化 | 保持 tools schema 不变，只改变已有 tool_choice；单独统计 terminal request |
| Agent 想继续工作但 ordinary tools 被隐藏 | 需要先 rework/扩图，多一步 control | 这是无 lease 状态的合法恢复路径；不强制 `finish_end` variant |
| provider 不遵守 named tool choice | Map 开放但响应试图结束 | completion gate 明确报机制错误，不重试、不自动提交 |
| named choice 关闭 DeepSeek thinking | final summary 或 rework 判断质量下降 | live trace 单独比较 summary 完整性、action 选择和 output token |
| offline binary 与 run binary 不一致 | replay 结论不可信 | benchmark 固定调用 attested run binary 并记录 SHA256 |
| observer 失去 fallback 后报告不可用 | 暂时减少可见指标 | 明确失败优于输出错误 Map；成本指标可单独保留并标注范围 |

## 13. 外部参考与取舍

1. [DeepSeek Chat Completion API](https://api-docs.deepseek.com/api/create-chat-completion/)：指定具体
   `tool_choice` 会强制模型调用该工具，支持 Finish READY 复用现有 named `taskspace_control`。
2. [OpenAI Responses API](https://platform.openai.com/docs/api-reference/responses)：`auto`、`required` 和指定
   tool 是 provider 原生 action-surface 约束，R6 沿用协议能力而不增加 Runtime 语义判断。
3. [Azure Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)：
   event store 是 system of record，projection/read model 应由同一事件流重建。R6 因此删除 observer 的
   checkpoint-only 平行状态解释。
4. [RFC 6902 JSON Patch](https://www.rfc-editor.org/rfc/rfc6902)：patch 操作按顺序应用，失败必须终止。
   R6 继续由 Rust canonical replay 统一执行顺序和失败处理，不在报告脚本复制实现。

## 14. 决策记录

| Date | Decision | Reason |
|---|---|---|
| 2026-07-16 | Finish READY 复用 named `taskspace_control` | 利用既有 hard-state 能力，不新增 tool/handler 分叉 |
| 2026-07-16 | Hook 完全移出本专项实施范围 | 无已证实 Hook 缺陷，核心修复不依赖 Hook，避免预防性扩散 |
| 2026-07-16 | observer 调用 canonical Rust replay | 单一 hash/sequence/patch 权威 |
| 2026-07-16 | replay 失败无 fallback | 防止已知错误状态进入性能与进度结论 |
