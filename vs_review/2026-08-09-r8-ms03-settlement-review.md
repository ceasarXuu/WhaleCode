# Subagent VS Review: R8 MS-03 Action 结算闭环

- Created: 2026-08-09T05:09:58+08:00
- Updated: 2026-08-09T07:05:00+08:00
- Report schema: adversarial-v2
- Task: 对已完成的 MS-03 Action 结算方案执行独立对抗性审查
- Report path: `vs_review/2026-08-09-r8-ms03-settlement-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked
- Control outcome: user-decision-required
- Automatic round budget: 2
- Completed rounds: 1
- Last known-good checkpoint: `e5925b45d`

## Review Control Contract

### Frozen Objective

验证 MS-03 是否在不复制完整 Tool 结果、不新增持久化消息队列、不重建 Map、不重试 Tool 且不改变 Standard 路径的前提下，
可靠地把已观察到的 client Tool 终态写入唯一 canonical Map。

### Acceptance Criteria

- Tool 返回后，outer future 取消不能撤销已投递终态事实。
- Store 只能核对归属后执行 `Pending -> terminal`，同终态幂等，冲突终态和错归属拒绝。
- SQLite writer busy 超过旧 5 秒边界不能静默丢失事实。
- 下一次 TaskSpace Provider 请求前完成恢复对账和 FIFO 屏障；永久错误明确阻断。
- rollout 仍是完整执行历史，Map 仍是唯一当前状态；Standard 路径保持原样。
- 日志和测试能覆盖生产入口，而非只覆盖孤立 helper。

### Explicit Non-goals

- 不要求任意外部 Tool 副作用与 Map 数据库形成进程级原子事务。
- 不新增持久化消费队列、Event Store、第二份完整 Tool 结果或 Map replay/rebuild。
- 不让 Runtime 推断 Agent 语义、自动推进 Node、默认终态或自动重试 Tool。
- 不执行真实 Whale Agent 或 Provider run。

### Frozen Target Locations

- `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_maps/`
- `third_party/codex-cli/codex-rs/core/src/session/taskspace_store/`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/session.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec/`
- `third_party/codex-cli/codex-rs/core/src/tools/output_reference.rs`
- 对应测试及 R8 MS-03 活动文档

### Allowed Change Categories

- 本轮只读审查、审查报告和确定性验证。
- 若发现 E0-E3 支持的 blocker，后续只允许在冻结目标内修复直接根因、补测试和日志。

### Approval-required Changes

- 新顶层模块、外部依赖、公开 API 或持久化 schema 变化。
- 新跨模块抽象或冻结目标外变更。
- 改变产品语义、恢复策略、Standard 行为或真实付费运行。

### Authoritative Sources

| Authority | Source | What It Controls |
|---|---|---|
| E0 | 用户对消息队列、Map/rollout 责任和 MS-03 收敛方案的确认 | 产品边界、非目标和审查授权 |
| E1 | `docs/v0.0.5/build-R8/taskspace-exec/12-phase-b-zero-base-plan.md` | MS-03 工程合同与阶段门禁 |
| E1 | `docs/v0.0.5/build-R8/taskspace-exec/18-phase-b3-execution-feedback-result.md` | 已声明实现、日志和验收证据 |
| E1 | 项目 `AGENTS.md` | Runtime 边界、测试、日志、成本与提交规则 |
| E2 | commits `e5925b45d`、`702f885a0` 及生产调用链 | 实际实现行为 |
| E2 | State 133、TaskSpace Exec 56、settlement 4、output-reference 11 tests | 已执行的确定性证据 |
| E4 | reviewer/main-agent 推断 | 仅作为待验证假设 |

### Baseline And Rollback

- Baseline revision: `05d232bb9`
- Rollback checkpoint: `e5925b45d`
- Expected benefit: 消除固定 writer timeout、post-Tool cancellation 和通用整图 Store 权限导致的事实丢失路径。
- Acceptable side effects: TaskSpace 下一请求在既有结算未完成时等待；永久归属冲突阻断后续 TaskSpace 请求。
- Automatic round budget: 2

## Round 1: MS-03 独立状态一致性审查

### Round Control

- Round type: initial
- Round number: 1
- Completed automatic rounds before launch: 0
- User approval for this round: 用户明确要求“对抗性审查”
- Closure finding IDs: n/a
- Permitted closure relation: n/a
- Target scope delta allowed: none

### Review Input

#### Objective

尝试证伪 MS-03 已经关闭：检查已发生 client Tool outcome 是否仍可能因取消、并发、恢复、输出折叠、身份错配或请求时序而
永久遗留 Pending、写错 Action，或在未结算时进入下一次 TaskSpace Provider 请求。

#### Acceptance Criteria

- 对照 Review Control Contract 的六项验收标准逐项给出生产路径证据。
- 所有 blocker 必须包含可达场景、影响、证据级别和可复现/补证方式。

#### Explicit Non-goals

- 不把进程崩溃前尚未进入 rollout 的未知结果强行判定为终态。
- 不建议增加语义决策、Tool 自动重试、完整结果复制或新的持久化队列。
- 不修改文件，不运行真实 Provider。

#### Review Target

`e5925b45d` 与 `702f885a0` 引入的 outcome-only Store API、Session 结算执行器、恢复对账、请求前屏障、稳定 outer feedback 和
Standard output-reference 复用；同时核对 `05d232bb9` 文档结论是否超过实际证据。

#### Target Locations

- `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_maps/`
- `third_party/codex-cli/codex-rs/core/src/session/taskspace_store/action_settlement.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec/{dispatch.rs,handler.rs}`
- `third_party/codex-cli/codex-rs/core/src/tools/output_reference.rs`
- 对应 tests 与 `docs/v0.0.5/build-R8/taskspace-exec/18-phase-b3-execution-feedback-result.md`

#### Baseline And Rollback Checkpoint

- Baseline: `05d232bb9`
- Rollback checkpoint: `e5925b45d`

#### Change Introduction

完整 Tool 结果继续走 outer feedback/Standard rollout；Tool 返回后同步向 Session FIFO 投递窄化终态，worker 使用 outcome-only
Store transaction 结算。TaskSpace Provider 请求前先扫描 rollout 中仍为 Pending 的稳定反馈，再等待 FIFO barrier。

#### Risk Focus

- worker 生命周期、channel 关闭、Session drop、outer cancellation 和投递时序。
- busy retry 是否真正覆盖所有 SQLite writer busy/locked 入口，是否造成永久悬挂或饥饿。
- barrier 与 recovery 的顺序、一次扫描标志、同一 Session 后续 Pending、并发请求及永久错误传播。
- rollout feedback 是否必然持久化、output-reference 是否可完整恢复、身份字段是否稳定且不可错绑。
- 本地 runtime cache 与 canonical Store latest head 是否可能倒退、覆盖并发变化或观察陈旧状态。
- Standard 是否在任何恢复/屏障/反馈 schema 路径上被改变。

#### User-Perspective Review Focus

- Agent 是否只看到忠实的 Tool 反馈与明确机械错误，而不会收到 Runtime 语义再解释。
- 暂时 writer 竞争是否只增加等待，不产生重复 Tool、错误终态或含糊恢复。

#### Implementation Completeness Focus

- 从 TaskSpace Exec 生产 handler 到 dispatch、enqueue、worker、State transaction、cache install、下一请求 composer 的完整调用链。
- 故障测试是否真实穿过生产入口，还是只直接调用 helper。
- 日志是否能按 map/outer/action/mutation 定位 queued/committed/failed/barrier/recovery/busy。

#### Target Benefit Focus

- 可靠性：已知 timeout/cancellation/generic Store 权限三项缺口是否由确定性测试闭合。
- 延迟：Tool feedback 是否不再同步等待 Map Store；请求前等待是否为明确且可接受的转移。
- 成本：不运行真实模型；只检查 Standard 路径和缓存门禁是否保持。

#### Evidence Sources And Gaps

- E0-E2：用户决策、活动计划/结果文档、生产源码、提交和已运行测试。
- E4 hypothesis：跨请求并发、Session drop 与 recovery marker 可能仍有未覆盖交错。
- Known evidence gap：未执行真实 Provider；本轮不需要也不允许用付费运行替代确定性机制审查。

#### Assumptions To Attack

- “同一次 poll 内 enqueue”足以隔离 outer cancellation。
- FIFO barrier 覆盖了所有此前已观察结果。
- recovery 扫描一次不会漏掉同 Session 后续需要恢复的结果。
- Store busy retry 不会把永久非 busy 错误误判为可恢复。
- 稳定 outer feedback 在 inline、summarized、referenced 三种输出下都可对账。

#### Adversarial Lenses

- state、concurrency、failure、data、implementation-completeness、testing、observability、maintenance、target-benefit

#### Verification Status

- 已通过：State 133、TaskSpace Exec 56、settlement 4、output-reference 11、workspace check、zero-base/cache gate。
- 未运行：真实 Whale Agent/Provider；不属于本轮机制验收。

#### Reviewer Instructions

- Fresh internal subagent session; no inherited main-agent context.
- Read target files directly; inspect commits when useful.
- Do not modify files and do not run real Provider/Whale Agent.
- Prioritize concrete correctness/reliability blockers over style.
- Cite paths and line numbers where possible.
- Classify blocking and scope-expanding claims as E0-E4.
- Return: summary, blocking findings, non-blocking risks, implementation completeness, benefit checks, required fixes, missing tests, missing logs.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | one 10-minute extension | 2 | review unavailable cannot pass |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| State consistency and failure-path reviewer | MS-03 横跨异步生命周期、SQLite transaction、恢复与上下文边界 | cancellation、concurrency、recovery、idempotency、data integrity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| State consistency and failure-path reviewer | `multi_agent_v1.spawn_agent` | `019fe337-6475-7042-896d-4c338c40d420` (`Helmholtz`) | internal spawn tool call and completion notification | `fork_context=false` | Round 1 Review Input 的冻结目标、验收、非目标、路径、风险和输出合同 | main-agent history、reasoning、drafts、conclusions、完整 diff dump | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-state-consistency | State consistency and failure-path reviewer | 1 | `019fe337-6475-7042-896d-4c338c40d420` | 约 10 分钟 | completed | 在 20 分钟窗口内完成跨模块只读审查 | completed |

### Reviewer Outputs

#### R1-state-consistency

##### Summary

MS-03 尚不能标记 verified。审查发现三个 blocker：生产 Tool 完成与结算投递之间仍有 outer cancellation 窗口；graceful
Session shutdown 不 drain FIFO；现有测试没有覆盖完整的持久化生产链。reviewer 离线运行 State settlement 3、Session
settlement 4、TaskSpace handler 8、output-reference 10 条定向测试，全部通过；没有运行真实 Whale Agent/Provider。

##### Blocking Findings

- `R8-MS03-B01`：Tool completion 在子任务中发生，父 future 收到结果后才 enqueue。
  - Broken assumption: “Tool 返回与事实投递发生在同一次不可分割 lifecycle boundary”不成立。
  - Failure scenario: 原生 Tool 子任务已完成副作用和结果，但 outer TaskSpace future 在 poll `JoinHandle` 之前被 abort；
    `AbortOnDropHandle` 丢弃已完成结果，dispatch 未执行 enqueue，rollout 也没有 outer feedback。
  - Trigger condition: 子任务完成与父 future 下一次 poll 之间发生 turn abort/drop。
  - Impact: 已执行 Action 永久保持 Pending，现有 recovery 无结果可对账。
  - Proof needed: 用 latch 固定“子 Tool 完成、父 future 尚未接收”的交错并复现 Pending。
  - Evidence authority: E2 生产代码与 `AbortOnDropHandle` drop 语义；精确确定性复现尚缺。
  - Evidence source: `core/src/tools/parallel.rs:167-208`、`core/src/tools/taskspace_exec/dispatch.rs:110-132`。
  - Closure relation: original-blocker-open。
  - Scope effect: 需要重新确定 TaskSpace-owned Tool execution producer 的生命周期归属。

- `R8-MS03-B02`：graceful shutdown 没有等待结算 producer 和 FIFO drain。
  - Broken assumption: Session 持有 queue 即可保证 shutdown 前已观察事实落盘。
  - Failure scenario: shutdown abort active tasks 后继续关闭 persistence 并发送 `ShutdownComplete`；外部 Session owner 随后
    drop，worker 的 `Weak<Session>` 无法 upgrade，队列中剩余事实被放弃。
  - Trigger condition: shutdown 时存在已排队或正等待 SQLite writer 的结算，或 B01 修复后的 producer 尚未 enqueue。
  - Impact: graceful shutdown 可无错误地遗留 Pending。
  - Proof needed: queued、multi-queued 和 writer-busy shutdown tests。
  - Evidence authority: E2 lifecycle 代码；确定性 shutdown 反例尚缺。
  - Evidence source: `core/src/session/handlers.rs:1072-1120`、`core/src/session/taskspace_store/action_settlement.rs:379-412`。
  - Closure relation: direct-adjacent-objective-failure。
  - Scope effect: shutdown 必须先收敛 producer，再 drain FIFO，并在完成信号前暴露永久结算错误。

- `R8-MS03-B03`：现有绿色测试没有覆盖组合生产链。
  - Broken assumption: helper 和 test-only in-memory harness 足以证明 persisted production path。
  - Failure scenario: dispatch、queue、SQLite、rollout/output-ref、recovery/barrier 或 provider preparation 接缝出错时，孤立测试仍全绿。
  - Trigger condition: 任一跨模块集成缺陷；B01、B02 已说明该缺口并非理论问题。
  - Impact: 活动文档的 verified 结论超过证据。
  - Proof needed: persisted handler 到下一 provider preparation/transport blocking 的端到端确定性测试。
  - Evidence authority: E1 验收合同 + E2 tests/call graph。
  - Evidence source: `core/src/tools/taskspace_exec_handler_tests.rs:161-183`、
    `core/src/session/taskspace_action_settlement_tests.rs:65-108`、`core/src/session/mod.rs:3193-3217`。
  - Closure relation: original-blocker-open。
  - Scope effect: 测试脚手架需复用生产入口，但不得调用真实 Provider。

##### Non-blocking Risks

- `R8-MS03-N01`：busy classifier 只匹配字符串 `5/6`，而当前 SQLx `DatabaseError::code()` 返回 SQLite extended code。
  - Failure scenario: `SQLITE_BUSY_*` / `SQLITE_LOCKED_*` 被当成永久错误，后续 TaskSpace 请求持续阻断。
  - Evidence authority: E2；`state/src/runtime/taskspace_action_settlements.rs:198-207` 与本机
    `sqlx-sqlite-0.8.6/src/error.rs` 明确返回 extended result code。
  - Closure relation: direct-adjacent-objective-failure。

- `R8-MS03-N02`：恢复未交叉验证 `outer_call_id` 与 action identity，并会跳过更早的同 Action 冲突反馈。
  - Failure scenario: rollout 出现身份不一致或重复冲突反馈时，恢复可能接受最后一条而不报告历史冲突。
  - Evidence authority: E2 code；构造恶意/损坏 history 的可达性仍是 E4。
  - Closure relation: direct-adjacent-objective-failure。

- `R8-MS03-N03`：reviewer 声称 indefinite busy 与 unbounded channel 可持续无限增长。
  - Main evidence qualification: 当前下一请求 barrier 与单 active turn 限制了跨 turn 持续生产；同一 response 的 calls 数仍受
    provider 输出大小约束，因此“无限增长”没有 E2 生产者证据。保留 queue depth/age 可观测性建议，不把该表述作为正确性风险。
  - Evidence authority: E4；closure relation: unrelated-existing-risk。

- `R8-MS03-N04`：Store read 可在并发 settlement 后安装更旧的本地 cache record。
  - Failure scenario: read 先加载旧 revision，settlement 提交并安装新 revision，随后 read 再安装旧 revision。
  - Impact: SQLite canonical Store 正确，但 Session cache 可暂时倒退并影响 snapshot/recovery 的即时观察。
  - Evidence authority: E2 call sequence；`core/src/session/taskspace_store_read.rs:37-71`、
    `core/src/session/taskspace_store.rs:444-457`。
  - Closure relation: direct-adjacent-objective-failure。

##### User-Perspective Checks

- Usability: blocked；graceful shutdown 可报告完成但留下 Pending，错误反馈与真实状态不一致（B02）。
- Ease of use: risk；永久 Store 错误会持续阻断，但当前日志不足以快速区分归属冲突与 extended busy（N01）。
- Ease of understanding: pass by inspection；outer feedback 没有增加 Runtime 语义再解释，本轮问题集中在事实生命周期。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| post-Tool cancellation | Tool 完成后事实不可撤回 | `parallel.rs` -> `dispatch.rs` | Exec handler | manual enqueue 后 abort | queued/committed | cancellation 边界被 mock 掉 | partial | B01 |
| graceful shutdown | producer 收敛后 drain FIFO | `handlers.rs::shutdown` | Session shutdown | missing | worker 无 stop/drain 事件 | none | not-started | B02 |
| outcome-only Store | 只推进匹配 Action 终态 | State settlement API | Session worker | State targeted tests | committed/failed | none | landed | none |
| writer busy | 超过 5 秒继续等待 | State retry loop | outcome-only API | 5.2 秒 writer test | busy event | extended code 未覆盖 | partial | N01 |
| rollout recovery/barrier | Pending-only 对账并在 provider 前阻断 | `prepare_provider_visible_prompt_items` | provider composer | helper-level recovery/barrier | recovery/barrier | persisted composed path missing | partial | B03 |
| Standard isolation | 不进入 TaskSpace recovery/barrier | early return in session composer | Standard provider path | provider composer tests | cache gate | limited end-to-end proof | landed | none |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| reliability | 旧 timeout/cancellation 可丢事实 | 已观察 outcome 不丢失 | deterministic fault tests | helper tests only | regressed/open | B01/B02 | regressed | B01/B02/B03 |
| latency | 旧 handler 同步等待 Store | feedback 不同步等待；下一请求必要等待 | trace latency | missing | unmeasured | wait 转移到 barrier | unmeasured | none |
| cost | 不复制结果、不增加 Provider run | 保持单 rollout/Map | static data-flow + cache gate | no second result copy, gate PASS | structurally neutral | queue metadata only | weak-evidence | none |

##### Required Fixes

- `B01`：先以 latch test 复现，再把 Tool completion 与 settlement publication 放进不会随 outer collector drop 的
  TaskSpace-owned producer；不得改变普通 Tool 自身合同。
- `B02`：定义并测试 shutdown 顺序：停止/等待 producer 后 drain FIFO，永久错误必须在 `ShutdownComplete` 前暴露。
- `B03`：增加持久化组合测试，覆盖 handler dispatch、SQLite settlement、rollout/output-ref recovery、provider preparation
  和 transport 未被调用的阻断断言。
- `N01`：按 SQLite primary result code 识别 extended BUSY/LOCKED，并补正反测试。
- `N04`：使本地 cache install revision 单调，或把 refresh install 纳入已有 Store serializer。
- `N02`：恢复时核对 outer/action identity，并拒绝同 Action 冲突终态。

##### Missing Tests

- B01 精确交错；B02 queued/multi-queued/writer-busy shutdown；B03 persisted composed path。
- 永久 settlement failure 阻止 fake transport；并发 recovery/provider preparation；cache monotonicity。
- wrong Tool/missing Action/Pending/conflicting terminal/duplicate nodes/wrong Map 的 State 负例。
- 大 outer feedback 真正写成 `OutputReferenceV1` 后 resume/SHA/recovery。
- Standard 模式在 poisoned TaskSpace worker 下仍直接绕过。

##### Missing Logs / Observability

- `taskspace.action_settlement_queued` 应在 channel send 成功后记录，而不是发送前。
- worker failure 应携带 map/outer/action/node/tool/outcome/mutation identity。
- shutdown drain 应记录 start/result；busy 应记录 raw/primary code、累计等待；barrier/recovery 应记录耗时。
- queue depth/age 属于延迟与容量观测，不是本轮正确性 blocker。

## Main-Agent Triage

| Finding | Decision | Authority | Main-Agent Response | Scope / Side Effect | Approval |
|---|---|---|---|---|---|
| B01 | accept | E2 | `AbortOnDropHandle` 的完成结果确实在父 future poll 后才进入 enqueue；当前测试只证明 enqueue 后取消安全，未覆盖 enqueue 前窗口 | 需要调整 TaskSpace execution producer 生命周期，可能触及并行/取消语义 | 重大生命周期设计，先交还用户 |
| B02 | accept | E2 | shutdown 没有 producer join 或 FIFO barrier，worker 只持有 Weak；当前 `ShutdownComplete` 不能证明队列已 drain | 需要定义 producer -> queue -> persistence 的 shutdown 顺序 | 与 B01 一体设计，先交还用户 |
| B03 | accept | E1+E2 | 持久化测试直接注入 fact/feedback，handler harness 使用 test-only in-memory Map；verified 证据不足 | 新增确定性生产链集成测试，不使用真实 Provider | 可随 B01/B02 修复实施 |
| N01 | fixed | E2 | `4be93ba31` 按 extended code 的低字节识别 primary `BUSY/LOCKED`，日志同时记录 raw/primary code，并覆盖 `261/262/517` | 局部 classifier 与测试；不改变重试策略 | 已闭合 |
| N02 | fixed | E2+E4 | `4be93ba31` 在任何 enqueue 前完成 outer/call-index/action identity 与同 Action 冲突历史的整批校验 | 恢复硬校验与负例，不引入语义判断 | 已闭合 |
| N03 | reject | E4 | barrier、单 active turn 和单 response 有限 calls 不支持“随 busy 持续无限生产”；仅接受 queue depth/age 观测建议 | 不引入 backpressure/持久化队列 | n/a |
| N04 | fixed | E2 | `2aa968348` 将 Store read、settlement 和 conflict refresh 收敛到同一原子安装门禁；旧 revision 跳过、同 revision 异 hash 与 Map 绑定变化拒绝 | 不给读取增加全局写锁，不改变 canonical Store | 已闭合 |

`4be93ba31` 还把 `taskspace.action_settlement_queued` 移到 channel send 成功之后，并补齐 worker failure 的
map/outer/action/node/tool/outcome identity。State 定向 4、Session settlement 6、TaskSpace Exec 56、workspace check、
zero-base 与 cache gate 均通过；没有真实 Whale Agent/Provider run。

B01 与 B02 仍共同改变 execution producer 和 shutdown 所有权，属于项目规则要求用户复核的重大技术路线点；B03 必须随
该设计补组合生产链测试。`2aa968348` 已用 5 条 cache 安装测试和实际 Session 反例闭合 N04；Round 2 尚未启动。

## Review Governor

- Decision: user-decision-required
- Rationale: Round 1 有 3 个 accepted blocker，未解决数为 3。B01/B02 需要共同定义 TaskSpace-owned producer 与 graceful
  shutdown drain 的生命周期，不能靠另一个后置补丁或孤立 barrier 解决；这会改变跨模块执行/取消边界，必须先由用户确认
  修复方向。自动预算尚余 1 轮，只能在修复完成后用于聚焦 B01～B03 的 closure review。

## Convergence And Closure

- Unresolved blockers: 3（B01、B02、B03）；accepted non-blocking 项 N01、N02、N04 已闭合
- Scope growth: 审查报告 1 个；生产代码无变化；未增加依赖、API、schema 或真实运行。
- Side effects: MS-03 与 Phase B3 的 verified 结论必须重新打开。
- Evidence inventory: E0 产品边界；E1 MS-03 合同；E2 production code/tests/SQLx dependency；E4 精确交错待确定性复现。
- Risk direction: 原修复关闭了 Store timeout/generic API，但 cancellation 风险只从“enqueue 后”缩小到“子任务完成到 enqueue 前”，
  尚未收敛；graceful shutdown 增加了相邻 lifecycle 缺口。
- Last known-good checkpoint: `e5925b45d`；回滚 `702f885a0` 会恢复已知同步 settlement 缺口，不建议在无替代方案时回滚。
- Closure status: blocked pending user decision
- Bounded next decision: 确认把 Tool execution + settlement publication 作为 TaskSpace-owned producer，并在 shutdown 先 join
  producers、再 drain FIFO；确认后实施 B01/B02/B03 与 accepted 相邻硬缺口，再使用剩余 1 轮做 focused closure review。
