# R6 Phase F 上下文唯一性与成本收敛实施计划

- Created: 2026-07-16
- Updated: 2026-07-16
- Version: 1.0
- Status: In Progress
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

## 10. Phase F4：正式验证

### 10.1 Deterministic

- `codex-tools` schema 与 registry plan；
- `codex-core` control args/handler/output/sequence/provider contract；
- `codex-protocol` result/event wire；
- replay、resume、fork、terminal transaction；
- benchmark cost/performance observer 与 Docker harness；
- `cargo build -p codex-cli --bin whale --locked`。

### 10.2 Docker Live

每个策略完成后先各跑 simple/complex 1 次；F4 再执行：

- simple：Standard/R6 各 3 次，左右轮换；
- complex：Standard/R6 各 3 次，左右轮换；
- 固定 model、prompt、validator、hidden oracle、Docker hard boundary；
- 报告结果、动作、request、wall、input/cached/uncached/output、section cost、Map 和 terminal proof。

### 10.3 总退出门禁

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

## 11. 实现完整性矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| F0 section trace | 请求组成可归因 | `provider_wire_trace.rs`, `client.rs` | provider request | cache trace tests | section hash/bytes | none | planned |
| F1 result delta | 当前 Map 单 owner | `taskspace_control_output.rs` | tool result | handler/output tests | control result trace | none | planned |
| F2 stable contract | schema/choice 全 turn 稳定 | `session/turn.rs`, `taskspace_tool.rs` | Prompt | provider contract tests | tools/choice hash | provider live probe | planned |
| F3 continuation | Agent 声明序列机械执行 | args/schema/sequence | tool router | sequence tests | step/skipped refs | none | planned |
| F4 live gate | 成本与正确性可比较 | benchmark scripts | Docker harness | harness tests | performance report | none | planned |

## 12. 变更链日志

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| payload build | section measured | section hashes/bytes complete | section unavailable | `unavailable_reason` | request/epoch id | info | observer |
| control execute | validated/committed | revision+delta | protocol/state/resource failure | class/code | call/map/revision | info/warn | Agent/observer |
| sequence | started/executed/stopped | ordered step refs | nested failure/skipped tail | failure code | parent/nested call id | info | Agent/observer |
| provider choice | required sent | tool call returned | no-tool/API rejection | provider error code | request id | info/error | Runtime/observer |
| terminal | committed/published | carrier+hash | open Map response | terminal protocol code | turn/map/revision | info/error | CLI/observer |

## 13. 风险、回滚与恢复

| 风险 | 影响 | 缓解/回滚 |
|---|---|---|
| `required` provider 行为不一致 | 无工具响应或任务失败 | F2 probe 未通过即暂停，不落本地模拟 |
| 统一 schema 增加 Agent 误选 | hard-state failure 增加 | 记录错误率；回滚 F2 独立提交 |
| result 去重造成反馈缺失 | Agent 重试或状态误判 | call+delta+projection 对账 fixture；失败即回滚 F1 |
| continuation 扩大 blast radius | 部分执行或顺序错误 | candidate/preflight、首错停止、独立 F3 提交 |
| benchmark 随机性误判收益 | 错误接受优化 | 三次轮换，报告总和/均值/中位数和 trace outlier |

每阶段均为独立 commit，可通过普通 `git revert <commit>` 回滚；不保留并行旧 schema 或运行时 feature
fallback。实验项目不迁移旧数据。

## 14. Phase Gate

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| F0 | fixture + Phase E artifact reprocess | 不依赖 F1 | section report | 100% | pending |
| F1 | control fixture + simple/complex smoke | 不依赖 F2 | ownership/bytes report | 100% | pending |
| F2 | provider probe + schema/cache trace | 不依赖 F3 | one-shape report | 100% | pending |
| F3 | sequence regression + live adoption | 不依赖 F4 | request path report | 100% | pending |
| F4 | full deterministic + Docker matrix | none | Phase F result doc | 100% | pending |

## 15. 决策记录

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
