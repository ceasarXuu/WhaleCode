# Subagent VS Review: R71-01 direct failure carrier

- Created: 2026-07-31T07:51:28+08:00
- Updated: 2026-07-31T08:52:22+08:00
- Report schema: adversarial-v1
- Task: 执行 R7.1 原子 Phase R71-01，并对 strict direct failure carrier 实施对抗性审查
- Report path: `vs_review/2026-07-31-r71-01-direct-failure-carrier-review.md`
- Review mode: fresh internal subagent
- Source session policy: 不继承主 Agent 上下文，只接收下述中性导航包
- Status: open

## Round 1: implementation adversarial review

### Review Input

#### Objective

验证 commit `2a0530a6c00e10879409775cf238eb64654ec8e3` 是否真正完成 R71-01：
direct failure adapter 必须严格、fail closed、忠实保留事实，且不得把普通 Tool 语义重新解释为
TaskSpace 失败。

#### Review Target

R71-01 的 PowerShell observer 实现、严格 JSON parser、failure envelope validator、回归测试和机器证据。

#### Target Locations

- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `scripts/taskspace-benchmark/lib/r7-json-facts.ps1`
- `scripts/taskspace-benchmark/lib/r7-direct-failure-carrier.ps1`
- `scripts/taskspace-benchmark/lib/r7-call-evidence.ps1`
- `scripts/taskspace-benchmark/lib/r7-state-failure-contract.ps1`
- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
- `scripts/taskspace-benchmark/test-r71-direct-failure-carrier.ps1`
- `scripts/taskspace-benchmark/test-r7-five-layer-trace-analysis.ps1`
- `benchmarks/taskspace/r7/evidence/r71-01-direct-failure-carrier.json`
- 生产事实参考：
  `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs`、
  `third_party/codex-cli/codex-rs/core/src/action_map/response.rs`

#### Change Introduction

direct carrier 入口改为 System.Text.Json 先行解析并拒绝大小写不敏感的重复属性；外层
`toolSuccess` 与内层 `success` 必须一致；`TaskSpaceControlResultV2` failure envelope 增加 action、
status/class、actual/expected、revision 和 state violation 一致性校验；普通 Tool 仅在冒用保留
TaskSpace schema 时进入 untrusted 分支；当前 response-prepare 成功载体
`TaskSpaceResponseCommitV1` 使用独立严格 shape。

#### Risk Focus

- hostile JSON 是否仍可利用 duplicate、大小写、数组、null、数字强转或嵌套结构绕过。
- outer/inner success、state_commit、status/class/code/violation、action/revision 是否仍可相互矛盾。
- 合法生产载体是否被过严 validator 误伤，尤其 response-prepare 和 protocol/argument/resource failure。
- 普通 Tool 自有 JSON schema 是否仍被 TaskSpace 分类污染。
- helper 依赖方向、单文件尺寸和未来 action/schema 漂移是否可维护。

#### User-Perspective Review Focus

- Agent 最终看到的 Tool 成功/失败事实不得被 observer 改写。
- 诊断人员应能只凭 `carrier_schema/parse_status/reason_code` 区分 malformed、untrusted、
  ordinary 和有效 TaskSpace failure。

#### Implementation Completeness Focus

- 不能只存在测试 helper；确认生产 benchmark trace 入口实际调用新 parser。
- 对照 R71-01 的 duplicate、非法 action、outer/inner 冲突、ordinary isolation 四类退出门。
- 核对 evidence artifact 的 required fields、schema、内容与真实测试输出。
- 查找未接入、旧宽松 parser、静默 fallback、mock-only 或重复解析入口。

#### Target Benefit Focus

- 目标收益是 failure evidence 分类正确性和可诊断性；基线为 COE E-034 的四类确定性反例。
- 目标是四类反例全部反转且旧 trace/state/supplemental/performance/cost 合同无回归。
- 不主张 Runtime 性能收益；额外解析成本只需识别潜在明显回归。

#### Assumptions To Attack

- PowerShell 与 System.Text.Json 对 JSON 数字、属性大小写和 root type 的行为一致。
- `TaskSpaceResponseCommitV1` 的 action/revision/reservation shape 与当前 Rust 生产端一致。
- 所有 direct control 输出都携带正确 `ToolName`；TrustedRuntimeCarrier 不会成为绕过入口。
- 普通 Tool 的 JSON 失败可由 ordinary exit 独立分类。

#### Adversarial Lenses

- input
- failure
- data
- implementation-completeness
- maintenance
- testing
- observability

#### Verification Status

- commit: `2a0530a6c00e10879409775cf238eb64654ec8e3`
- R71-01 正反矩阵：PASS
- trace/state/supplemental failure 回归：PASS
- performance observation 与 cost instrumentation self-test：PASS
- five-layer contracts `All`：PASS
- evidence schema 与 required fields：PASS
- 尚未执行 fresh adversarial review。

#### Reviewer Instructions

- 使用全新内部 subagent session，不继承主 Agent 上下文。
- 直接读取目标文件和 commit，保持只读，不修改任何文件。
- 尝试证伪实现，不要确认主 Agent 的结论。
- 每项 finding 给出 broken assumption、触发方式、影响、所需证明，并尽可能引用路径和行号。
- 明确列出 blocking findings、non-blocking risks、缺失测试、缺失日志以及实现完整性。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 分钟 | 最多延长 10 分钟一次 | 2 | 审查不可用时不得通过 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | 最高风险是 untrusted input、跨字段一致性和生产载体误分类 | correctness、failure handling、data consistency |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.6-sol/xhigh` | `019fb571-c2e3-7850-b2b4-d90a3296f913` | spawn tool response，nickname=`Erdos` | `fork_context=false` | Round 1 Review Input | 主 Agent 历史、推理、草稿、结论和完整 diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-Erdos | implementation-adversary | 1 | `019fb571-c2e3-7850-b2b4-d90a3296f913` | 小于 15 分钟上限 | completed | 正常返回 | completed |

### Reviewer Outputs

#### R1-Erdos

##### Summary

结论为 `BLOCKED`。精确反转 E-034 的四个 fixture 尚不足以关闭 R71-01；审查者给出 4 个阻断反例。

##### Blocking Findings

- **B1：跨字段矛盾仍可成为有效 TaskSpace 证据。**
  - Broken assumption: validator 已覆盖 status/class、revision 和 node presence/state 一致性。
  - Failure scenario: `state_rejected + resource`、envelope revision 与 actual revision 冲突、
    `node_present=false + state=waiting` 均被接受。
  - Trigger condition: 可信 carrier 的字段分别合法但组合矛盾。
  - Impact: observer 从矛盾事实中选择语义，违反忠实透传和 fail closed。
  - Proof needed: 三类反例均返回稳定 invalid outcome，并保留合法 protocol/argument/resource 正例。
- **B2：严格验证之前仍有宽松 parser 和 reservation 副作用。**
  - Broken assumption: direct carrier 在任何 observation mutation 前只经过 strict parser。
  - Failure scenario: invalid response-prepare 可先写 sibling reservation；duplicate 可先触发旧 parser 异常。
  - Trigger condition: `toolSuccess=true` 的 invalid/duplicate `TaskSpaceResponseCommitV1`。
  - Impact: 无效 carrier 污染 attribution，且 `parse_status` 不稳定。
  - Proof needed: parse/validate first、一次性 mutate；invalid/duplicate 均零 sibling mutation。
- **B3：普通 Tool 隔离仍被突破。**
  - Broken assumption: 只有保留 TaskSpace schema 才进入 TaskSpace parser。
  - Failure scenario: malformed ordinary JSON + exit code 被报为 JSON parse failure；普通 domain
    `success=false` 被当作 Tool transport failure。
  - Trigger condition: 普通 Tool 输出以 `{` 开头，或业务 JSON 含 `success` 字段。
  - Impact: observer 重写普通 Tool 的真实执行语义。
  - Proof needed: malformed ordinary、domain success、unknown schema、reserved spoof 四类隔离测试。
- **B4：Phase evidence 与权威状态不足以支持关闭。**
  - Broken assumption: artifact 字段来自生产 call row，且 schema 能拒绝缺字段记录。
  - Failure scenario: `carrier_schema` 是测试字面量；通用 schema 接受 `{"anything":1}`。
  - Trigger condition: adapter 字段退化或 artifact 缺字段。
  - Impact: artifact 可与真实 adapter 脱节。
  - Proof needed: call row 一等字段、专用 schema 条件、source hash；通过审查后再执行权威状态迁移。

##### Non-blocking Risks

- **NR1：成功 `TaskSpaceControlResultV2` 没有完整 shape validator。**
  - 影响：未来复用可能出现 success schema drift；R71-01 当前只声明 direct failure 范围。
- **NR2：多次 JSON parse 没有量化性能证据。**
  - 影响：当前没有性能收益声明，但重复解析可能造成额外 observer 开销。

##### User-Perspective Checks

- 合法 protocol/argument/resource failure：通过。
- 合法 `TaskSpaceResponseCommitV1`：通过。
- malformed/duplicate TaskSpace carrier：direct helper 通过，生产 Apply 路径失败。
- 普通 malformed/domain success：失败，被 observer 改写。
- 要求的诊断字段：未成为 call row 一等字段。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Strict JSON | duplicate/root/type fail closed | response item → Apply → adapter | strict parser 晚于 reservation parser | direct fixture | artifact | none | partial | B2 |
| Failure envelope | 跨字段一致 | state/supplemental validators | 已接入 | 部分负例 | call outcome | none | partial | B1 |
| Ordinary isolation | 普通语义不进入 TaskSpace 推断 | response outcome → adapter | domain success 仍被读取 | 单一 ordinary fixture | call outcome | none | partial | B3 |
| Evidence | 真实 call row 与强 schema | call descriptor → artifact | 测试 literal | schema 仅 minProperties | artifact | test-generated | partial | B4 |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| E-034 四反例反转 | 四类误接受/误分类 | 四类均严格 | directed fixture | 4/4 通过 | achieved | 更广反例仍失败 | weak-evidence | B1～B3 |
| 无关联回归 | 旧 tests PASS | 继续 PASS | test suite | 作者报告 | unmeasured by reviewer | 生产 Apply 未覆盖 | weak-evidence | B2 |
| 性能收益 | 未声明 | n/a | n/a | n/a | neutral | 未量化 parser 开销 | deferred | NR2 |

##### Required Fixes

- B1：补 status/class、revision 和 node presence/state 一致性。
- B2：删除 reservation 前置宽松 parse，验证通过后一次性写入。
- B3：普通 Tool 不读取任意 domain `success`，malformed ordinary 继续走 ordinary classifier。
- B4：call row 增加证据字段，artifact 从 call row 派生并强化 schema。

##### Missing Tests

- invalid/duplicate response-prepare 零 attribution mutation。
- supplemental status/class、revision、node presence/state 矛盾矩阵。
- ordinary malformed JSON + exit code、domain `success=false`。
- strict failure evidence 缺字段必须被 schema 拒绝。

##### Missing Logs / Observability

- call row 缺少 `carrier_schema/reason_code`。
- artifact 缺 source hash，不能证明输入来源。

##### Evidence

- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1:131` - 读取任意 output `success`。
- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1:185` - reservation 使用宽松 parser。
- `scripts/taskspace-benchmark/lib/r7-state-failure-contract.ps1:204` - revision 只检查 expected。
- `benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json:13` - record 仅要求 `minProperties:1`。

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| R1-Erdos | B1 | 分别合法的矛盾字段可通过 | blocking | accept | reviewer 最小反例可复现 | 增加 status/class 映射、actual/envelope revision 对账、absent-node/state 互斥及正反测试 | Round 2 |
| R1-Erdos | B2 | invalid carrier 在 strict validation 前写 reservation | blocking | accept | Apply 顺序与旧 `ConvertFrom-Json` 入口成立 | ResponseItem 仅提取 raw fact；唯一 adapter 验证后，reservation 全量校验并一次性写入 | Round 2 |
| R1-Erdos | B3 | ordinary domain JSON 被当成 Tool contract | blocking | accept | ordinary malformed/domain success 反例成立 | 移除文本内层 `success` 推断；按 Tool identity 与明确 transport fact 分类 | Round 2 |
| R1-Erdos | B4 | evidence literal 与弱 schema 可脱离 call row | blocking | accept | schema mutant 确实通过 | call row 新增 `carrier_schema/reason_code`；artifact 经 Apply 生成并带 source hash；schema 增加 strict artifact 条件 | Round 2 |
| R1-Erdos | B4-status | 首轮审查时 register 尚未关闭 | blocking | reject | 关闭门要求 accepted blocker 完成 fresh re-review；在首轮前关闭会破坏治理顺序 | Round 2 通过后再写 evidence hash 和状态迁移 | Round 2 后 |
| R1-Erdos | NR1 | successful ControlResultV2 没有完整 shape validator | non-blocking | reject | R71-01 的机器边界明确是 direct failure；response-prepare success 仅因 reservation 副作用进入本次验证 | 不扩张到 control success 合同 | none |
| R1-Erdos | NR2 | parser 开销未量化 | non-blocking | reject | 本 Phase 不声明性能收益；R71-16/17 分别负责固定与动态成本验收 | 删除重复 parser，成本结论留给既有成本门 | R71-16/17 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: implementation and directed tests complete
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Rejected findings backed by evidence: yes
- Deferred findings documented: n/a
- Implementation completeness gaps resolved or accepted by user: pending re-review
- Target benefit warnings recorded: yes
- Blocked reason: accepted blocking fixes require a fresh closure review
- Allowed to proceed: no

## Round 2: accepted blocker closure review

### Review Input

#### Objective

只读复审 commit `4ad980f1efc58155ea54f3e5599cad44d23bea1b` 是否关闭 Round 1 的 B1～B4，
并尝试发现修复引入的新 correctness 或 boundary 回归。

#### Review Target

- Round 1 report、findings 和 Main Agent response。
- `52d69df8f` 的 accepted blocker 修复。
- `4ad980f1e` 的 report-level strict fixture 修正。

#### Target Locations

- Round 1 列出的全部实现、测试和 evidence 文件。
- `scripts/taskspace-benchmark/test-r7-request-observability-report.ps1`
- 当前 `git diff 2a0530a6c..4ad980f1e`

#### Risk Focus

- B1：status/class、revision、node presence/state 是否真正 fail closed，合法生产正例是否保留。
- B2：strict parse 是否确实先于任何 reservation mutation，失败是否保持稳定 reason。
- B3：ordinary Tool 是否完全独立于 domain `success` 与 malformed JSON。
- B4：call row/evidence/schema 是否形成可追溯闭环，而非换一种 test literal。
- 新风险：unknown transport status、partial reservation、supplemental carrier、report taxonomy。

#### Implementation Completeness Focus

- 必须沿 `Get-R7ResponseItemOutcome -> Apply-R7ObservedOutcome -> Get-R7CallOutcome` 真实入口核对。
- 验证 artifact 每个 record 可追溯到 Apply 后 call row 与 source hash。
- 核对 request observability report 的合法 control fixture，不接受放宽 parser。

#### Verification Status

- Round 1 B1～B4 定向反例：PASS。
- trace/state/supplemental/exact-count/provider-token/state-summary/performance/cost/contracts：PASS。
- clean-worktree request observability report：PASS。
- Phase register 尚未关闭，等待本轮 blocker closure review。

#### Reviewer Instructions

- 全新内部 subagent，`fork_context=false`，只读。
- 不接受主 Agent 的修复结论，直接重放或构造反例。
- 逐项给出 B1～B4：closed / still open / regressed，并引用路径与行号。
- 新 blocking finding 必须包含 trigger、impact、proof needed。
- 不修改文件。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 分钟 | 最多延长 10 分钟一次 | 2 | 未完成不得通过 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | accepted blocking closure 必须由新的实现审查者逐项证伪 | correctness、failure boundaries、evidence integrity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.6-sol/xhigh` | `019fb587-b86a-77b1-b250-b8203607272b` | spawn tool response，nickname=`Lagrange` | `fork_context=false` | Round 2 Review Input | 主 Agent 与 Round 1 reviewer 上下文、推理和结论 | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R2-Lagrange | implementation-adversary | 1 | `019fb587-b86a-77b1-b250-b8203607272b` | 小于 20 分钟上限 | completed | 正常返回 | completed |

### Reviewer Outputs

#### R2-Lagrange

##### Summary

结论为 `BLOCKED`。Round 1 的 ordinary Tool 隔离和 artifact 追溯已关闭；revision
精确类型、response-prepare 与原始 control request 的身份绑定、以及 transport status
缺失时的 fail-closed 仍有缺口。

##### Round 1 Closure

| Round 1 Finding | Result | Evidence |
|---|---|---|
| B1 跨字段矛盾 | still open | PowerShell 宽松比较仍接受字符串或布尔 revision |
| B2 strict parse 与 reservation 副作用 | still open | response commit 未与原请求 action/expected_revision 对账 |
| B3 ordinary Tool 隔离 | closed | ordinary 输出不再读取业务 `success` 语义 |
| B4 evidence 追溯 | closed | call row 一等字段、source hash 和专用 schema 已接通 |

##### New Blocking Finding

- **N1：TaskSpace transport status 缺失时 fail open。**
  - Trigger：TaskSpace direct/trusted carrier 没有布尔 `toolSuccess`，或携带字符串 `"true"`。
  - Impact：observer 可能把 transport 事实缺失的载体认作有效成功/失败证据。
  - Proof needed：缺失和非布尔 transport status 均产生稳定 invalid outcome，且不写入
    sibling reservation。

##### Non-blocking Risks

- evidence schema 不能对任意 taxonomy/hash 做密码学绑定；当前 source hash 已满足本 Phase
  的可追溯范围。
- supplemental `originalRole` 尚未与 raw role 交叉验证，属于后续 carrier provenance 范围。
- 其他 protocol 字段仍有默认大小写比较；本轮只要求身份、action 和 revision 精确。
- successful `TaskSpaceControlResultV2` 完整 validator 仍在 R71-01 范围外。

##### Required Fixes

- revision 对账只接受 exact nonnegative Int64，不接受字符串、布尔或隐式强转。
- `TaskSpaceResponseCommitV1` 必须绑定原始 control action 与 expected_revision。
- transport status 必须存在且为布尔值。
- 绑定失败不得产生部分 sibling reservation，且应暴露稳定诊断字段。

### Main Agent Response

| Finding | Decision | Action Taken | Deterministic Evidence |
|---|---|---|---|
| B1 exact revision | accept | envelope/actual/expected revision 统一走 exact nonnegative Int64 | string/bool actual 与 expected revision 反例 |
| B2 request binding | accept | 新增独立 response-commit binding 模块，逐项核对 action、revision 与 request identity | action/revision/missing revision/case drift 反例 |
| B2 atomic reservation | accept | 先验证全部 reservation，再一次性写入；成功后记录 `reservation_mutated=true` | 双 sibling 后段 mismatch 零写入反例 |
| N1 transport status | accept | TaskSpace direct/trusted carrier 要求布尔 `toolSuccess`，否则 fail closed | missing/string transport 反例 |
| non-blocking risks | defer | 不扩张 R71-01 的 direct failure adapter 边界 | 交由对应后续 Phase 或未来 issue |

### Closure Status

- Round 1 blocker closure: implementation complete, fresh review pending
- New blocking findings: implementation complete, fresh review pending
- Allowed to close R71-01: no

## Final Conclusion

Round 2 接受项已全部实现并通过本地确定性测试。按项目审查门禁，代码变更后的 fresh
closure review 需要用户再次授权；在此之前 R71-01 保持 `repair`，不伪造关闭结论。
