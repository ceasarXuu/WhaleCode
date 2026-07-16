# R6 Phase F 上下文唯一性与成本收敛实施计划

- Created: 2026-07-16
- Updated: 2026-07-17
- Version: 1.0
- Status: F0-F4 Mechanisms Complete / Outcome Gate Failed / Superseded by Phase F5
- Owner / Responsible: WhaleCode Runtime
- Related Systems: TaskSpace projection、tool schema、provider request、Event Store、benchmark observer
- Related Links: `01-r6-phased-implementation-plan.md`、`15-r6-phase-e6-atomicity-live-result.md`
- Risk Level: High
- Plan Type: Full

## 1. 背景与问题

Phase E 已证明 R6 Rooted DAG、显式 `finish_end`、terminal 原子事务和 replay 一致性成立，但成本门禁只形成
基线，没有证明性能收益。相对同轮 Standard：

| Sample | Request | Input | Uncached | Wall | Request 2+ cache |
|---|---:|---:|---:|---:|---:|
| simple | 1.40x | 1.52x | 3.33x | 1.69x | -5.43pp |
| complex | 1.16x | 1.35x | 2.33x | 1.12x | -3.65pp |

现有证据把差异分成三类：

1. TaskSpace 增加 provider request，后续请求反复携带已经增长的自然历史；
2. `taskspace_control` call/result、当前 Map projection 和历史 `map_state` 存在状态事实重复；
3. bootstrap/work/terminal 使用 named/auto/named，工具列表和 schema 也切换，破坏 DeepSeek 严格前缀缓存。

> 2026-07-17 状态修正：F0-F4 局部机制已实现，但 final 相对 Phase E 的 request/input 继续回归，不能进入
> Phase G。本计划保留为历史实施记录，修复与新 outcome gate 以
> `18-r6-phase-f5-cost-regression-repair-plan.md` 为准。

Phase F 是 ownership 与 request contract 收敛，不是压缩阶段。任何基于“可能不重要”的语义裁剪都留到
Phase G 独立实验。

## 2. 目标

1. 每个 provider request 的 system、tool schema、natural history、Map projection 和 tool feedback 都可独立计量。
2. 当前完整 Map 状态只有 projection 一个 provider-visible owner。
3. control result 忠实返回动作结果、revision、delta、错误和引用，不重复返回完整 Map。
4. TaskSpace 生命周期使用一套稳定 tool schema；不得因 bootstrap、work、terminal 重建不同 schema。
5. 验证稳定 `tool_choice=required` 是否能在不指定具体动作的前提下保持 Agent 工具选择和终结可靠性。
6. 允许 Agent 把状态变更与有明确执行顺序的普通动作声明在一次 control call 中；Runtime 只机械执行。
7. simple 与 complex correctness、Map、terminal、replay 不回退，并能把收益定位到具体 request 和 payload section。

## 3. 非目标

- 不分页或删除 Root、Finish、全 nodes/edges skeleton。
- 不新增 projection 策略提示、next-action 文案、错误解释或 Runtime 语义决策。
- 不解析 reasoning，不根据自然语言判断任务是否完成。
- 不改写 ordinary tool 原始 outcome、stderr、excerpt、truncation 或 output ref。
- 不在本阶段引入长期 Map 超限压缩策略。
- 不为 R5 数据或旧 schema 增加兼容分支。

## 4. 约束与事实

| 类型 | 内容 | 证据 |
|---|---|---|
| 事实 | 每个 R6 provider payload 只有一个 projection marker，但现有 scanner 不验证 freshness | exact payload scan `active_projection_count=1` |
| 事实 | steady-state 仅检查历史是否已有 projection，可能继续暴露首次 bootstrap snapshot | `session/mod.rs` provider context path |
| 事实 | simple/complex 每次运行各有两次 tool choice/shape 切换 | Phase E performance observation |
| 事实 | 最终 named request 分别贡献 11,262/26,398 uncached input | provider cache trace 聚合 |
| 事实 | control call+result 原始文本每 run 约 3-6.3KB，后续请求继续携带 | canonical task context event |
| 事实 | Rust API 与 ChatCompletions serializer 已支持 `ToolChoice::Required` | `codex-api` code/test |
| 未验证 | DeepSeek V4 live 是否稳定遵守 `required` 且保留 thinking 质量 | F2 provider probe；当前 generic strict choice 会关闭 thinking |
| 约束 | Runtime 只能执行硬状态和 Agent 声明顺序 | R6 charter |

## 5. 方案总览

```text
F0 payload section observability
  -> F1 current Map single owner
  -> F2 immutable tool contract + required probe
  -> F3 declared state/action sequence
  -> F3.5 epoch baseline projection + canonical delta journal
  -> F4 deterministic + Docker live gate
```

每个阶段必须独立提交、独立测试、独立生成对比结果。前一阶段未达到 100% 退出门禁时暂停，不用后续阶段
补证据。

## 6. Phase F0：Payload 分区观测

### 6.1 目标

在 provider wire 序列化前对请求做纯机械分区，不保存 secret 或完整用户文本，只记录 section bytes、估算 token、
SHA-256、message role/count 和相邻请求 LCP。

### 6.2 实施

1. 为 provider cache trace 增加 `base_instructions`、`tools`、`messages`、`active_projection`、
   `taskspace_control_feedback`、`ordinary_tool_feedback` 和 `other` 的 bytes/token estimate。
2. 记录 `tools_hash`、`tool_choice_kind`、`active_projection_count`、control feedback count/bytes。
3. benchmark observer 聚合每 request 明细和每 section 总和/均值/中位数。
4. projection 无法解析时明确 `unavailable_reason`，禁止以零代替缺失值。

### 6.3 退出门禁

- fixture 对每个 section 的 bytes/count/hash 断言通过；
- Standard 不产生 TaskSpace section；
- TaskSpace projection count 恰好为 1；
- Phase E 原始 artifacts 可离线重算，缺失值标记为 unavailable；
- 不记录 API key、Authorization header 或未经 hash 的完整 payload。

### 6.4 完成证据（2026-07-16）

- provider final-wire trace 已升级为 `provider-chat-wire-trace-v3`，八类 section bytes 与最终 payload bytes
  逐请求精确对账；只记录 count、bytes、估算 token 和 SHA-256。
- projection identity 区分 `bootstrap`、`active(map hash/revision/projection hash)` 与 `unavailable(reason)`；
  历史 v2 artifact 明确 unavailable，不以零冒充观测值。
- 性能报告已输出逐 request 数值、section 总和/均值/中位数及 projection identity 聚合。
- 验证：Rust 定向测试 11/11，成本观测自测、性能报告自测、benchmark harness 全部通过。
- 实现提交：`12b479171`；均值/中位数闭环提交见本阶段后续提交记录。

## 7. Phase F1：Projection Freshness 与 Map 当前状态唯一所有权

### 7.1 Freshness 前置修复

当前“projection count=1”只证明 marker 唯一，不证明它与 canonical DAG 同 revision/hash。F1 必须先完成：

1. projection 不再作为普通历史项长期持有；provider composer 从 canonical natural history 中过滤旧 projection；
2. 每个初始请求、后续请求、retry、resume 和 compaction continuation 都重新读取同一 canonical DAG；
3. 在全部自然历史之后追加一份 ephemeral current projection，使历史 LCP 保持到动态 suffix 之前；
4. scanner 同时校验 projection map id、revision、projection hash 与本次 canonical snapshot identity；
5. freshness fixture 和 simple/complex smoke 通过后，才允许删除 control result 的 `map_state`。

### 7.2 所有权规则

| 信息 | 唯一 provider-visible owner | 其他载体允许内容 |
|---|---|---|
| 当前完整 Map | active projection | snapshot/replay 只持久化，不额外注入 |
| control 动作 | Agent 原始 function call | result 不复述参数全文 |
| control 成功 | control result | committed revision、机械 delta、step refs |
| control 失败 | control result | class/code/message、expected/actual、state_commit |
| ordinary tool 结果 | 原始 tool feedback/Event Store | projection 只保存忠实 ref |

### 7.3 实施

1. 从成功和失败 control result 删除完整 `map_state`。
2. 增加稳定的 `committed_revision` 和 typed delta；delta 必须来自 canonical domain event batch，不能从简化 handler outcome 推断。
3. 失败结果保持原始错误语义和零提交证明，不由 projection 生成纠错说明。
4. observer 同步读取新结果，不保留旧生产 schema 兼容路径；历史 benchmark 使用冻结版本 observer。

### 7.4 退出门禁

- 每个 provider request 的 projection identity 与本次 canonical map revision/hash 一致；
- success/failure fixture 可从原始 call、result delta 和下一轮 projection 对账；
- provider payload 中完整 active Map section 计数为 1；
- control result 不含 `map_state`；
- failure exit/stderr/ref 与 Event Store 逐字一致；
- deterministic control feedback bytes 相对 E6 fixture 降低至少 30%。

### 7.5 完成证据（2026-07-16）

- projection 改为 provider request 前从 canonical DAG 机械构造的 ephemeral view；canonical history、resume 和
  compaction continuation 均不再长期持有旧 projection。
- exact payload scanner 逐请求对账 projection kind、Map revision/hash 和 projection hash；simple 15/15、
  complex 13/13 freshness 通过，active projection section 始终为 1。
- success/failure control result 已删除 `map_state`，成功结果只返回 committed revision、canonical event refs
  和必要 step identity；失败结果保持原始错误与 `state_commit=false`、`partial_commit=0`。
- nested ordinary call/output 继续逐字独立可见并由 Event Store 的 parent linkage 关联；outer control result 不再
  二次复制其 tool name、call id、success 和 event refs。
- 初始化 control result 从 E6 的 1,018 bytes 降至 539 bytes，下降 47.1%；确定性 fixture 强制至少下降
  30%。
- Docker smoke：simple Standard/R6 均 solved（7/15 requests，14.51s/35.35s）；complex Standard/R6
  均 solved（15/13 requests，52.42s/48.96s），两个 R6 Map 均完整闭合。
- simple 首次 smoke 出现 Agent 未闭合 Map，被 terminal protocol 中断；同构复跑成功。该随机性及成功复跑中的
  3 次可恢复状态错误保留为 F2/F3 输入，不计为 F1 ownership 回归。
- 验证：action map 67/67、session 182/182、control 21/21、sequence 11/11、nested Event Store 4/4；
  cost/performance/harness self-test 与 skill validator 通过。

## 8. Phase F2：稳定 Tool Contract 与缓存形态

### 8.1 设计

1. 合并 bootstrap/active control schema，为整个 TaskSpace turn 构造一套 immutable lifecycle schema。
2. `update_plan` 始终隐藏，其他工具列表不因 Map 阶段变化。
3. 单独验证 `ToolChoice::Required`：它只要求 Agent 选择某个工具，不选择具体工具，但当前 generic strict
   choice 会关闭 thinking，不能仅因缓存收益直接采用。
4. Runtime 对当前阶段不合法的 action 返回现有硬状态错误，不隐藏工具、不自动改写为其他 action。
5. `finish_end` 成功后 terminal carrier 直接发布 Agent summary，不再发 provider request。

### 8.2 Provider 决策门禁

| 结果 | 决策 |
|---|---|
| DeepSeek 接受 `required`，thinking 保持或质量无回退，terminal 6/6 | 使用稳定 `required` |
| `required` 必须关闭 thinking 或质量回退 | 记录为 HOLD；不以缓存收益换取 Agent 智能能力 |
| API 拒绝或模型返回无工具响应 | 记录为 HOLD；不得本地伪造 tool call 或自动重试 |
| correctness 通过但错误工具调用显著增加 | 记录为 HOLD；不得用 Runtime 语义筛选补救 |

### 8.3 退出门禁

- 每个 R6 run 的 `tools_hash` 唯一值为 1；
- immutable schema 下每个 R6 run 的 `tools_hash` 唯一值为 1；
- 若 `required` 通过质量门禁，`tool_choice_kind` 唯一值为 1、shape transition=0；否则明确记录 HOLD；
- terminal 前一请求到 terminal request 的 messages LCP 和 tools prefix 保持；
- terminal adoption、Root/Finish closure、replay hash 均为 100%。

### 8.4 完成证据（2026-07-16）

- `taskspace_control` 已收敛为一套 immutable lifecycle schema，固定包含 initialize、graph mutation、node
  transition、finish、expand 和 output-ref read；bootstrap/work/terminal 不再替换 schema 或工具列表。
- TaskSpace 始终隐藏线性 `update_plan`，其他 13 个工具全生命周期保持可见；simple/complex 每个 run 的
  `tools_hash` 唯一值均为 1，tools count 恒为 13。
- 初版 immutable schema 复制了顶层普通工具参数，tools section 达 35,648 bytes/request；按单一所有权修正后，
  顶层工具保留完整 schema，continuation 只引用工具名与原始参数信封，降至 24,449 bytes/request（-31.4%）。
- 同为 13 requests 的 simple TaskSpace input 从未去重版本 156,655 降至 112,079（-28.5%）；该对比可
  归因于 schema 去重。复杂样本请求随机从 20 降到 14，不把全部 input 降幅归因于 schema。
- DeepSeek provider probe：`required` 在 thinking disabled 下可用且支持有序多工具；thinking enabled 时返回
  HTTP 400 `thinking_tool_choice_incompatible`，无 reasoning content。决策为 HOLD，不进入生产路径。
- 因 HOLD，named/auto choice 仍有两个 shape；这是明确保留 Agent thinking 的 provider 能力边界，不增加本地
  伪造、自动重试或 Runtime 语义筛选。
- Docker smoke：simple、complex 的 Standard/R6 均 solved，R6 Map 均闭合；Rust `codex-tools` 141/141、
  Core tool-contract 14/14 通过。

## 9. Phase F3：Agent 声明的机械动作合并

### 9.1 设计

在现有 `TaskSpaceContinuation` 和 sequence executor 上扩展，不增加第二个 carrier：

- `initialize_map + continuation` 保持；
- `mutate_graph + continuation`：仅当已有 main binding 在图事务后仍有效时，执行 Agent 声明的普通动作；
- `transition_node(bind) + continuation`：bind 成功建立 current node/lease 后执行普通动作；
- `complete`、`block`、`unblock`、`rework` 不允许 continuation；Agent 可在同一 provider response 中显式声明
  `complete -> bind -> ordinary actions` 等多个 sibling calls，现有 sequence barrier 按顺序执行；
- `finish_end` 不允许 continuation，它本身就是唯一终点；
- 每个 request 最多一个 patch，patch 必须位于唯一 patch slot；
- sequence 首个失败后停止，未执行动作返回 typed skipped result。

Runtime 不补动作、不重排依赖、不判断动作是否“有意义”。

### 9.2 退出门禁

- sequence/parallel tests 覆盖成功、state failure、nested failure、patch failure 和 skipped tail；
- control call/result 的 call id、parent id、event ref 完整；
- 单 request 多 patch 在执行前拒绝且无部分提交；
- simple/complex request 不高于 F2 同样本中位数；若 Agent 未自然采用，只确认机制，不宣称 live 收益。
- ChatCompletions `parallel_tool_calls` wire 暴露单独做 provider probe；未验证前不把它当作 DeepSeek 行为前提。

### 9.3 完成证据（2026-07-16）

- schema 要求 `transition_node(bind)` 必须声明 continuation；`mutate_graph` 仅在已有有效 main binding 时允许
  continuation；其他 transition 和 `finish_end` 不接受 continuation。
- sequence 保持 Agent 声明顺序、首错停止和单 patch 预检；Runtime 不补动作、不推断下一节点。
- deterministic：schema 3/3、args 14/14、sequence 13/13、control 23/23，成本/原生 control/性能观察/
  harness 自测通过。
- simple live：Standard/R6 均 solved；R6 13 requests，等于 F2 基线；自然采用 init continuation 1 次、
  bind continuation 3 次，Map 完整闭合。
- complex live：Standard/R6 均 solved；R6 自然采用 bind continuation 2 次并闭合 Map，但 3 次初始化参数纠错和
  1 次未 bind complete 使 request 达 19，高于 F2 单次基线 14。F3 只确认机制成立，不宣称复杂样本成本收益。
- 两个 live run 的 TaskSpace request 2+ cache hit 仅 13.22%/13.27%，且 message prefix 0/12、0/18；
  该结构性失败进入 F3.5，F4 在修复前不得开始。

## 10. Phase F3.5：Epoch Baseline Projection 与前缀恢复

### 10.1 根因

F1 的 ephemeral current projection 每个请求都先从自然历史过滤，再追加到末尾。下一请求新增的 assistant/tool
history 会出现在旧 projection 原位置，因此上一请求的 projection 不再是下一请求的前缀，哪怕 Map revision 和
projection 内容完全没变也会破坏 provider 严格前缀缓存。F3 simple/complex 的 0% message prefix preservation 已
证实这是结构性问题，不是缓存冷启动或 projection 体积问题。

### 10.2 设计

1. projection 改为当前上下文 epoch 的 canonical Map 基线，固定在首次 provider request 的机械锚点；同一 epoch
   后续请求在该锚点之后追加原始 assistant、control call/result 和 ordinary tool call/result。
2. bootstrap 到 active Map 可替换同一锚点并开启 active epoch；此处允许一次可解释的 prefix break。
3. active epoch 内不因普通状态 transition 重写基线。基线 revision 之后的当前状态由 Agent 原始 control call、
   typed result delta 和 ordinary feedback 顺序表达；不得摘要、重排或语义改写。
4. compaction、resume、fork、历史替换或锚点前缀不一致时机械开启新 epoch，从当前 canonical Map 生成新基线。
5. projection 明确标记 `projection_role: epoch_baseline`。观测层区分“基线 identity 一致”与“当前 revision 相同”，
   不再把基线 hash 对账表述成 current freshness。
6. 该方案不是 Map 压缩策略；projection 全局拓扑、Root/Finish 和节点骨架仍完整，长期超限留给后续专项。

### 10.3 所有权

| 事实 | Provider-visible owner |
|---|---|
| epoch 起点完整 Map | 唯一 baseline projection |
| epoch 内 Agent 动作 | 原始 function/custom call |
| epoch 内状态变化 | 原始 typed control result delta |
| ordinary 工具反馈 | 原始 tool result / output ref |
| 当前状态 | baseline + 其后 canonical delta journal 的确定性回放结果 |

这不是平行维护第二套上下文：baseline 是 Event Store 当前 epoch 的起点，delta journal 是原自然上下文按状态机归属
后的原始顺序记录。

### 10.4 退出门禁

- 同一 epoch 第二个及后续请求必须以前一请求 messages 为精确前缀；fixture 覆盖 bootstrap、active、retry、
  compaction、resume 和 stale projection 输入。
- 每个请求 projection marker 仍恰好为 1，baseline identity 与 epoch 缓存一致；不存在 current freshness 假声明。
- baseline 后的 control call/result 与 Event Store 逐字一致，delta 缺失、重排、语义 rewrite 均为 0。
- simple/complex 各 1 次：correctness、Map、terminal、replay 100%；TaskSpace request 2+ cache hit 不低于 80%，
  prefix preservation 不低于 80%。未达到则停留 F3.5，不进入 F4。

### 10.5 完成证据（2026-07-16）

- projection 固定为上下文 epoch 的机械基线；bootstrap -> active 只在同一锚点替换一次，普通状态提交不再移动
  projection。
- compaction、resume、fork、history replacement 和锚点前缀变化会显式开启新 epoch；不保留 stale projection。
- simple smoke：request 2+ cache 84.27%、prefix 84.62%；complex smoke：83.02%、83.33%，均达到门禁。
- active epoch 的 projection revision/hash 固定，后续原始 call/result/tool feedback 构成 canonical delta journal，
  semantic replacement=0。
- 实现提交：`36a02b1eb`。

## 11. Phase F4：正式验证

### 11.1 Deterministic

- `codex-tools` schema 与 registry plan；
- `codex-core` control args/handler/output/sequence/provider contract；
- `codex-protocol` result/event wire；
- replay、resume、fork、terminal transaction；
- benchmark cost/performance observer 与 Docker harness；
- `cargo build -p codex-cli --bin whale --locked`。

### 11.2 Docker Live

每个策略完成后先各跑 simple/complex 1 次；F4 再执行：

- simple：Standard/R6 各 3 次，左右轮换；
- complex：Standard/R6 各 3 次，左右轮换；
- 固定 model、prompt、validator、hidden oracle、Docker hard boundary；
- 报告结果、动作、request、wall、input/cached/uncached/output、section cost、Map 和 terminal proof。

### 11.3 总退出门禁

```text
public/hidden correctness = 100%
finish_end adoption = 100%
Map closed = 100%
terminal raw hash = replay hash = 100%
active projection authoritative section = 1/request
semantic rewrite count = 0
tool schema transition = 0
tool choice transition = 0 or evidence-backed HOLD when thinking/quality gate fails
plain-final-open-map = 0
partial commit = 0
```

性能只报告观测值，不以牺牲 correctness 或语义忠实度换取门禁通过。

### 11.4 完成证据（2026-07-16）

- deterministic：tools 141、protocol 197、action map 67、control 25、sequence 13、session 183、replay 18、
  reconstruction 33；observer/harness/build/attestation 全部通过。
- Docker formal：simple 和 final complex 各 3 对，12/12 side solved，6/6 R6 Map/Root/Finish 完整闭合。
- F4 发现并修复 H-006：TaskSpace 路径感知 parser 未验证 JSON 尾部，可能静默执行 malformed 首值。最终 complex
  矩阵自然验证 malformed call 返回同 call id 的 typed protocol failure、零提交，Agent 后续正确恢复。
- simple R6 cache/prefix=84.19%/85.00%；complex=88.56%/89.09%。
- 成本未反转：simple requests/input=2.15x/3.10x，complex=1.41x/1.94x；后续审计证明相对 Phase E 也发生
  明确回归，因此 Phase F 重开并进入 F5，Phase G blocked。
- 完整报告见 `17-r6-phase-f-result.md`；最终代码提交 `726d3298b`。

## 12. 实现完整性矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| F0 section trace | 请求组成可归因 | `provider_wire_trace.rs`, `client.rs` | provider request | 11 Rust tests + 3 PowerShell suites | section hash/bytes/revision | none | completed |
| F1 result delta | 当前 Map 单 owner | `taskspace_control_output.rs` | tool result | 285 Rust tests + 3 PowerShell suites | control result + projection freshness trace | none | completed |
| F2 stable contract | schema 全 turn 稳定；choice 有证据 HOLD | `session/turn.rs`, `taskspace_tool.rs` | Prompt | 155 Rust tests | tools/choice hash | provider live probe | completed |
| F3 continuation | Agent 声明序列机械执行 | args/schema/sequence | tool router | 53 Rust tests + live | step/skipped refs | none | completed |
| F3.5 epoch baseline | projection 固定锚点，delta 自然追加 | session/state/client | provider composer | prefix/epoch tests | identity/prefix trace | none | completed |
| F4 live gate | 成本与正确性可比较 | benchmark scripts | Docker harness | harness tests | performance report | none | completed |

## 13. 变更链日志

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| payload build | section measured | section hashes/bytes complete | section unavailable | `unavailable_reason` | request/epoch id | info | observer |
| control execute | validated/committed | revision+delta | protocol/state/resource failure | class/code | call/map/revision | info/warn | Agent/observer |
| sequence | started/executed/stopped | ordered step refs | nested failure/skipped tail | failure code | parent/nested call id | info | Agent/observer |
| projection epoch | baseline anchored/delta appended | exact message prefix | prefix/identity mismatch | reset reason | epoch/prefix hash | info/warn | observer |
| provider choice | required sent | tool call returned | no-tool/API rejection | provider error code | request id | info/error | Runtime/observer |
| terminal | committed/published | carrier+hash | open Map response | terminal protocol code | turn/map/revision | info/error | CLI/observer |

## 14. 风险、回滚与恢复

| 风险 | 影响 | 缓解/回滚 |
|---|---|---|
| `required` provider 行为不一致 | 无工具响应或任务失败 | F2 probe 未通过即暂停，不落本地模拟 |
| 统一 schema 增加 Agent 误选 | hard-state failure 增加 | 记录错误率；回滚 F2 独立提交 |
| result 去重造成反馈缺失 | Agent 重试或状态误判 | call+delta+projection 对账 fixture；失败即回滚 F1 |
| continuation 扩大 blast radius | 部分执行或顺序错误 | candidate/preflight、首错停止、独立 F3 提交 |
| baseline 被误解为当前快照 | Agent 使用过期状态 | 明确 role/revision；其后只保留原始 typed delta，不注入解释 |
| epoch 锚点在 compaction 后失效 | history 错位或 marker 重复 | prefix hash 不一致即机械重建 epoch；marker count 强校验 |
| benchmark 随机性误判收益 | 错误接受优化 | 三次轮换，报告总和/均值/中位数和 trace outlier |

每阶段均为独立 commit，可通过普通 `git revert <commit>` 回滚；不保留并行旧 schema 或运行时 feature
fallback。实验项目不迁移旧数据。

## 15. Phase Gate

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| F0 | fixture + Phase E artifact reprocess | 不依赖 F1 | section report | 100% | completed |
| F1 | control fixture + simple/complex smoke | 不依赖 F2 | ownership/bytes report | 100% | completed |
| F2 | provider probe + schema/cache trace | 不依赖 F3 | one-schema + required HOLD report | 100% | completed |
| F3 | sequence regression + live adoption | 不依赖 F3.5 | request path report | 100% | completed |
| F3.5 | prefix fixture + simple/complex smoke | 不依赖 F4 | epoch/prefix/cache report | 100% | completed |
| F4 | full deterministic + Docker matrix | none | Phase F result doc | 100% | completed |

## 16. 决策记录

| Date | Decision | Reason |
|---|---|---|
| 2026-07-16 | Phase F 不做 projection 语义压缩 | 当前问题是所有权、请求和缓存形态，不是 Map 超限 |
| 2026-07-16 | `map_state` 从 control success result 移出 | 当前完整状态应由 projection 唯一持有 |
| 2026-07-16 | 不接受“schema 稳定但 named/auto 继续切换” | `tool_choice` 已被实跑证明属于 provider cache shape |
| 2026-07-16 | `required` 必须先 live probe | transport 支持不等于 provider/model 行为已验证 |
| 2026-07-16 | 每种策略独立验证与提交 | 防止收益和回归无法归因 |
| 2026-07-16 | freshness 先于 `map_state` 删除 | marker 唯一不能证明 projection 是当前状态 |
| 2026-07-16 | `required` 增加 thinking/质量门禁 | 当前 strict choice 会关闭 thinking，不能以成本换智能能力 |
| 2026-07-16 | continuation 只进入 bind 和有效绑定下的 mutation | complete/block 会清除 lease，rework/unblock 不建立 binding |
| 2026-07-16 | nested ordinary feedback 不复制进 outer control result | 原始 call/output 已独立可见，outer 复制只增加成本和事实载体 |
| 2026-07-16 | `required` 维持 HOLD | provider 在 thinking enabled 下明确拒绝，不能用缓存收益换思考能力 |
| 2026-07-16 | continuation 不复制完整普通工具 schema | 顶层工具是参数契约 owner，continuation 只引用同一调用信封 |
| 2026-07-16 | current ephemeral projection 改为 epoch baseline + canonical delta journal | 每轮尾部替换会结构性破坏消息前缀；原始 delta 已足以忠实推进当前状态 |
| 2026-07-16 | 不通过压缩修复缓存 | projection 大小不是前缀 0% 的根因，F3.5 不引入语义裁剪 |
| 2026-07-16 | control parser 必须消费完整 JSON 文档 | Runtime 不截断或静默修复 Agent 的 malformed 参数；执行、Event Store 与 replay 必须一致 |
