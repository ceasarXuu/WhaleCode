# Subagent VS Review: R7.1 原子执行计划

- Created: 2026-07-31T04:17:18+08:00
- Updated: 2026-07-31T04:27:50+08:00
- Report schema: adversarial-v1
- Task: 审查 R7.1 是否已拆成工程边界明确、可独立验证的小主题，避免 Phase 聚合造成工程混乱
- Report path: `vs_review/2026-07-31-r7-1-atomic-execution-plan-review.md`
- Review mode: fresh internal subagent
- Source session policy: no inherited main-agent context
- Review target commit: `cfffa1fe0c578aff28e83a5f4f94e30781fad2c1`
- Status: blocked

## Round 1: 原子边界、依赖与机器门禁

### Review Input

#### Objective

验证当前 21 个 R7.1 Phase 是否各自只有一个根因域、一个主要工程改动域和一套不依赖未来 Phase 的关闭证据；
路线决策、实现、复验、成本和晋升是否真正分离。

#### Review Target

R7.1 当前权威执行计划、里程碑映射和原子计划机器门禁。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md`
- `docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md`
- `scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`
- `scripts/taskspace-benchmark/test-r7-five-layer-contracts.ps1`
- `vs_review/2026-07-30-r7-1-w0-factual-foundation-review.md`

#### Change Introduction

上一版使用 7 个阶段和阶段内小数任务。当前版本取消小数任务，改为 `R71-01` 至 `R71-21` 的同级 Phase，
并为每个 Phase 声明入口、唯一改动域、非目标、产物、收益、独立验收、退出和回退。

#### Risk Focus

- 一个 Phase 是否仍聚合多个可独立修复的根因或多个生产所有权模块；
- “验证多个行为”是否被包装成单一验收，但实际上需要不同候选或不同关闭结论；
- 决策 Phase 是否夹带实现倾向，条件 Phase 是否会产生不可执行或无法关闭的状态；
- 依赖是否循环、缺边、错误串行，或要求未来 Phase 才能证明当前 Phase；
- 批次并行规则是否与“每次只把一个 Phase 标记实施中”自相矛盾；
- 机器门禁是否只检查文字存在，无法阻止聚合、错依赖、状态漂移或编号遗漏；
- 10 个缺陷根因到 21 个 Phase 的映射是否丢失问题、重复计算或制造无责任单元。

#### User-Perspective Review Focus

- 未来接手工程师能否从当前文档唯一判断下一项工作、停止点和验收方式；
- 用户需要决策时是否能明确知道在决定什么，而不会在实现后才被告知；
- 状态和依赖表达是否容易误解为所有 21 项必须严格串行。

#### Implementation Completeness Focus

- 每个实现 Phase 是否能定位到生产改动边界，而不只是协议、文档或测试脚手架；
- 每个测量/复验 Phase 是否有真实 evidence entry、可重算 artifact 和 fail-closed 资格；
- 门禁是否验证 Phase 全集、必填合同、关键拆分和里程碑同步，而非只匹配标题。

#### Target Benefit Focus

- 声称收益：减少跨主题提交、回归互相覆盖和无法归因的修复；
- 基线：上一版 7 个聚合阶段、10 个小数任务；
- 目标：每个当前编号可独立实现、验证、回退和关闭；
- 方法：静态计划审查、依赖反例、门禁负向能力检查；
- 本轮不把未来 Runtime 性能或 sample 成本视为已实现收益。

#### Assumptions To Attack

- “同一文件或同一 adapter”天然等于一个原子工程主题；
- “一个表格行”天然等于一个可独立关闭 Phase；
- 条件 Phase 可以在没有预先定义关闭语义时自然收敛；
- 全量区间依赖不会隐藏过度串行或循环；
- 文本必填字段检查足以防止后续计划重新聚合。

#### Adversarial Lenses

- requirements
- architecture
- state
- failure
- maintenance
- testing
- observability
- comprehension
- implementation-completeness
- target-benefit

#### Verification Status

- `test-r7-unified-execution-plan.ps1`: PASS
- `test-r7-five-layer-contracts.ps1 -Phase All`: PASS
- `cargo test -p codex-core context::taskspace_contract --lib`: 2 passed
- `git diff --check`: PASS
- 未执行真实 sample；本轮未改变 Agent/Runtime 行为

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Try to falsify atomicity and independent-verification claims.
- Cite evidence paths and line numbers.
- Return findings in the report output contract below.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User approval requested: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one 10-minute extension when alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | 当前最高风险是 Phase 边界、依赖方向和长期可执行性，而非生产代码正确性 | 聚合、分叉、循环依赖、不可独立关闭、门禁失真 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb4ac-f6af-7e42-b89b-15147190f95a` | spawn result + completion notification | `fork_context=false` | Round 1 Review Input | main-agent history、reasoning、drafts、conclusions、full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round1-architecture | architecture-adversary | 1 | `019fb4ac-f6af-7e42-b89b-15147190f95a` | 10 minutes | completed | reviewer returned before timeout | completed |

### Reviewer Outputs

#### round1-architecture

##### Summary

**Verdict: BLOCKED。**

“21 个 Phase 均可按单根因域、单主要改动域、独立证据关闭”的主张被证伪：

1. `R71-16` 将模型 multi-Patch 尝试行为和 Runtime 执行安全边界合并验收，失败后错误回退到 `R71-08`；
2. `R71-13/R71-14` 的“不触发关闭”与表中唯一退出证据矛盾，条件分支没有机器可判定的关闭结果；
3. 原子计划门只验证 ID、字段和固定文本存在，不能发现上述依赖、状态和责任矛盾。

人工核对迁移表后，历史 `GI-001` 至 `GI-010` 均有当前责任单元，没有根因漏项；
`GI-003/GI-006/GI-007/GI-008` 的一对多映射也是显式的。问题在于部分映射后的失败路由和关闭语义仍不闭合。

##### Blocking Findings

###### ARCH-B01：multi-Patch 复验把行为失败错误归责给 Runtime 边界

- Broken assumption：`R71-16` 的任何失败都表示 `R71-08` nested hard boundary 仍需返修。
- Failure scenario：Agent 在顶层 response 生成两个 Patch；preflight 正确拒绝且零 Patch 执行。此时
  `R71-08` 已正确，但 `R71-16` 因 attempt 非零失败，并要求回到 `R71-08`。
- Trigger：`multi_patch_attempt_count > 0`、`executed_patch_count = 0`，且调用来自 top-level。
- Impact：形成 `R71-08 -> R71-16 -> R71-08` 错误循环；可能诱导 Runtime 增加本不应承担的 Agent
  行为修复，破坏决策、实现、复验分离。
- Proof/fix needed：将 attempt、preflight rejection、execution 和 top-level/nested 分别建模。只有证明
  nested 调用绕过硬门时才能返修 `R71-08`；顶层 attempt 持续存在时，应暂停并新增独立诊断、决策、实现 Phase。
- Evidence：`47-r7.1-global-issue-register.md:260`、`:268`、`:270`；
  `47-r7.1-global-issue-register-legacy.md:225`；
  `2026-07-30-r7-1-w0-factual-foundation-review.md:1340`。

###### ARCH-B02：条件 Phase 的“不触发关闭”没有一致退出证据

- Broken assumption：`R71-13/R71-14` 在“不复现”分支仍能满足统一的 `已关闭` 定义。
- Failure scenario：`R71-12` 判定问题消失。正文允许 `R71-13/R71-14` 以“不触发”关闭，但总表仍要求
  `R71-13` 提供“根因记录与用户决策”、`R71-14` 提供“schema identity 与 live trace”。这些证据在未决策、
  未实现分支不可能存在。
- Trigger：`R71-12.outcome = not_reproduced`。
- Impact：选择 `已关闭` 会违反“独立关闭标准全部成立”；不设为 `已关闭` 又使 `R71-18` 的
  `01～17 全部关闭` 永远不可达。接手者也无法仅从状态判断是否存在生产实现。
- Proof/fix needed：增加机器可读的 `closure_outcome=implemented|not_triggered`；为不触发分支定义独立证据
  `R71-12 sealed no-reproduction verdict`；明确下游依赖接受哪种 outcome。
- Evidence：`47-r7.1-global-issue-register.md:38`、`:65`、`:224`、`:233`、`:236`、`:344`。

###### ARCH-B03：机器门禁不能证明原子性、依赖闭合或状态一致

- Broken assumption：原子计划门 PASS 表示 21 个 Phase 的依赖、关闭语义和责任边界有效。
- Failure scenario：当前文档同时包含 ARCH-B01、ARCH-B02，但门禁仍 PASS。脚本只检查固定 ID 顺序、
  九个字段标签和三段固定依赖文本，不解析表中依赖、状态、退出证据或迁移关系。
- Trigger：修改某一表格依赖、写入非法状态、删除某根因映射，或令表格与正文冲突，同时保留被搜索的固定文本。
- Impact：文档可重新聚合、形成循环或孤儿责任单元而持续绿灯；五层合同门仅调用该弱门禁，不能补足语义检查。
- Proof/fix needed：建立单一结构化 Phase 事实源；验证依赖 DAG、允许状态、条件 outcome、表体一致、
  10 根因覆盖、唯一实施中 Phase、新 Phase 到晋升门的可达性及退出证据类型；增加负向 mutant fixtures。
- Evidence：`test-r7-unified-execution-plan.ps1:34`、`:40`、`:51`、`:76`、`:83`；
  `test-r7-five-layer-contracts.ps1:87`。

##### Non-blocking Risks

###### ARCH-N01：新增 Phase 可能成为 R71-18 的孤儿前置

`R71-15/R71-17` 允许发现新根因后新增 Phase，但 `R71-18` 固定依赖 `01～17`，门禁也硬编码 21 个 ID。
新增 `R71-22` 时必须人工修改依赖和门禁，否则它可能不阻断成本及晋升。

Evidence：`47-r7.1-global-issue-register.md:257`、`:282`；`test-r7-unified-execution-plan.ps1:35`。

###### ARCH-N02：R71-12 的多根因结果缺少明确转移

`R71-12` 同时复验初始化、ordinary-only、control-first、finish+next，退出仅允许“消失”或“唯一根因”。
通用规则要求第二根因拆分，但没有说明如何关闭或替换已执行的 `R71-12`。建议定义
`multiple_roots_detected -> pause + create phases + rerun R71-12`。

Evidence：`47-r7.1-global-issue-register.md:218`、`:221`、`:362`。

###### ARCH-N03：两轮 held-out 的隔离身份未定义

`R71-18` 已运行 held-out repeat-3，`R71-20` 又运行“额外 held-out”。若复用样本，正式评测发生暴露；
若使用不同样本，计划没有声明两套 sealed identity。建议冻结 `engineering_held_out_set` 和
`promotion_held_out_set`，并禁止前者替代后者。

Evidence：`47-r7.1-global-issue-register.md:284`、`:308`。

##### User-Perspective Checks

| 维度 | 结论 |
|---|---|
| Usability | 当前主项 `R71-01` 明确，但同时允许 `R71-01～07` 并行；缺少机器生成的 ready queue，接手者仍需手算可执行项 |
| Ease of use | 全局停止条件、冻结前置和晋升禁令清楚，可操作性较好 |
| Ease of understanding | 决策、实现、复验的大结构清楚；但“不触发即关闭”和 multi-Patch 的 attempt/execution 混用会让接手者无法唯一判断停止点和责任人 |
| User decisions | `R71-07`、条件 `R71-13`、`R71-21` 均显式要求用户决定；ARCH-B02 修复前，`R71-13` 的不触发路径仍有歧义 |

##### Implementation Completeness Checks

| Phase | 生产路径或产物 | 测试/日志证据 | 状态 | Finding |
|---|---|---|---|---|
| 01 | `r7-call-evidence.ps1`、`r7-state-failure-contract.ps1` | strict fixture、fresh B | 返修 | - |
| 02 | `r7-artifact-provenance.ps1`、`r7-final-status-provenance.ps1` | exact-value fixture、fresh C | 返修 | - |
| 03 | `provider_wire_trace.rs`、Observer classifier | layer identity fixture | 返修 | - |
| 04 | freshness、performance observation/report | stale/ineligible 对偶 | 返修 | - |
| 05 | `client.rs` epoch producer 与 wire/cache/benchmark consumers | producer/consumer identity | 待实施 | - |
| 06 | Runtime `state.rs`、`taskspace_store.rs` | invalid/valid Store fixture | 待实施 | - |
| 07 | 决策文档 | 外部证据、用户批准 | 待决策 | - |
| 08 | nested tool config、CodeMode、preflight | top/nested 对偶、零执行 | 待实施 | ARCH-B01 邻接 |
| 09 | `taskspace_control_output.rs` | carrier fixture、fresh trace | 返修 | - |
| 10 | sequence/response/control output | revision integration | 待实施 | - |
| 11 | sequence、session response、provider wire | role snapshot、cache repeat-3 | 待实施 | - |
| 12 | sealed benchmark rerun/report | 同候选 repeat-3 | 待复验 | ARCH-N02 |
| 13 | 条件决策记录 | 触发分支有证据；不触发证据缺失 | 待判定 | ARCH-B02 |
| 14 | 获批组件，当前未确定 | 实现分支有测试；不触发证据缺失 | 待判定 | ARCH-B02 |
| 15 | reservation attempt report | lifecycle repeat-3 | 待复验 | ARCH-N01 |
| 16 | Patch observation/trace | attempt/reject/execute 未分离 | 待复验 | ARCH-B01 |
| 17 | provider wire/cost ledger | byte/token 重算 | 待准入 | ARCH-N01 |
| 18 | benchmark harness/report | repeat-3、held-out | 待准入 | ARCH-N03 |
| 19 | candidate/environment manifest、attestation | hash/digest gate | 待准入 | - |
| 20 | four-arm runner/report | repeat-10、promotion held-out | 待准入 | ARCH-N03 |
| 21 | 用户决策记录 | sealed matrix 引用 | 待决策 | - |

##### Target Benefit Checks

| 目标 | 基线 | 目标/方法 | 当前证据 | 状态 |
|---|---|---|---|---|
| 证据可信度 01～04 | B/C/H 已证明 parser、provenance、分类、eligibility fail-open | 负向 fixture + fresh shard | 只有 blocker，无关闭证据 | BLOCKED |
| capability epoch 05 | 当前使用 `current_window_id` | profile/provider/tools identity 对偶测试 | Shard G blocker | OPEN |
| Store hydrate 06 | 非法 Map 可进 cache | invalid/valid resume/fork/child | Shard F blocker | OPEN |
| nested 安全 07～08 | nested control/Patch 绕过 preflight | top/nested 原子测试 | Shard E blocker | OPEN |
| feedback 09～11 | violation 重复、revision 双权威、receipt 破坏 cache | fixture、revision integration、cache repeat-3 | 历史因果明确，尚未修复 | OPEN |
| 行为 12～16 | initialization/control/lifecycle/multi-Patch 不稳定 | sealed repeat-3 | 未产生新候选证据 | OPEN |
| 成本 17～18 | 固定增量约 1,493 token/request，动态成本受拒绝与 cache 污染 | ledger + repeat-3/held-out | 旧数据不可晋升 | OPEN |
| 晋升 19～21 | 无冻结候选、无 repeat-10 | manifest + sealed matrix + 用户决策 | 未开始 | NOT READY |

##### Required Fixes

- 重写 `R71-16` 的指标和失败路由，严格区分模型 attempt、preflight rejection、实际执行及
  top-level/nested 来源；
- 为 `R71-13/R71-14` 增加明确的条件关闭 outcome、替代退出证据和下游依赖语义；
- 将 Phase 数据迁入单一结构化事实源，并让门禁验证 DAG、状态、条件分支、根因覆盖和退出证据，而非检查固定文本。

##### Missing Tests

- 条件分支两路径：`reproduced` 与 `not_reproduced` 的状态、证据及下游可达性；
- multi-Patch 四象限：attempt 0/>0、execute 0/>0，并分别覆盖 top-level/nested；
- 依赖图负向测试：循环、未来依赖、未知 ID、关闭但前置未关闭；
- 根因迁移测试：`GI-001..010` 恰好全部覆盖，A2-D 只能映射发布 Phase；
- 新增 Phase 后自动阻断 `R71-18/19` 的测试；
- 两套 held-out identity、未提前运行和 seal 不可互换测试。

##### Missing Logs / Observability

通用规则要求结构化日志，但各实现 Phase 未指定事件名和断言位置。最低需补：

- R71-05：old/new epoch、profile、provider capability hash、tools hash、change reason、request identity；
- R71-06：map/revision/schema、失败 invariant、cache/handle mutation=false；
- R71-08：entry kind、nested depth、manifest actions、reject reason、dispatch/commit count；
- R71-10：prepare revision、final revision、control call、attribution count；
- R71-11：carrier role、receipt count、cache segment、revision identity；
- R71-14：合同版本、action count/order、ordinary schema hash、preflight outcome；
- 计划层：Phase status、closure outcome、evidence artifact、candidate commit、dependency snapshot。

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | ARCH-B01 | top-level Agent attempt 非零但 Runtime 正确零执行时，R71-16 仍错误返修 R71-08 | blocking | accept | `R71-16` 的退出与回退没有区分 attempt/reject/execute 和入口；反例成立 | 本轮只记录，不修改被审计划 | 先拆分行为诊断与边界复验，再执行 fresh closure review |
| architecture-adversary | ARCH-B02 | `not_reproduced` 分支无法同时满足 R71-13/R71-14 总表证据和 `已关闭` 定义 | blocking | accept | 总表与正文的条件退出确有互斥要求 | 本轮只记录，不修改被审计划 | 设计单一 closure outcome 合同和两分支证据 |
| architecture-adversary | ARCH-B03 | 字段存在性 PASS 被误当作 DAG、状态和责任边界有效 | blocking | accept | 当前脚本不解析依赖、状态、迁移或退出证据；B01/B02 共存时仍 PASS | 本轮只记录，不修改被审计划 | 设计单一结构化事实源；避免 Markdown/manifest 双权威 |
| architecture-adversary | ARCH-N01 | 新编号不会自动成为成本与晋升前置 | non-blocking | accept | 固定区间和硬编码 21 个 ID 不能覆盖未来编号 | 记录为 B03 的验收反例 | 结构化门禁增加所有开放修复到晋升门可达性 |
| architecture-adversary | ARCH-N02 | R71-12 只会得到零或一个根因 | non-blocking | accept | 当前同时观测四类动作异常，多根因结果现实可达 | 记录为条件状态机补充项 | 增加 `multiple_roots_detected` 转移与 rerun 规则 |
| architecture-adversary | ARCH-N03 | 两次 held-out 天然隔离 | non-blocking | accept | 当前没有两套 sample identity 和不可互换 seal | 记录为评测设计缺口 | 区分 engineering 与 promotion held-out identity |
| architecture-adversary | Missing logs / observability | 通用“结构化日志”声明足以指导各 Phase 验收 | non-blocking | accept | 关键事件名、字段和断言位置尚未落到具体实现 Phase | 记录为各 Phase 实施前补全项 | 随对应 Phase 设计事件合同，不建立平行日志架构 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending after ARCH-B01/B02/B03 fixes
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: ARCH-B01、ARCH-B02、ARCH-B03 已接受且尚未修复
- Allowed to proceed: no

## Final Conclusion

当前原子化方向正确，历史 10 个缺陷根因也没有漏项，但该版本还不能作为实施权威继续推进。必须先修复
multi-Patch 失败归属、条件 Phase 关闭语义和机器门禁三项 blocking，再由新的
`architecture-adversary` 只读复审关闭；本轮未修改被审计划，因此不能标记 passed。
