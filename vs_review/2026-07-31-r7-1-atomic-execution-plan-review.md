# Subagent VS Review: R7.1 原子执行计划

- Created: 2026-07-31T04:17:18+08:00
- Updated: 2026-07-31T07:00:03+08:00
- Report schema: adversarial-v1
- Task: 审查 R7.1 是否已拆成工程边界明确、可独立验证的小主题，避免 Phase 聚合造成工程混乱
- Report path: `vs_review/2026-07-31-r7-1-atomic-execution-plan-review.md`
- Review mode: fresh internal subagent
- Source session policy: no inherited main-agent context
- Review target commit: `cfffa1fe0c578aff28e83a5f4f94e30781fad2c1`
- Status: open

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

## Round 2: ARCH-B01/B02/B03 blocking closure

### Review Input

#### Objective

只复审 Round 1 接受的三个 blocking 是否在提交
`e6f6ffc0c79542f4b7928534a3e256cd25b1a10d` 中真正关闭，并检查修复是否引入新的同级架构问题。

#### Review Target

- ARCH-B01：multi-Patch attempt、preflight rejection、execution、top-level/nested 的责任分离；
- ARCH-B02：删除条件占位或建立一致的关闭结果和下游可达性；
- ARCH-B03：单一结构化事实源、DAG/状态/根因/晋升门语义验证与负向 fixture。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `benchmarks/taskspace/r7/r7-1-execution-plan-v1.schema.json`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-contract.ps1`
- `scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`
- `docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md`
- Round 1 findings in this report

#### Change Introduction

- 删除未触发的 response-action 决策/实现占位 Phase；
- 将 multi-Patch 拆为 Runtime 安全边界复验与 Agent 行为诊断；
- 在同一 Markdown 权威文档中嵌入唯一 JSON 机器合同，读者表为受校验投影；
- 增加 JSON Schema、语义 validator 和 13 类负向 mutant fixture；
- 区分 engineering/promotion held-out identity，并为各 Phase 定义证据事件及字段。

#### Risk Focus

- multi-Patch 失败是否仍错误返修 Runtime，或只是在文字上拆分；
- 诊断 `root_causes_identified` 是否能独立关闭且确保新增 Phase 阻断成本/晋升；
- 嵌入 JSON 与读者说明是否形成事实重复或无法同步；
- graph validator 是否会漏掉循环、未来依赖、未知 ID、孤儿 blocker 和过早决策；
- 新 Phase 插入与重编号规则是否可执行；
- 负向 fixture 是否真的攻击 validator，而不是固定文本自证；
- 修复是否超过用户要求，形成新的计划管理子系统或不必要复杂度。

#### Verification Status

- `test-r7-unified-execution-plan.ps1`: PASS
- `test-r7-five-layer-contracts.ps1 -Phase All`: PASS
- `cargo test -p codex-core context::taskspace_contract --lib`: 2 passed
- `git diff --check`: PASS
- 本轮没有 Agent/Tool/Runtime 行为变化，因此未运行真实 sample

#### Reviewer Instructions

- Fresh internal subagent session，`fork_context=false`。
- 不继承 Round 1 reviewer 上下文；直接读取 Round 1 findings 与目标提交。
- 只读，不修改文件。
- 每个 blocking 必须给出 `closed` 或 `still_open`，并提供反例与路径行号。
- 若发现新的 blocking，必须说明是否由本轮修复引入。
- 不把风格偏好或未来产品收益未实现升级成 blocking。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | one 10-minute extension when alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | Round 1 blocking 均属于责任边界、依赖和事实源设计，需要 fresh 同角色复核 | B01/B02/B03 closure 与新架构回归 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb4f4-bbb0-7c62-adc9-afcf2681cc22` | spawn result + completion notification | `fork_context=false` | Round 2 Review Input | main-agent history、reasoning、uncommitted drafts、Round 1 reviewer context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round2-architecture | architecture-adversary | 1 | `019fb4f4-bbb0-7c62-adc9-afcf2681cc22` | under 20 minutes | completed | reviewer returned before timeout | completed |

### Reviewer Outputs

#### round2-architecture

##### Summary

**Verdict: BLOCKED。**

- ARCH-B01 `closed`：multi-Patch Runtime 安全与 Agent 行为已经拆成不同 Phase；
- ARCH-B02 `closed`：不可关闭的条件占位 Phase 已删除；
- ARCH-B03 `still_open`：结构化事实源已建立，但语义 validator 仍信任可篡改声明值；
- 新增两个同级 blocker：特殊角色可以重绑定；关闭证据可以用任意非空字符串伪造。

##### Blocking Findings

###### ARCH-B03：图与状态约束仍可被构造性绕过

审查者实际构造并通过了以下 mutant：

1. R71-12 的派生 Phase 指向已有祖先 R71-01；
2. R71-07 未关闭时将 R71-08 标记为 `in_progress`；
3. 清空 blocker 布尔量并删除依赖，使开放根因单元脱离成本/晋升路径；
4. 工程说明改成失败后回退 R71-08，机器合同仍通过。

所需修复：派生修复必须带根因映射和父诊断反向引用；active/current 必须 ready；成本和晋升阻断必须从
DAG 推导；失败路由与禁止目标必须机器化。

###### ARCH-NB01：特殊 Phase 角色可被重新绑定

将 `promotion_decision_phase_id` 指向 R71-19，并同步清空声明 blocker 后，validator 仍可接受，绕过用户
产品晋升决策。所需修复：特殊角色 ID 唯一、绑定正确 kind，并形成严格
dynamic-cost -> freeze -> formal-evaluation -> promotion 末端链。

###### ARCH-NB02：evidence_artifact 不证明真实证据存在

当前只要求非空字符串；所有 Phase 可引用 `missing://...` 后关闭。所需修复：证据引用至少包含仓库相对路径、
SHA-256 和证据 schema/version，门禁必须验证真实文件与摘要。

##### Non-blocking Risks

- held-out 只验证 identity 字符串，不验证 sealed sample manifest、摘要和集合不相交；
- observability event name 未验证唯一；
- 缺少新增 Phase 插入、重编号并保持晋升门闭合的正向 fixture；
- 状态变化缺少统一 transition audit event；
- 读者投影未直接暴露 ready、closure outcome 和 spawned repair；
- `current_phase_id` 只验证存在，没有验证 open + ready；
- 工程说明中的失败目标没有与机器合同绑定。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| ARCH-B01 | accept closed | R71-14/15 已分离 Runtime safety 与 Agent behavior | 保持拆分，不再改回聚合 Phase | Round 3 复核 |
| ARCH-B02 | accept closed | 条件占位已删除，诊断用明确 closure outcome 独立关闭 | 保持当前删除结果 | Round 3 复核 |
| ARCH-B03 | accept | reviewer 四个 mutant 均暴露 validator 信任声明值 | 删除 mutable blocker；新增 ready、派生根因/父诊断、DAG 祖先和机器 failure route 校验 | Round 3 复核 |
| ARCH-NB01 | accept | 特殊 ID 缺少 role/kind/顺序绑定 | 校验四个角色唯一、kind 正确、严格末端直连且 promotion 为最终 Phase | Round 3 复核 |
| ARCH-NB02 | accept | 非空字符串不能证明 evidence 存在 | evidence 改为 path + SHA-256 + schema_version，并校验仓库内真实文件 | Round 3 复核 |
| held-out 隔离 | accept | 两个 label 不足以证明样本未复用 | 增加 sealed sample manifest 引用、摘要和 sample ID 不相交校验 | Round 3 复核 |
| event/transition | accept | 当前 event identity 可重复，状态改变不可审计 | event name 唯一；新增统一 state transition audit contract | Round 3 复核 |
| insertion fixture | accept | 固定 20/R71-20 的测试无法证明扩展安全 | 增加插入 Phase、全引用重编号并通过 21 Phase 合同的正向 fixture | Round 3 复核 |
| reader/current | accept | 接手者仍需手算 ready，current 可指向 blocked Phase | 投影增加 Ready/关闭结果/派生修复；current 强制 open + ready | Round 3 复核 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 2 Conclusion

ARCH-B01/B02 已关闭；ARCH-B03 仍开放，并新增 ARCH-NB01/NB02。主 Agent 已接受并实施第二轮修复，
但依据 review hard rule，必须由新的 fresh reviewer 完成 Round 3 后才能关闭报告。

## Round 3: Validator bypass closure

### Review Input

#### Objective

只验证提交 `0d5a02f9c` 是否关闭 Round 2 的 ARCH-B03、ARCH-NB01、ARCH-NB02，并确认修复没有建立
平行事实源、不可维护的固定编号合同或虚假 evidence 证明。

#### Review Target

- Phase DAG、ready/current、诊断派生修复和成本/晋升祖先关系；
- dynamic cost、candidate freeze、formal evaluation、promotion 特殊角色；
- evidence path、SHA-256、artifact type、schema/version 与 held-out sample 隔离；
- machine failure route、读者投影、状态转移审计和插入 Phase 正向 fixture。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `benchmarks/taskspace/r7/r7-1-execution-plan-v1.schema.json`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-contract.ps1`
- `scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`
- Round 2 findings and responses in this report

#### Change Introduction

- 删除 `blocks_dynamic_cost` / `blocks_promotion`，从编号位置和 DAG 祖先关系推导阻断；
- 特殊角色绑定 kind、最终四段直接依赖链和最终 promotion 位置；
- `in_progress`、`decision_pending`、`current_phase_id` 必须 open + ready；
- `spawned_repairs` 建立 root-cause/phase 映射，派生 Phase 反向引用父诊断；
- evidence 改为仓库相对路径、SHA-256、artifact type、schema/version，并验证真实文件；
- held-out 使用 sealed sample manifest，验证内容摘要和 sample ID 不相交；
- failure route 声明禁止目标，工程说明的退出段不得回退禁止目标；
- 增加 20 类负向 mutant、held-out 正反 fixture 和 20 -> 21 Phase 插入/重编号正向 fixture。

#### Risk Focus

- 是否还能通过重绑定特殊 ID、改变 kind 或断开末端链绕过用户晋升；
- 是否存在早于 dynamic cost 但不是其祖先的 Phase；
- 派生 Phase 是否能指向自身、祖先、已关闭节点，或伪造 parent/root 映射；
- `in_progress` 或 current 是否仍能指向依赖未关闭的 Phase；
- 任意真实 JSON 是否能因自报 artifact type/schema 而冒充验收证据；
- held-out 是否只比较字符串，没有验证实际 sample 集合；
- failure route 是否只是另一段不受校验的文字；
- 正向插入 fixture 是否真正重编号全部引用，还是只迎合固定 20 Phase；
- helper/test 是否因本轮扩张形成超过必要复杂度或第二事实源。

#### User-Perspective Focus

- 接手者能否从读者投影直接判断 ready、关闭结果和派生修复；
- current 是否始终指向可实际推进的 Phase；
- 用户晋升决策是否仍是不可绕过的唯一最终节点；
- 验收证据错误时，失败原因是否能定位到路径、摘要、类型或依赖。

#### Implementation-Completeness Focus

- Schema、semantic validator、projection validator 和 mutant 是否覆盖同一合同；
- 测试是否包含 reviewer Round 2 的真实绕过方式；
- 插入新 Phase 是否自动阻断成本/晋升，并保持特殊链正确；
- 关闭证据和 held-out 引用是否读取真实文件而非只校验字符串。

#### Target-Benefit Focus

- 目标收益是计划门可以拒绝构造性绕过，不是声称 Runtime/Agent 性能提升；
- 基线为 Round 2 reviewer 成功通过的五类 mutant；
- 验证方法为同类负向 mutant、真实文件摘要反例和扩展正向 fixture；
- 本轮未改生产行为，因此 sample 性能不是关闭条件。

#### Assumptions To Attack

- “编号在成本 Phase 前”天然意味着依赖成本 Phase；
- 特殊角色有正确名字就不会被重绑定；
- 真实存在且 SHA 正确的 JSON 一定是正确类型的证据；
- parent 反向引用天然与 source 的 spawned mapping 一致；
- 两套 held-out manifest 不同就表示 sample 集合不重叠；
- 失败路由禁止目标不会在工程说明中重新出现；
- 单个插入 fixture 足以证明无固定 20 Phase 假设。

#### Adversarial Lenses

- architecture
- state
- failure
- data integrity
- maintenance
- testing
- observability
- implementation completeness
- user comprehension

#### Verification Status

- `test-r7-unified-execution-plan.ps1`: PASS
- `test-r7-five-layer-contracts.ps1 -Phase All`: PASS
- `cargo test -p codex-core context::taskspace_contract --lib`: 2 passed
- PowerShell parser: PASS
- `git diff --check`: PASS
- helper 499 lines；test 461 lines
- no production Agent/Runtime change; no sample run

#### Reviewer Instructions

- Fresh internal subagent session，`fork_context=false`；
- 直接读取提交和目标文件，不继承主 Agent 或 Round 2 reviewer 上下文；
- 只读，不修改文件；
- 优先亲自构造 mutant，不以现有测试 PASS 代替审查；
- 对 ARCH-B03/NB01/NB02 分别给出 `closed` 或 `still_open`；
- 新 blocking 必须给出可复现输入、影响和所需证明；
- 不把风格偏好或未实现的未来产品收益升级为 blocking。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | one 10-minute extension when alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | 本轮是接受 blocker 后的机器合同关闭审查，需要 fresh 独立 falsification | DAG、角色、证据、扩展与维护 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb50e-0f7e-7a72-a454-4f1e658e0879` | spawn result + completion notification | `fork_context=false` | Round 3 Review Input | main-agent history、reasoning、uncommitted drafts、Round 2 reviewer context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round3-architecture | architecture-adversary | 1 | `019fb50e-0f7e-7a72-a454-4f1e658e0879` | under 20 minutes | completed | reviewer returned before timeout | completed |

### Reviewer Outputs

#### round3-architecture

##### Verdict

**BLOCKED**

##### R3-B01：结构为空的 evidence 可以关闭任意 Phase

reviewer 构造了只有 `schema_version` 和 `artifact_type` 的真实 JSON，计算正确 SHA-256 后用于关闭 R71-02。
Schema、semantic、projection 和 definition-route validator 全部接受，输出
`FULL_TARGET_VALIDATORS_ACCEPTED_BOGUS_CLOSURE`。

根因：当前只验证 evidence reference envelope、文件存在性、摘要和文件自报的 type/version；
没有用权威 artifact schema 验证内容，也没有验证 Phase observability 所需字段。

影响：空元数据壳可以宣称实现、评测、冻结甚至晋升完成。SHA-256 只能证明内容身份，不能证明验收有效。

##### R3-B02：failure route 可通过角色重绑定和标题别名绕过

reviewer 将 `nested_boundary_pair` 从 R71-08 移到 R71-13，同步修改禁止 ID，并在 R71-14 工程说明中使用
“nested dispatcher 硬边界实现”标题而非 ID 表达回退。所有 validator 接受，输出
`FULL_TARGET_VALIDATORS_ACCEPTED_ROUTE_ROLE_REBIND`。

根因：受保护角色从可变、非唯一的 `acceptance_evidence_type` 推导；工程说明只搜索禁止 ID 字符串，
形成可用标题别名绕过的平行语义源。

影响：Runtime safety 失败仍可重新打开既有 Runtime 实现 Phase，恢复 Round 1 的责任循环。

##### Requested Closure Status

- ARCH-B03：`still_open`；DAG/ready/parent 部分已关闭，但 false evidence 和 failure route 仍可绕过；
- ARCH-NB01：`still_open`；reference identity 已建立，artifact 内容有效性仍是自报；
- ARCH-NB02：`still_open`；编号扩展已通过，failure routing 仍是机器与文字双源。

##### Positive Controls

特殊角色重绑定、promotion kind 漂移、末端链断开、成本前孤儿、blocked current/in-progress 和伪造父诊断均被拒绝。
reviewer 还在 R71-10 任意位置插入 Phase，生产 validator 正确接受 21 Phase 并重排成本与 promotion，
证明生产 validator 不依赖固定 20 个编号。

##### Non-blocking Risks

- 自带 insertion helper 只测试临近成本门插入，不测试任意位置；
- evidence 路径 containment 未处理 symlink canonicalization；
- state-transition audit 只验证声明，未校验已发出的 audit artifact/replay；
- 缺少 same-type/same-hash 但内容结构非法的 evidence mutant。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| R3-B01 | accept | reviewer 的空壳 artifact 在完整 validator 路径通过，反例可复现 | 为 Phase evidence 和 held-out 各建立固定权威 schema；reference 固定 schema path/version；Phase evidence records 必须满足 observability required fields | Round 4 fresh closure review |
| R3-B02 | accept | evidence label 不是稳定 routing role；substring 检查不能约束标题别名 | 新增独立 route role IDs 并绑定 kind/change domain；evidence type 全局唯一；删除 substring 门，改为机器生成 failure-route 读者投影和 canonical 工程引用 | Round 4 fresh closure review |
| 任意位置 insertion | accept passed | reviewer 的 R71-10 插入证明生产 validator 不硬编码 20 Phase | 保留生产逻辑；扩展自带正向 fixture 到任意插入位置 | Round 4 复核 |
| symlink containment | accept | lexical prefix 不能证明真实路径仍在 repo | evidence resolver 增加 symlink/reparse-point 拒绝 | Round 4 复核 |
| transition audit artifact | defer non-blocking | 本轮是静态计划治理；真实 transition 尚未发生 | 在首次状态变更实现时要求 audit artifact/replay，不以声明替代运行证据 | R71-01 状态变更前 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 3 Conclusion

图、状态、特殊末端链和编号扩展门禁有效，但 evidence 内容与 failure route 仍存在自证循环。两项 blocker
均接受，必须完成权威 artifact schema、稳定 route roles 和单一机器投影后再启动 fresh Round 4。

## Round 4: Evidence and failure-route closure

### Review Input

#### Objective

只验证提交 `fc9197080` 是否关闭 R3-B01/R3-B02，以及 ARCH-B03/NB01/NB02 是否可以最终关闭。

#### Review Target

- Phase evidence 固定 schema、required record fields、path/hash/type/version 和 symlink 防护；
- held-out 固定 schema 与 sample IDs；
- stable route role IDs、role kind/change-domain/dependency 绑定；
- `spawn_atomic_phase + existing_phase_reuse: forbidden`；
- 机器生成 phase/failure-route 投影与 canonical 工程引用；
- 任意位置 Phase 插入和 route role 重编号。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `benchmarks/taskspace/r7/r7-1-execution-plan-v1.schema.json`
- `benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json`
- `benchmarks/taskspace/r7/r7-held-out-set-v1.schema.json`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-contract.ps1`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-evidence.ps1`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-projection.ps1`
- `scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`
- Round 3 reviewer output and main-agent response in this report

#### Change Introduction

- evidence reference 的 schema path/version 只能指向固定权威 schema；
- Phase evidence 至少一条 record，且每条 record 必须包含该 Phase observability required fields；
- held-out 使用独立 schema，sample IDs 非空、唯一且两套集合不相交；
- evidence/schema 路径拒绝 symlink/reparse point；
- route roles 不再从 evidence label 推导，而由固定 role key 绑定 kind/change domain/依赖关系；
- failure route 不列举可重绑定 ID，统一禁止复用全部既有 Phase；
- failure-route 表和工程 exit/rollback 都从机器合同受控，不再 substring 搜索禁止 ID；
- 正向 fixture 同时覆盖临近成本门和 R71-10 任意位置插入。

#### Risk Focus

- 同 type/hash/version 但缺 required fields 的 artifact 是否仍能关闭 Phase；
- 改成未知或宽松 schema path/version 是否可通过；
- symlink 是否能绕过 repo containment；
- route role 是否能重绑到 R71-13，或通过修改 evidence label 改变受保护角色；
- title alias 是否能写入 exit/rollback 并绕过机器 route；
- `existing_phase_reuse` 是否存在允许值或投影再解释；
- 通用 evidence schema + Phase required-fields validator 是否形成可被空 record 绕过的组合；
- 新 helper 分层是否形成重复权威或固定 20 Phase 假设。

#### User-Perspective Focus

- 读者看到 closed 时是否至少有结构上满足本 Phase 验收字段的真实 artifact；
- 失败后是否只有“新增原子 Phase”一个路径，不会被文字引导回旧 Runtime Phase；
- ready、允许关闭结果、当前结果和派生修复是否仍可直接读取。

#### Verification Status

- `test-r7-unified-execution-plan.ps1`: PASS
- `test-r7-five-layer-contracts.ps1 -Phase All`: PASS
- PowerShell parser: PASS
- `git diff --check`: PASS
- all changed code files < 500 lines
- no production Agent/Runtime change; no sample run

#### Reviewer Instructions

- Fresh internal subagent，`fork_context=false`，只读；
- 直接重放 R3-B01 的空壳/same-type 缺字段 evidence 和 R3-B02 的 role-rebind/title-alias；
- 对 R3-B01、R3-B02、ARCH-B03、ARCH-NB01、ARCH-NB02 分别给出 closed/still_open；
- 新 blocking 必须包含可复现 mutant、影响和所需证明；
- 不将 value-level 业务真实性无法由静态结构完全证明本身视作 blocker，除非计划声称已证明该语义；
- 不修改文件。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | one 10-minute extension when alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | R3 accepted blocker 的独立 closure review | evidence validity、route identity、single source |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb522-f443-7453-9377-ec5f2badf5ff` | spawn result + completion notification | `fork_context=false` | Round 4 Review Input | main-agent history、reasoning、uncommitted drafts、Round 3 reviewer context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round4-architecture | architecture-adversary | 1 | `019fb522-f443-7453-9377-ec5f2badf5ff` | under 20 minutes | completed | reviewer returned before timeout | completed |

### Reviewer Outputs

#### round4-architecture

##### Verdict

**BLOCKED**

##### Blocking Finding

R3-B02 `still_open`：保留 canonical exit/rollback 后，再追加第二条冲突的同名 directive，完整统一门仍 PASS。
当前 validator 只验证 canonical 行存在，没有验证每个工程段落恰好只有一条 exit 和一条 rollback。

影响：机器表禁止复用既有 Phase，但同一读者段落可以额外指示重新打开 Runtime Phase，恢复平行语义源。

##### Closure Status

- R3-B01：`closed`；空 records、空 record、同类型缺字段、宽松 schema path、未知 schema version 均被拒绝；
- R3-B02：`still_open`；简单 role rebind、`existing_phase_reuse=permitted` 和替换式 alias 已拒绝，
  但 additive alias 仍通过；
- ARCH-B03：`closed`（本轮限定的图/状态/角色结构范围）；
- ARCH-NB01：`still_open`，由 R3-B02 additive prose 问题包含；
- ARCH-NB02：`closed`；计数、末端门和任意位置插入均由 manifest/DAG 推导。

##### Non-blocking Risks

- 协同重写所有身份事实仍能改变 route role，属于静态权威文件本身被整体改写，不单列 blocker；
- required fields 可填无意义非空值；当前合同只声明结构证据，不声明静态门可证明现实真实性；
- 尚未修改生产 Agent/Runtime，因此没有 sample 运行或实际性能收益。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| R3-B01 | accept closed | reviewer 重放空壳、缺字段、未知 schema 均失败 | 保持固定 schema + required fields 设计 | Round 5 最终复核 |
| R3-B02 additive alias | accept | canonical 存在性不能排除第二条冲突 directive | 每个 failure-route Phase 精确计数：exit=1、rollback=1，且值必须 canonical；新增 additive exit/rollback mutants | Round 5 最终复核 |
| ARCH-B03 | accept closed | DAG、ready、special/route role 和 held-out 反例均关闭 | 不扩大修改 | Round 5 最终复核 |
| ARCH-NB01 | accept | 唯一剩余来源是 additive prose | 由 exact-count 修复统一关闭 | Round 5 最终复核 |
| ARCH-NB02 | accept closed | 任意位置插入与末端链已通过 | 不扩大修改 | Round 5 最终复核 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 4 Conclusion

evidence、DAG、角色与扩展门已通过；只剩工程说明允许追加第二条冲突 exit/rollback。修复必须限定为
exact-count + exact-value，不再扩张合同。

### Round 4 Fix Verification

- `failure_route_additive_exit_alias`: rejected
- `failure_route_additive_rollback_alias`: rejected
- `test-r7-unified-execution-plan.ps1`: PASS
- `test-r7-five-layer-contracts.ps1 -Phase All`: PASS
- PowerShell parser: PASS
- `git diff --check`: PASS
- changed code files: all below 500 lines
- production Agent/Runtime behavior: unchanged, so no sample run

## Round 5: Additive failure-route closure

### Review Input

#### Objective

只验证提交 `11fb836c0` 是否关闭 Round 4 最后一个 blocker：保留 canonical exit/rollback 后追加第二条冲突
directive；并判定 R3-B02 与 ARCH-NB01 能否最终关闭。

#### Review Target

- 每个 failure-route Phase 的 `退出/分流` 必须恰好一条且值为 canonical；
- 每个 failure-route Phase 的 `回退` 必须恰好一条且值为 canonical；
- additive alias、重复 canonical 和替换式 alias 必须拒绝；
- 当前权威文档必须继续通过正向验证。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `scripts/taskspace-benchmark/lib/r7-execution-plan-projection.ps1`
- `scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`
- 本报告 Round 4

#### Scope Guard

本轮不重新扩大到已关闭的 evidence、DAG、特殊末端链和任意插入议题，除非发现明确回归。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| focused | 20 minutes | one 10-minute extension when alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | Round 4 accepted blocker 的独立 closure review | additive directive、single source |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb52a-7005-7930-b549-29738ff5a394` | spawn result + completion notification | `fork_context=false` | Round 5 Review Input | main-agent history、reasoning、uncommitted drafts、Round 4 reviewer context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round5-architecture | architecture-adversary | 1 | `019fb52a-7005-7930-b549-29738ff5a394` | under 20 minutes | completed | reviewer returned before timeout | blocker accepted |

### Reviewer Outputs

#### round5-architecture

##### Verdict

**BLOCKED**

##### Blocking Finding

`R3-B02` 与 `ARCH-NB01` 仍为 `still_open`。实现只统计列首 `- ` 的 directive；在 canonical 行后追加
Markdown 等价的 `* 退出/分流：...` 或 `* 回退：...`，完整统一合同仍 PASS。缩进列表和加粗字段名也可绕过。

这仍是 Round 4 的原 blocker：机器投影禁止复用旧 Phase，但工程段可保留 canonical 后追加第二条冲突
directive，形成平行语义源。

##### Reproduction

- 固定提交正向文档：PASS；
- 列首 `- ` additive exit/rollback：rejected；
- duplicate canonical exit/rollback：rejected；
- replacement alias：rejected；
- `*` additive exit/rollback：unexpected PASS；
- 缩进和强调变体：未进入 exact-count；
- 统一命令：`pwsh -NoLogo -NoProfile -File scripts/taskspace-benchmark/test-r7-unified-execution-plan.ps1`。

##### Minimum Closure

1. 唯一性检查与 Markdown marker、缩进和强调样式无关；
2. 唯一标签必须属于 exact canonical 行；
3. 持久化 `*`、缩进、强调和 duplicate canonical mutants；
4. fresh Round 6 重放后才能关闭。

reviewer 未扩大 evidence/DAG 范围，也未发现该范围回归。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| Markdown marker bypass | accept | `*` additive mutant 在固定提交的完整门禁中通过 | 改为段内语义标签唯一计数 + canonical 行精确存在，两项条件同时满足 | Round 6 fresh closure review |
| Mutant coverage gap | accept | 原测试只覆盖列首 `- `，未持久化 duplicate canonical | 表驱动加入 dash、star、indent、bold、duplicate 的 exit/rollback 共 10 个 mutants | Round 6 fresh closure review |
| R3-B02 / ARCH-NB01 | accept still open | Round 5 提供同 blocker 的可复现语法变体 | 修复完成但不自行关闭 | Round 6 fresh closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 5 Conclusion

Round 4 修复只关闭了单一 Markdown 形态，没有关闭同一语义标签的等价表达。门禁已收敛为标签唯一性和
canonical 行双重合同，必须由 fresh Round 6 再验证。

## Round 6: Structural failure-route closure

### Review Input

#### Objective

只验证提交 `70181a1f6` 是否结构性关闭 Round 5 blocker：每个 failure-route 工程段的 `退出/分流` 和
`回退` 标签各自唯一，且唯一标签必须存在于 exact canonical 行。

#### Required Mutants

- dash、star、indent、bold additive exit；
- dash、star、indent、bold additive rollback；
- duplicate canonical exit/rollback；
- replacement alias；
- 正向权威文档。

#### Scope Guard

可攻击合理的标点和 Markdown 包装变体，但不要求 Runtime 解析任意自然语言同义句；不重新扩大到已关闭的
evidence/DAG 范围，除非发现明确回归。

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb531-070c-7e11-9d3a-294dfb029d67` | spawn result + completion notification | `fork_context=false` | Round 6 Review Input | main-agent history、reasoning、uncommitted drafts、Round 5 reviewer context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round6-architecture | architecture-adversary | 1 | `019fb531-070c-7e11-9d3a-294dfb029d67` | under 20 minutes | completed | reviewer returned before timeout | blocker accepted |

### Reviewer Outputs

#### round6-architecture

##### Verdict

**BLOCKED**

- R3-B02：`still_open`
- ARCH-NB01：`still_open`
- 不允许关闭整轮审查。

##### Blocking Finding

在 canonical rollback 后追加 `- 回**退**：重新打开既有 Phase。`，Markdown 可见标签仍为“回退”，但
原始文本不存在连续 `回退` 子串，因此 validator 得到 `rollback_raw_label=1`、`canonical_line=1` 并接受。
partial italic、全角/空格/Markdown 转义斜杠也存在同类绕过。

##### Verification

- 固定提交 `70181a1f6`；
- unified contract：PASS；
- five-layer contract：PASS；
- 规定的 dash/star/indent/bold、duplicate canonical 和 replacement alias 均按预期拒绝；
- 扩展格式攻击有 6 项意外接受；
- 未发现 evidence/DAG 明确回归。

##### Minimum Closure

1. 唯一性基于 Markdown 规范化后的可见标签，而非原始连续子串；
2. 原始 exact canonical 行仍必须恰好一次；
3. 持久化 partial emphasis 和合理斜杠变体；
4. fresh closure review。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| inline Markdown label bypass | accept | `回**退**` 在 Markdown 中保持同一可见标签，但 raw substring 计数失效 | 增加机械可见文本规范化：Unicode NFKC、去 inline emphasis/escape/whitespace、统一常见 slash，再计数标签 | Round 7 fresh closure review |
| extended mutant gap | accept | 现有 bold 只包裹完整标签，没有拆分标签字符 | 新增 partial bold/italic、fullwidth slash、escaped slash mutants；保留 exact canonical 断言 | Round 7 fresh closure review |
| R3-B02 / ARCH-NB01 | accept still open | Round 6 仍提供原 blocker 的可复现格式变体 | 修复完成但不自行关闭 | Round 7 fresh closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 6 Conclusion

raw label count 不是 Markdown 可见语义合同。修复已提升为有限、机械、无语义推断的 Markdown 标签规范化，
并继续以原始 exact canonical 行作为唯一权威值。

## Round 7: Normalized label closure

### Review Input

只验证提交 `3d16c016e` 是否关闭 Round 6 blocker。必须重放标准 additive/duplicate/replacement
mutants，以及 partial emphasis、fullwidth slash、escaped slash 和常见 inline Markdown 包装；不要求解析
任意自然语言同义句、Unicode 欺骗字符或完整 CommonMark。

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb536-de9f-7b80-a08a-bf4d5837770a` | spawn result + completion notification | `fork_context=false` | Round 7 Review Input | main-agent history、reasoning、uncommitted drafts、Round 6 reviewer context | yes |

### Reviewer Outputs

#### round7-architecture

##### Verdict

**BLOCKED**

- R3-B02：`still_open`
- ARCH-NB01：`still_open`
- 不允许关闭总审查。

##### Blocking Finding

规范化器仍在自行解析 Markdown。canonical rollback 后追加带括号 title 的 inline link：
`- [回](https://example.invalid "route (rollback)")退：重新打开既有 Phase。` 时，页面可见第二个“回退”，
但正则残留 link metadata，导致 normalized label 计数仍为 1。reference-style link 和带 `>` 引号属性的
HTML emphasis 也有同类绕过。

##### Verification

- 固定提交 `3d16c016e`；
- unified/five-layer contract：PASS；
-既有格式矩阵均正确拒绝；
- 扩展 inline Markdown 攻击有 6 项意外接受；
- 未发现 evidence/DAG 回归。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| home-grown Markdown normalization | accept root cause | 不完整 Markdown parser 必然继续追逐格式变体 | 删除 Markdown 规范化器；把 Phase 工程段作为已有结构化合同验证 | Round 8 fresh closure review |
| structured definition contract | adopt | 每个 Phase 已声明固定 8 字段，失败路由无需从任意 Markdown 推断 | failure-route Phase 只允许 8 条非空字段行，每个已知字段恰好一次且非空；exit/rollback 值仍须 exact canonical | Round 8 fresh closure review |
| extended mutant gap | accept | link/reference/HTML 包装未持久化 | 新增 reference link、带 title inline link、带属性 HTML mutants | Round 8 fresh closure review |
| R3-B02 / ARCH-NB01 | accept still open | Round 7 仍有可复现绕过 | 修复完成但不自行关闭 | Round 8 fresh closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 7 Conclusion

继续扩张正则不是正确方向。当前修复改为验证文档本来就声明的 8 字段结构：额外 directive 无论采用何种
Markdown 表达都会增加未知行或重复字段并 fail closed；route 的唯一值仍由 exact canonical 行约束。

## Round 8: Structured definition closure

### Review Input

只验证提交 `6d36bcb3a` 是否最终关闭 R3-B02/ARCH-NB01。除历次格式 mutants 外，攻击额外自由行、未知
字段、重复已知字段、缺字段替换和空字段；不要求静态门理解任意自然语言同义句或证明其他字段值的业务真实性。

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`gpt-5.6-sol/xhigh/priority`) | `019fb53c-b77c-7bb1-9a14-960d6e31c13e` | spawn result + completion notification | `fork_context=false` | Round 8 Review Input | main-agent history、reasoning、uncommitted drafts、Round 7 reviewer context | yes |

### Reviewer Outputs

#### round8-architecture

##### Verdict

**BLOCKED**

- R3-B02：`still_open`
- ARCH-NB01：`still_open`
- 不允许关闭总审查。

##### Blocking Finding

`R71-12` 的 `- 入口：测量与反馈前置关闭。` 替换为 `- 入口：   ` 后，unified 和 five-layer 均意外
PASS。根因是 populated 检查只比较原始长度，没有判断字段值是否全为空白。

既有格式、链接、HTML、duplicate、replacement、额外自由行、未知字段、重复字段、缺字段替换和零字符
空值均正确拒绝；仅 space/tab-only 值存在缺口。未扩大 evidence/DAG。

### Main Agent Response

| Finding | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|
| whitespace-only field value | accept | `Length > prefix.Length` 把空格和 Tab 当成内容 | 截取字段值后使用 `IsNullOrWhiteSpace`；新增 zero-char、space-only、tab-only mutants | Round 9 fresh closure review |
| R3-B02 / ARCH-NB01 | accept still open | 结构合同尚有一个机械空值缺口 | 修复完成但不自行关闭 | Round 9 fresh closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, pending fresh verification
- Blocking re-review completed: no
- Blocking re-review passed: no
- Allowed to proceed: no

## Round 8 Conclusion

结构合同方向有效，剩余缺陷已收敛为字段空白判定。修复不改变合同边界，只补全 populated 的机械定义。
