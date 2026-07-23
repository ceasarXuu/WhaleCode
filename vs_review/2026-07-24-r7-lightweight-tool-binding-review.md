# Subagent VS Review: R7 轻量 Tool 绑定

- Created: 2026-07-24 02:17:16 +0800
- Updated: 2026-07-24 05:00:00 +0800
- Report schema: adversarial-v1
- Task: 审查 R7 普通 Tool 轻量绑定修复是否在降低 schema 成本的同时，完整保留连续动作硬合同和 TaskSpace 边界。
- Report path: `vs_review/2026-07-24-r7-lightweight-tool-binding-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: 实现正确性与边界审查

### Review Input

#### Objective

验证提交 `a105dfdee` 是否以简洁、一致的生产实现达成以下目标：普通 Tool 不再复制完整
TaskSpace 生命周期联合，但仍保留不可绕过的连续动作机械合同；Runtime 只执行状态机硬规则，不推断
Agent 语义；Tool 结果忠实进入上下文；Standard 路径不受 TaskSpace 合同污染。

#### Review Target

代码实现、状态和 Tool sequence、测试策略、观测与成本证据。

#### Target Locations

- `third_party/codex-cli/codex-rs/tools/src/taskspace_binding.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/router.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_binding.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_manifest.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control*.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/*tests.rs`
- `docs/v0.0.5/build-R7/37-r7-lightweight-tool-binding-repair-plan.md`
- `benchmarks/taskspace/r7/five-layer-taskspace-control-v3.schema.json`
- `benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json`
- `scripts/taskspace-benchmark/lib/*`
- `git show a105dfdee`

#### Change Introduction

提交把普通 Tool 上的完整 `taskspace_action` 生命周期联合替换为必填两值
`taskspace_binding=active|after_boundary`。`initialize_map`、`bind_node` 和
`complete_then_continue` 的完整参数回到中央 `taskspace_control`。Response 级 preflight
检查边界 control 与紧邻普通 Tool 的双向配对；Router 在普通 Tool handler 前移除 binding；
control 与普通 Tool 结果作为独立有序事实返回。

#### Risk Focus

- `taskspace_binding` 是否覆盖所有 Agent 可调用的普通 Tool 形态，是否存在 Custom、MCP、
  ToolSearch、code mode、namespace 或动态 Tool 绕过。
- preflight 是否总在任何调用执行前完成，是否存在 parse、并行、barrier、取消、失败或终态路径造成
  部分提交、乱序执行或错误跳过。
- `active` 与 `after_boundary` 是否只做机械关系校验，是否错误弱化原有 binding、lease、
  reservation、DAG 或 revision 硬规则。
- lifecycle schema 是否真的只有一个生产 owner，旧 carrier/parser/兼容分支是否仍可达。
- control 成功而普通 Tool 失败、control 失败、Tool handler parse 失败、同 response 多个 boundary
  pair 时，模型可见反馈是否完整、忠实且无重复。
- Standard 是否保持原 schema 和原执行语义，TaskSpace 字段是否可能泄漏或被静默接受。
- 测试和观测是否只验证内部字符串/fixture，而没有覆盖真实生产入口和失败路径。

#### User-Perspective Review Focus

- Agent 能否从 L2、中央 Tool schema 和普通 Tool 字段理解合法组合。
- 被拒绝时是否能从反馈中准确知道实际序列和机械期望，而不会收到 Runtime 的语义建议。
- 合法普通动作是否增加不必要的重复负担，或因字段含义歧义导致稳定空转。

#### Implementation Completeness Focus

- Tool registry 到 provider visibility、response parse、preflight、sequence executor、router、
  handler、ActionMap gate、结果重建和观测的生产路径是否全部落地。
- Function、freeform patch、code mode、ToolSearch、MCP 和动态 namespace 是否有真实入口证据。
- 单元测试、集成测试、Docker 自然样本与 wire schema 成本是否能共同证明实现，不把
  schema-only、test-only 或 report-only 工作计为完成。
- 日志能否关联 provider schema、声明序列、零执行拒绝、control 提交和普通 Tool 结果。

#### Target Benefit Focus

- 声称的成本收益：相对 Standard 的 TaskSpace Tool schema 额外字节从 `39,074` 降至
  `7,531`，目标为至少降低 80%，方法为同一 Docker harness 的最终 wire `tools` section。
- 声称的正确性收益：连续动作仍可在同一 response 执行，非法边界序列零执行。
- 检查比较是否同二进制、同 Tool 集合、同 provider 条件，及是否存在 request、token、cache、
  Agent 采用率或业务成功率回归。

#### Assumptions To Attack

- provider 返回的所有普通 Tool 都经过 schema decorator 和 binding extractor。
- `taskspace_control` 参数可在不执行 handler 的情况下被 preflight 稳定识别。
- response Tool calls 在 sequence preflight 前没有任何副作用。
- control barrier 能阻止后续动作越过失败，但不会阻止合法连续动作。
- ActionMap 原有 gate 足以校验当前 Work binding，轻量字段无需重复 node/revision。
- 单个 JSON Schema 无法保证 sibling 存在，但 Runtime 拒绝与双边字段足以守住正确性底线。

#### Adversarial Lenses

- state
- input
- concurrency
- failure
- implementation-completeness
- target-benefit
- maintenance
- testing
- observability
- comprehension

#### Verification Status

- `cargo test -p codex-tools taskspace --lib`: 9 passed。
- `cargo test -p codex-core taskspace --lib --no-fail-fast`: 93 passed。
- `cargo test -p codex-core --test all taskspace_terminal_contract --no-fail-fast`: 2 passed。
- R7 five-layer contract `-Phase All`: passed。
- trace/cost/performance observer self-tests: passed。
- `cargo check -p codex-tools -p codex-core`: passed。
- `cargo build -p codex-cli --bin whale --locked`: passed。
- repeat-1 Docker map-request 自然样本业务与 hidden oracle 通过，但出现一次单独初始化 preflight
  拒绝和一次过早 finish 拒绝。
- 尚未对本提交执行独立审查。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Try to falsify the implementation, tests, logs, and benefit claims rather than confirm them.
- Cite evidence paths and line numbers when possible.
- Return summary, blocking findings, non-blocking risks, user-perspective checks,
  implementation-completeness table, target-benefit table, required fixes, missing tests,
  missing observability, and evidence.
- For each blocking or major finding, state broken assumption, trigger, impact, and proof needed.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one bounded 10-minute extension | 2 | accepted blocking finding requires a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | 变更跨越 Tool schema、provider parse、状态预检、顺序执行和反馈，最高风险是状态与失败路径正确性 | partial execution、bypass、ordering、state gate、feedback |

### Reviewer Launch Records

首次调用 `spawn_agent` 在创建 session 前返回 `agent thread limit reached`。关闭一个状态为
`completed` 的历史会话后重新启动；失败调用没有产生 Agent ID，也没有 reviewer output。

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f9033-93b3-7230-8417-17edf8279de7` (`Pauli`) | spawn tool response in main session | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, persuasion brief | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-implementation | implementation-adversary | 1 | `019f9033-93b3-7230-8417-17edf8279de7` | about 16 minutes | completed | reviewer returned within the 20-minute initial window | completed |

### Reviewer Outputs

#### R1-implementation

##### Summary

Reviewer verdict: **BLOCK**. The common Function path has a lightweight binding and retains the
ActionMap gate, but the response-level contract is not yet unbypassable. The reviewer independently
reran the three declared Rust suites; 9, 93, and 2 tests passed, but those suites did not exercise the
counterexamples below.

##### Blocking Findings

- **BF-1 Critical: mailbox preemption can execute a provider-response prefix before
  `response.completed`.**
  - Broken assumption: preflight always sees the complete provider response.
  - Failure scenario: a Tool call is accumulated, a Reasoning or commentary item arrives, mailbox
    input is pending, and the stream still has a mechanically invalid suffix.
  - Trigger condition: `turn.rs:2885` exits sampling successfully on mailbox preemption; the
    accumulated prefix is executed at `turn.rs:3255`.
  - Impact: prefix side effects or Map commits occur without validating the omitted suffix.
  - Proof needed: streaming integration with a Tool before reasoning/mail and an invalid suffix;
    assert zero dispatch until a definitive completed event.

- **BF-2 High: malformed or mechanically forbidden control calls evade response preflight.**
  - Broken assumption: preflight can classify every control call's mechanical validity.
  - Failure scenario: a valid ordinary call is followed by malformed control JSON, a missing or
    non-string action, or a control carrying forbidden `taskspace_binding`.
  - Trigger condition: `sequence_manifest.rs:54` reduces parse failure to `None`, while
    `sequence_preflight.rs:106` does not reject a binding on control.
  - Impact: earlier segments execute before the control handler rejects the call.
  - Proof needed: side-effect-spy executor tests proving zero dispatch for these response shapes.

- **BF-3 High: ToolSearch failures are reported as success and do not stop later controls.**
  - Broken assumption: an ordinary Tool failure always creates a faithful unsuccessful output.
  - Failure scenario: invalid ToolSearch followed by state-changing control.
  - Trigger condition: `parallel.rs:276` emits `status=completed` with no error, and
    `sequence.rs:285` interprets that status as success.
  - Impact: the Agent loses the error fact and the control executes across a failed action.
  - Proof needed: executor test with failing ToolSearch followed by control; assert visible error and
    skipped control.

- **BF-4 High: tools discovered through ToolSearch are returned without TaskSpace binding.**
  - Broken assumption: every provider-visible ordinary Tool passes through the decorator.
  - Failure scenario: TaskSpace ToolSearch reveals a deferred MCP or dynamic Tool, then the Agent
    invokes the schema it was shown.
  - Trigger condition: `tool_search_entry.rs:55` and `:70` build raw `LoadableToolSpec`;
    `context.rs:338` serializes it unchanged while runtime requires a binding.
  - Impact: advertised schema and runtime contract conflict, causing deterministic rejection loops.
  - Proof needed: ToolSearch-output schema assertion and successful subsequent TaskSpace invocation;
    Standard output must remain unchanged.

- **BF-5 High: current ToolSpec variants bypass or cannot express the contract.**
  - Broken assumption: all Agent-callable Tool shapes are decoratable and preflighted.
  - Failure scenario: TaskSpace enables LocalShell, native WebSearch, ImageGeneration, or an unknown
    Freeform Tool.
  - Trigger condition: the decorator's fallback at `tools/src/taskspace_binding.rs:40` leaves these
    variants unchanged; LocalShell is parsed without binding and native web/image events do not
    enter the Tool sequence.
  - Impact: an active action can bypass binding, and these Tools cannot serve an
    `after_boundary` action.
  - Proof needed: exhaustive ToolSpec visibility/runtime test plus a product decision to project,
    replace, or hide unsupported native shapes in TaskSpace.

- **BF-6 High: Standard semantics change on a legitimate reserved-field collision.**
  - Broken assumption: external MCP/dynamic schemas never contain a business field named
    `taskspace_binding`.
  - Failure scenario: a Standard external Tool legitimately owns that top-level argument.
  - Trigger condition: Router strips the field mode-independently at `router.rs:311`, Standard
    preflight rejects it, and TaskSpace decoration panics at
    `tools/src/taskspace_binding.rs:49`.
  - Impact: Standard input is altered/rejected; TaskSpace prompt construction can panic.
  - Proof needed: Standard exact-forwarding test and TaskSpace deterministic collision handling
    without panic.

##### Non-blocking Risks

- **NR-1:** preflight repeats the complete actual/expected sequence in every failed call output,
  multiplying context cost; the canonical revision argument is unused.
- **NR-2:** unknown future Freeform Tools remain Custom and undecorated; this is also part of BF-5's
  extensibility problem.
- **NR-3:** `ToolSequencePreflightResultV1` says zero calls executed but omits explicit
  `state_commit=false`.
- **NR-4:** the one natural sample recovered from standalone initialization and premature finish;
  stable Agent adoption is not established.

##### User-Perspective Checks

| Check | Result |
|---|---|
| L2 and central control explain legal boundary pairs | pass |
| Every visible ordinary Tool explains the field | fail for deferred and native Tool shapes |
| Rejection exposes actual and mechanically expected sequence | partial; Function path passes, ToolSearch loses errors, multi-call rejection duplicates payload |
| Runtime avoids semantic node selection | pass in reviewed ActionMap paths |
| Standard users retain original behavior | fail on reserved-field collision |

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Initial Function visibility | binding visible and required | decorator + provider visibility | prompt build | happy-path unit | schema profile | none | landed | none |
| Namespace visibility | members decorated | decorator | prompt build | no production-entry test | schema count only | none | partial | MT-5 |
| Patch/code mode | Function projection preserves raw action | freeform projection + router | provider response | patch schema and exec happy path | natural exec sample | none | partial | MT-7 |
| ToolSearch outer call | truthful success/failure | parallel result + sequence | direct response | success only | incomplete | none | partial | BF-3 |
| Deferred search results | returned Tool has binding | search entry/output | ToolSearch result | opposite fixture exists | absent | none | not-started | BF-4 |
| Native/other ToolSpec shapes | participate or are explicitly unavailable | visibility + parser | provider-native events | absent | absent | none | not-started | BF-5 |
| Complete-response preflight | zero dispatch before completed response | turn sampling + sequence | response stream | absent | incomplete | none | partial | BF-1 |
| Control mechanical preflight | invalid response zero dispatch | manifest + preflight | sequence | pure validator only | partial | none | partial | BF-2 |
| Ordered failure propagation | failed action skips successors | parallel result + sequence | executor | Function path only | partial | none | partial | BF-3 |
| ActionMap gate | binding/lease/reservation remain enforced | ActionMap runtime | ordinary dispatch | existing tests | runtime events | none | landed | none |
| Central lifecycle owner | one production schema/handler | control v3 | provider + handler | authority and parser tests | manifest identity | none | landed | none |
| Standard isolation | no field removal or rejection | provider visibility + router | Standard response | absent collision test | absent | none | partial | BF-6 |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| extra schema bytes reduced | 39,074 extra bytes | at least 80% reduction | Docker wire Tool section | plan records 7,531 final extra bytes | 80.7% arithmetic achieved | raw identity evidence not committed | weak-evidence | TB-1 |
| legal continuous Function action | full carrier baseline | same-response control + action | tests and natural trace | exec happy path | achieved for covered shape | incomplete Tool-shape coverage | weak-evidence | BF-4/BF-5 |
| illegal boundary response zero dispatch | intended hard rule | zero calls | response preflight | pure validator tests | regressed | prefix and malformed-control bypass | regressed | BF-1/BF-2 |
| request/token/cache improvement | no paired final baseline | unspecified | natural benchmark | repeat-1 TaskSpace only | unmeasured | recovery requests present | unmeasured | TB-2 |
| stable adoption/business success | single successful run | unspecified | repeated natural samples | repeat-1 | unmeasured | two protocol recoveries | unmeasured | TB-3 |

##### Required Fixes

- **RF-1:** execute accumulated calls only after a definitive complete-response event.
- **RF-2:** represent control manifest parsing as valid/invalid and reject mechanically invalid
  controls, including forbidden binding, during response preflight.
- **RF-3:** preserve ToolSearch errors in a model-visible failed output and in sequence success.
- **RF-4:** decorate ToolSearch-returned `LoadableToolSpec` according to active visibility.
- **RF-5:** project, replace, or explicitly hide every Tool shape that cannot participate in
  TaskSpace binding/preflight.
- **RF-6:** make extraction mode-aware, preserve Standard arguments exactly, and replace collision
  panics with deterministic errors.

##### Missing Tests

- **MT-1:** Tool-before-reasoning mailbox preemption with a later invalid suffix.
- **MT-2:** malformed/unknown control and binding-on-control after a side-effecting call.
- **MT-3:** failing ToolSearch followed by state-changing control.
- **MT-4:** deferred MCP/dynamic ToolSearch schema and actual invocation.
- **MT-5:** exhaustive current ToolSpec visibility and runtime coverage.
- **MT-6:** Standard reserved-field forwarding and TaskSpace collision behavior.
- **MT-7:** executor-level control failure, ordinary failure after control, multiple boundary pairs,
  cancellation, and result ordering.
- **MT-8:** observer fixtures with ToolSearch, LocalShell, native calls, and interleaved reasoning.

##### Missing Logs / Observability

- **MO-1:** preflight rejection lacks explicit turn/response identity, call IDs, revision,
  sequence hash, zero dispatch, and state-commit fields.
- **MO-2:** schema profile lacks provider/request/binary/tool-set identity.
- **MO-3:** native cadence ignores ToolSearch calls and splits a response on any non-call item.
- **MO-4:** cost instrumentation counts only Function/Custom calls and outputs in the relevant path.

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2844`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2885`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:3255`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_manifest.rs:54`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs:106`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs:276`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs:285`
- `third_party/codex-cli/codex-rs/core/src/tools/tool_search_entry.rs:55`
- `third_party/codex-cli/codex-rs/core/src/tools/context.rs:338`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_binding.rs:12`
- `third_party/codex-cli/codex-rs/core/src/tools/router.rs:311`
- `scripts/taskspace-benchmark/lib/native-cadence.ps1:70`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:1408`

### Main Agent Response

All six blocking findings were independently checked against the production paths. BF-2 is accepted
only for mechanically decidable control validity; preflight must not absorb semantic Tool or Agent
decisions.

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | BF-1 / RF-1 | uncompleted response prefix can execute on mailbox preemption | blocking-critical | accept | `turn.rs:2885` returns `Ok`; `turn.rs:3255` executes pending calls outside Completed | recorded blocker; no code change during review | fix response completion ownership, add MT-1, fresh closure review |
| implementation-adversary | BF-2 / RF-2 | mechanically invalid control is detected after earlier segments | blocking-high | accept | manifest parse failure becomes `None`; binding-on-control is dispatch-time only | scope limited to mechanical parse/forbidden fields | add zero-dispatch preflight and MT-2 |
| implementation-adversary | BF-3 / RF-3 | ToolSearch error becomes successful empty result | blocking-high | accept | `parallel.rs:279-284` drops error text; sequence treats completed as success | recorded feedback-layer blocker | introduce truthful failure encoding and MT-3 |
| implementation-adversary | BF-4 / RF-4 | search result schema lacks required runtime binding | blocking-high | accept | raw `LoadableToolSpec` is serialized; runtime requires binding | recorded capability/feedback contradiction | mode-aware result projection and MT-4 |
| implementation-adversary | BF-5 / RF-5 | native/current ToolSpec variants bypass contract | blocking-high | accept | decorator fallback, LocalShell parser, and native event paths confirm incomplete coverage | recorded as product/technical decision boundary | decide projection vs TaskSpace exclusion, then MT-5 |
| implementation-adversary | BF-6 / RF-6 | Standard business argument is stripped; TaskSpace can panic | blocking-high | accept | Router extraction is mode-independent and decorator uses `assert!` | recorded Standard-isolation blocker | mode-aware extraction, typed collision failure, MT-6 |
| implementation-adversary | NR-1 | repeated preflight payload amplifies context | non-blocking | accept | every call receives the same full sequence JSON | recorded | redesign compact per-call correlation after blockers |
| implementation-adversary | NR-2 | future Freeform Tool bypass | non-blocking, subsumed | accept | decorator returns unknown Freeform unchanged | included in BF-5 | include in exhaustive ToolSpec policy |
| implementation-adversary | NR-3 | zero-execution result omits `state_commit=false` | non-blocking | accept | payload has execution count but no state field | recorded | add explicit mechanical state fact |
| implementation-adversary | NR-4 | stable adoption unsupported by repeat-1 | non-blocking | accept | run had two protocol recoveries | recorded without treating Agent recovery as correctness failure | repeat paired samples after blockers |
| implementation-adversary | TB-1 | same-binary/provider evidence not committed | non-blocking | accept | arithmetic is correct; raw run exists only under `/tmp` | report keeps benefit as weak evidence | persist bounded identity manifest in next benchmark |
| implementation-adversary | TB-2 | request/token/cache benefit unmeasured | non-blocking | accept | no final paired repeat | the repair plan already avoids policy selection from repeat-1 | run paired repeats after correctness closure |
| implementation-adversary | TB-3 | stable adoption unmeasured | non-blocking | accept | one natural run | no stability claim accepted | run multi-repeat after correctness closure |

All eight missing-test items MT-1 through MT-8 are **accepted**. MT-1 through MT-6 are required for
the corresponding blocking fixes. MT-7 is required to replace pure-validator confidence with
executor evidence. MT-8 is required before observers can claim full Tool-shape coverage.

All four observability items MO-1 through MO-4 are **accepted as non-blocking review findings**.
MO-1 and MO-3 are directly relevant to diagnosing BF-1/BF-2; MO-4 is required for BF-3/BF-5
coverage. Instrumentation must record facts and identities only, without semantic inference.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - required after fixes; not started in this review-only round
- Blocking re-review launch records:
  - not applicable until a closure implementation exists
- Rejected findings backed by evidence: n/a; no finding rejected
- Deferred findings documented: yes; no finding is silently deferred, BF-5 awaits an explicit product/technical choice
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: six accepted production-path correctness/isolation findings remain unfixed
- Allowed to proceed: no

## Final Conclusion

The adversarial review completed successfully, but commit `a105dfdee` did not pass. The lightweight
schema-cost direction remains valid; the implementation cannot be treated as closed until BF-1
through BF-6 are fixed and a fresh closure reviewer passes the result. BF-5 requires an explicit
decision for provider-native Tool shapes: project them into contract-capable Function Tools or make
them deterministically unavailable in TaskSpace. Silent bypass is not acceptable.

## Round 2: 首轮修复闭合审查

### Review Input

- Objective: 对提交 `a26affe1d` 与 `73380f696` 逐项复核 BF-1 至 BF-6，不接受 schema-only
  证明，重点攻击 build failure、ToolSearch 配对、native event 与五层合同。
- Review target: 完整 provider response 所有权、ToolSearch 反馈、ToolSpec runtime admission、
  Standard 隔离、合同和 Docker 证据。
- Reviewer instructions: fresh session，`fork_context=false`，只读，直接读取仓库，不修改文件。
- Timeout policy: high-risk，20 分钟初始窗口。

### Reviewer Launch Records

| Reviewer | Mechanism | Session | Context Forked | Input | Explicitly Excluded | Read-only |
|---|---|---|---:|---|---|---:|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f9075-0b18-7593-8e96-b0c1ce457865` | false | BF-1..BF-6 closure navigation packet | 主会话历史、结论和说服性摘要 | yes |

### Reviewer Timeout Records

| Output | Attempt | Session | Status |
|---|---:|---|---|
| R2-implementation | 1 | `019f9075-0b18-7593-8e96-b0c1ce457865` | completed |

### Reviewer Output

Reviewer verdict: **BLOCK**。

#### Blocking Findings

1. **R2-B1 Critical:** `build_tool_call` 的 model-visible failure 被即时写回但不进入 response
   manifest；合法 side-effect 前缀仍可在 Completed 后执行。
2. **R2-B2 High:** ToolSearch 的 developer supplemental fact 可能位于后续 call pairing
   之前，形成 `tool output -> system/developer -> late tool output`。
3. **R2-B3 High:** TaskSpace 只在 schema 层隐藏 Web/Image；provider/replay event 仍进入
   非 Tool 路径，ImageGeneration 可写文件。
4. **R2-B4 Gate:** governing document hash 陈旧，五层 `-Phase All` 不可复现。

#### BF Closure

| BF | Result |
|---|---|
| BF-1 | closed |
| BF-2 | open: build-failed item 不入 manifest |
| BF-3 | open: build/pairing 缺口 |
| BF-4 | partial: 缺 search -> invoke |
| BF-5 | open: hidden native event 仍可接纳 |
| BF-6 | implementation closed, proof partial |

#### Non-blocking Risks / Missing Evidence

- Docker repeat-1 的两次恢复是 adoption 观察，不是状态破坏；
- 全量 core suite 受历史 stack overflow 和无关失败影响，不能声明全绿；
- 缺 deferred actual invocation、Standard actual dispatch、native event、Fatal/cancel
  和 observer 精确分类。

### Main Agent Response

| Finding | Decision | Action |
|---|---|---|
| R2-B1 | accept | 新增统一 `ProviderToolDeclaration`，Ready/BuildFailed 共用整响应 preflight；真实 SSE 证明 malformed 后缀时合法前缀零执行 |
| R2-B2 | accept | 所有 call pairing output 先完成，再追加 supplemental factual message；增加顺序测试 |
| R2-B3 | accept | hidden Web/Image 的 added/done 在非 Tool handler 前转为 RejectedNative，并按 provider identity 去重 |
| R2-B4 | accept | 同步 governing document、authority、production manifest 与 Rust identity hash |
| BF-4 proof gap | accept | 增加 TaskSpace deferred search -> dynamic invocation 集成测试 |
| BF-6 proof gap | accept | 增加 Standard business-owned `taskspace_binding` 实际 dynamic dispatch 测试 |

### Closure Status

- Accepted blocking findings fixed: yes，提交 `5897cb8ba`
- Required fresh review: yes
- Status: continued to Round 3

## Round 3: Provider Declaration 完整性审查

### Review Input

- Objective: 尝试证伪 `5897cb8ba` 的统一 declaration 序列。
- Risk focus: Function/ToolSearch build errors、pairing 顺序、Web/Image added/done、deferred
  search -> invoke、Standard dispatch、去重、取消与 partial side effect。
- Verification navigation: tools 12/12、core TaskSpace 101/101、sequence 18/18、terminal
  2/2、五层 All 通过；reviewer 被要求独立抽查，不接受这些数字作为结论。

### Reviewer Launch Records

| Reviewer | Mechanism | Session | Model | Context Forked | Input | Read-only |
|---|---|---|---|---:|---|---:|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f9094-dbcf-7d31-89d6-f658a67ca95a` | GPT-5.5 low | false | neutral closure navigation packet | yes |

### Reviewer Timeout Records

| Output | Attempt | Session | Status |
|---|---:|---|---|
| R3-implementation | 1 | `019f9094-dbcf-7d31-89d6-f658a67ca95a` | completed |

### Reviewer Output

Reviewer verdict: **BLOCK**。

#### Blocking Finding

**R3-BF-A:** `ToolSearchCall { execution="client", call_id=None }` 被 Router 的 fallback
返回为 `Ok(None)`，不形成 ProviderToolDeclaration。若它位于合法普通 Tool 后面，Completed
只看到 Ready 前缀并执行。

- Broken assumption: 所有 invalid provider declarations 都进入 BuildFailed 或 RejectedNative。
- Trigger: valid FunctionCall prefix + client ToolSearch missing call_id + Completed。
- Impact: 部分执行，Agent 收不到完整机械失败事实。
- Evidence: `router.rs` 只匹配 `call_id=Some`；`stream_events_utils.rs` 的 `Ok(None)` 走非 Tool；
  `sequence.rs` 对 all-Ready 列表正常 dispatch。
- Proof needed: `id=Some` 和 `id=None` 两种真实 SSE 都应整响应零 dispatch。

#### Non-blocking Risks

- native 无 client pairing 时只依赖 developer fact，需要在合同中明确；
- Custom/freeform 在 TaskSpace 仍是明确 unsupported 边界，不是完整能力支持；
- schema 成本收益未被该 finding 推翻。

#### BF Closure

| BF | Result |
|---|---|
| BF-1 | mostly closed, but reopened by missing-call-id ToolSearch |
| BF-2 | appears closed |
| BF-3 | partial: missing-call-id ToolSearch 仍无失败事实 |
| BF-4 | appears closed |
| BF-5 | partial by explicit unsupported product boundary |
| BF-6 | appears closed |

### Main Agent Response

| Finding | Decision | Action |
|---|---|---|
| R3-BF-A | accept | client ToolSearch 缺 call_id 现在返回 build failure；无法配对的声明使用独立 `UnpairedBuildFailed`，保留 provider item id、tool、payload kind 和原始错误 |
| 去重风险 | accept | 只有真正的 RejectedNative added/done 去重；Ready、BuildFailed、UnpairedBuildFailed 不做泛化去重 |
| missing tests | accept | 一个集成测试顺序执行 `id=Some` 与 `id=None` 两个真实 TaskSpace SSE，均断言前缀文件不存在并存在 `build_failed_unpaired` |

### Closure Status

- Accepted blocking finding fixed: yes，等待提交和 Round 4
- Blocking re-review required: yes
- Status: blocked until fresh Round 4 passes

## Round 4: missing-call-id 闭合复审

### Review Input

- Objective: 尝试证伪 `0111ca95f` 对 client ToolSearch 缺少 `call_id` 的修复，并重新核验
  BF-1 至 BF-6。
- Risk focus: provider item id 有/无、unpaired feedback、dedup、server ToolSearch、Standard
  隔离、native event、整响应零执行和 deferred Tool。
- Reviewer policy: fresh session，`fork_context=false`，只读，不继承主会话结论。

### Reviewer Launch Records

| Reviewer | Mechanism | Session | Context Forked | Read-only |
|---|---|---|---:|---:|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f90a4-0467-72e2-a768-357497f5bdc3` | false | yes |

### Reviewer Output

Reviewer verdict: **PASS**，无 blocking finding。

| Contract | Result |
|---|---|
| BF-1 完整 response 所有权 | closed |
| BF-2 build failure 进入 declaration | closed |
| BF-3 missing-call-id 与忠实失败反馈 | closed |
| BF-4 deferred search -> invocation | closed |
| BF-5 hidden native runtime admission | closed by explicit unsupported boundary |
| BF-6 Standard 隔离 | closed |

Reviewer 确认：

- client ToolSearch 缺少 `call_id` 不再返回 `Ok(None)`；
- provider item id 存在或缺失时都形成 invalid declaration；
- 任一 invalid declaration 使完整 response 的 `executed_tool_call_count=0`；
- `UnpairedBuildFailed` 明确表示无配对协议失败，不伪装成 Tool 执行结果；
- 只有 hidden native added/done identity 进行去重。

### Non-blocking Findings And Response

| Finding | Decision | Action |
|---|---|---|
| Standard 直接测试仅覆盖 provider id 存在 | accept | `b0f61318a` 改为 `Some/None` 双变体，并校验 descriptor identity |
| build-failure 日志缺 provider item/tool/payload 字段 | accept | `b0f61318a` 增加三个结构化事实字段，不改变执行语义 |

### Runtime Validation

- missing-call-id 定向 Rust: 2 passed；
- locked CLI build: passed；
- Docker `single-file-fast-fix`: Standard 与 map-request 均 solved，public/hidden oracle 均通过；
- repeat-1 只形成 E2-candidate，runner 因 `repeats_lt_3` 证据门槛返回 1；run artifact 本身为
  `valid/completed`，不把该诊断 smoke 宣称为 E2；
- TaskSpace 一次 standalone `initialize_map` 被 preflight 零执行拒绝后自纠正，未发生部分执行或
  状态提交。

### Final Closure Status

- Blocking findings found in Round 4: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Deferred blockers: none
- Final status: passed
