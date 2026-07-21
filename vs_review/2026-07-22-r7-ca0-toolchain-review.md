# Subagent VS Review: R7 CA-0 executable-v2 工具链

- Created: 2026-07-22 00:40:39 +0800
- Updated: 2026-07-22 00:40:39 +0800
- Report schema: adversarial-v1
- Task: 在 FLA-3.5 CA-1 前完成、审查并不可变锚定 continuous-action executable-v2 工具链。
- Report path: `vs_review/2026-07-22-r7-ca0-toolchain-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: 首次实现审查

### Review Input

#### Objective
判断 `cb2beaff2` 是否真正满足 CA-0，能否由下一提交建立不可变工具链 anchor。

#### Review Target
候选 schema/generator/transition/verifier、严格 JSON、Rust closure、冻结评估合同、completion bootstrap/workflow 和测试。

#### Target Locations
- `docs/v0.0.5/build-R7/33-r7-continuous-action-regression-repair-plan.md`
- `benchmarks/taskspace/r7/*v2*.schema.json`
- `scripts/taskspace-benchmark/*r7-continuous-action*.ps1`
- `third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure*`
- `.github/workflows/r7-continuous-action-completion.yml`

#### Change Introduction
新增 candidate-only executable-v2 工具链；active authority 和 production manifest 在 CA-6 前保持不变。

#### Risk Focus
- pinned bootstrap 是否可能自证、替换或恢复后掩盖历史篡改。
- candidate identity、全局状态、promotion/revert、artifact 和 evaluation 是否真的可执行。
- closure 是否覆盖真实生产路径，required check 是否可信。

#### User-Perspective Review Focus
- 工具链失败是否提供明确机械错误，紧急回滚是否可执行。

#### Implementation Completeness Focus
- 区分 schema/scaffold 与真实 generator、transition、history replay、evaluator、bootstrap 生产路径。

#### Target Benefit Focus
- correctness、request/token/cache/time 非劣性必须由冻结输入和可重放 evaluator 证明，不能由 candidate 自签。

#### Assumptions To Attack
- first-parent、单 pending/promoted、无 orphan、无 symlink/mode 漂移、失败原子性、完整 Tool closure、GitHub run identity。

#### Adversarial Lenses
- requirements、state、input、concurrency、failure、security、implementation-completeness、testing、observability

#### Verification Status
- 实现提交前报告 self-test PASS、registry 41 pass/1 ignored、sequence 16 pass、五层/ownership/actionlint PASS。
- 尚未有 generator→transition→promotion→revert→bootstrap 黑盒测试。

#### Reviewer Instructions
- Fresh internal subagent session；`fork_context=false`。
- 只读仓库和 Git 对象，不修改文件；优先给出可复现路径和行号。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 30 minutes | none | 2 | accepted blocking findings require fresh re-review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| trust-boundary/state-machine reviewer | CA-0 将成为后续 promotion 的不可变信任根 | history、identity、atomicity、closure、required check |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| trust-boundary/state-machine reviewer | `multi_agent_v1__spawn_agent`, `gpt-5.6-sol`, xhigh | `019f8579-4842-7460-b640-91837b87bd6e` | spawn tool call + completion notification | `fork_context=false` | Round 1 neutral navigation packet | main-agent history、reasoning、drafts、conclusions | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1 | trust-boundary/state-machine reviewer | 1 | `019f8579-4842-7460-b640-91837b87bd6e` | within 30 minutes | completed | final report returned | completed |

### Reviewer Outputs

#### R1

##### Summary
`cb2beaff2` 没有修改 active authority/production，但 executable-v2 尚不能建立 anchor；信任链、candidate 语义、全局状态、原子命令、closure、evaluation 和 required check 均存在可绕过路径。Final verdict: `REJECT`。

##### Blocking Findings
1. **CRITICAL：bootstrap/history 不完整。** `r7-v2-toolchain-core.ps1:117,150` 只看最终 bytes；没有重放 baseline supersession、pinned toolchain/authority/production 的中间篡改，且使用一般 ancestor 而非 first-parent。
2. **CRITICAL：candidate identity 未绑定 promotion 语义。** ID 未包含完整 activation target/promotion contract；verifier 从 candidate 自己的 patch 推导 expected，`add` 也未验证 `new_value_sha256`。
3. **CRITICAL：全局 candidate 不变量不在 transition 中执行。** 可以依次创建两个 `promotion_pending`，promotion 后也可能遗留 pending。
4. **CRITICAL：8 个 artifact 主要只有字段外形。** transition、typed outcome、wire golden、oracle 和 rollback 缺实际 instance/value/trace 及重算规则，self-test 用随机 hash 即可通过。
5. **CRITICAL：closure 不是生产可达真实闭包。** 四个合成 profile 只出现 26/33 handler；存在死分支/错误映射风险，静默 dedup 违反重复即失败。
6. **CRITICAL：closure inventory 过窄且 pipeline 泛化。** 约 472 个扫描文件只绑定约 40 个；新增未硬编码 carrier runtime 不改变 digest，各入口没有表达实际 decorator/registry/handler 差异。
7. **CRITICAL：generator/transition/revert 失败不原子。** commit 后才 verifier，失败会留下非法 commit；generator 两提交中断会留下 orphan；rollback blanket preserve 可能漏回滚运行时代码。
8. **CRITICAL：completion evidence 可自签。** 只检查布尔值和引用 hash，没有 evidence schema、raw run 重算、seed/order/metric evaluator。
9. **CRITICAL：evaluation 使用 FLA-8 held-out 且不可执行。** `multi-file-order-pipeline` 属于 held-out；只绑定 `scenario.json`，没有 evaluator 实现。
10. **HIGH：required check 不固定且拒绝合法 revert。** GitHub actions 使用可变 tag；attestation 未绑定 workflow/repository/event/run attempt；revert 后无 promoted pointer 会必然失败。
11. **HIGH：path/mode/strict I-JSON 不完整。** 没有对 artifact 实际 `ls-tree` mode、symlink/ReparsePoint、I-JSON 数值/孤立 surrogate、ordinal canonicalization 做机械验证。

##### Non-blocking Risks
- 自制 JSON Patch/canonicalization 有数字和 culture 风险。
- `sha2`/`syn` 作为 `codex-tools` 普通依赖扩大生产依赖面。
- phase ownership 无独立 schema。
- 本地生成结果不是 required-check 重建的持久证据。

##### User-Perspective Checks
- Usability: risk - 紧急 revert 会被 required check 当成失败。
- Ease of use: risk - 中途失败可能留下 orphan/非法 commit，恢复路径不明确。
- Ease of understanding: risk - 当前 PASS 输出无法区分 schema 自洽与端到端可信。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Status | Finding Link |
|---|---|---|---|
| immutable history/bootstrap | pinned first-parent replay | partial | 1 |
| candidate identity/global state | independent expected contract + one active state | partial | 2,3 |
| 8 executable artifacts | actual instances/schema/traces | scaffold-only | 4 |
| generated closure | all reachable entries and exact pipeline | partial | 5,6 |
| atomic generator/transition/revert | no branch-visible partial event | partial | 7 |
| frozen executable evaluation | raw-run evaluator, no held-out | not-started | 8,9 |
| external required check | immutable action/run identity + promotion/revert | partial | 10 |
| path/I-JSON hardening | actual tree/path/canonical checks | partial | 11 |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Result | Status | Finding Link |
|---|---|---|---|---|---|---|
| correctness/cost non-inferiority | sibling baseline | frozen thresholds | candidate-provided booleans | unmeasured | unmeasured | 8,9 |

##### Required Fixes
- 修复全部 11 个 blocking findings 后重新运行空白上下文审查。

##### Missing Tests
- drift-restore、second-parent、双 pending/promoted、orphan、chmod/symlink、HEAD/concurrent staging、commit 后失败。
- 全 33 handler、真实 wire、完整 rollback drill、合法 revert workflow、完整端到端 bootstrap。

##### Missing Logs / Observability
- required-check 需要绑定并记录 repository/workflow/event/attempt/target。
- evaluator 需要保存 raw run set、重算 report 和 gate provenance。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| R1 | 1 history/bootstrap | critical | accept | 恢复后掩盖路径成立 | 待实现 first-parent 全链重放和 pinned-path 变更拒绝 | Round 2 |
| R1 | 2 identity/promotion self-proof | critical | accept | expected contract 必须独立生成 | 待把 activation/promotion template 纳入 ID 并 exact compare | Round 2 |
| R1 | 3 global candidate invariant | critical | accept | transition 只看单 candidate | 待新增 candidate-set verifier 并做 prospective gate | Round 2 |
| R1 | 4 hollow artifacts | critical | accept | 当前 fixture hash 不等于可执行 instance | 待升级 role schema/semantic linter/self-test | Round 2 |
| R1 | 5 closure reachability | critical | accept with correction | 当前 code-mode 实际输出为 `Freeform/CodeModeExecute`，但 dead branch、26/33 和 silent dedup 仍成立 | 待扩 profile、全 handler gate、duplicate fail | Round 2 |
| R1 | 6 closure inventory/pipeline | critical | accept | relevant-only inventory 无法发现新 runtime | 待绑定完整扫描 inventory 并按入口表达真实差异 | Round 2 |
| R1 | 7 atomicity/rollback | critical | accept | 黑盒测试已另发现锚后命令缺陷，happy-path preflight 不足 | 待使用 prospective Git object 验证后 CAS 更新 ref，收窄 preserve | Round 2 |
| R1 | 8 self-signed evidence | critical | accept | 当前布尔值可伪造 | 待新增 evidence/run-set schema 和 pinned evaluator 重算 | Round 2 |
| R1 | 9 held-out/evaluator | critical | accept | 与 FLA-8 contract 明确冲突 | 待移除 held-out，绑定完整 dev fixture/probe，落 evaluator | Round 2 |
| R1 | 10 required check/revert | high | accept | mutable action tag 和合法 revert 失败均成立 | 待 pin SHA、绑定 run identity、增加 revert attestation path | Round 2 |
| R1 | 11 path/mode/I-JSON | high | accept | manifest mode 声明不是 Git tree 事实 | 待补 tree mode、real path、surrogate/number/ordinal canonical tests | Round 2 |
| R1 | non-blocking risks | non-blocking | accept | 均为后续维护或证据风险 | 与 blocking 修复一并收敛；评估依赖移入独立工具 | Round 2 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 2 pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: no
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: Round 1 `REJECT`; anchor 禁止创建。
- Allowed to proceed: no

## Final Conclusion

CA-0 尚未收口。修复全部 accepted findings 并通过新的空白上下文 Round 2 前，不得创建 toolchain anchor 或进入 CA-1。

## Round 2: accepted blockers 修复复审

### Review Input

- Objective: 判断 `54165032e` 是否可由下一提交建立 immutable toolchain anchor。
- Review target: Round 1 的 11 个 blocker、candidate/evaluator/completion/rollback 全链和新增黑盒 lifecycle replay。
- Target locations: CA-0 计划、v2 schemas、`r7-v2-*`、candidate/evaluator/completion scripts、Rust closure generator、required-check workflow、E-014/E-015。
- Change introduction: history/identity/global state/atomic ref publication、462-entry closure、executable fixtures、sealed evaluator、pinned completion/revert 已实现；active authority/production 未激活。
- Risk focus: candidate self-proof、production reachability、first-parent/concurrency、rollback、anchor transitive trust、GitHub run identity。
- User perspective: 非法事件必须原子失败；合法 emergency revert 必须通过并可恢复。
- Implementation completeness: 区分 schema/fixture 自洽与生产实现真实执行。
- Target benefit: CA-0 不证明 Agent 收益，但必须提供可信的 correctness/cost measurement 基建。
- Verification status: isolated lifecycle PASS；toolchain/strict/evaluator/five-layer/ownership/actionlint/registry/sequence 回归 PASS。
- Reviewer instructions: 全新只读 session，直接检查仓库，不继承主 Agent 上下文，不修改文件。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| trust-boundary/state-machine reviewer | accepted blocking closure 需要重新攻击不可变信任根 | evaluator provenance、artifact execution、closure、rollback、Git/GitHub identity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| trust-boundary/state-machine reviewer | `multi_agent_v1__spawn_agent`, `gpt-5.6-sol`, xhigh | `019f85f0-915f-71d0-bcbb-d9ffa793c2a0` | spawn tool call + completion notification | false | Round 2 neutral navigation packet | conversation、reasoning、drafts、main-agent conclusions | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R2 | trust-boundary/state-machine reviewer | 1 | `019f85f0-915f-71d0-bcbb-d9ffa793c2a0` | within 30 minutes | completed | final report returned | completed |

### Reviewer Outputs

#### R2

##### Summary

Final verdict: `REJECT`。baseline 未漂移、formal anchor 不存在，但 evaluator/artifact/closure/rollback 仍允许生成者定义并证明自身，另有 successor、shared index、anchor path 和 workflow dispatch 身份缺口。

##### Blocking Findings

1. **CRITICAL：evaluation 仍由 candidate 自认证。** evaluator 信任 committed `verdict.correct` 和 candidate 提供的 events；未约束 `run_id`/artifact 复用，Holm 结果未进入最终 gate。要求 trusted runner invocation、typed raw event、oracle 重算、唯一 run/artifact 和 corrected decision gate。
2. **CRITICAL：四类 executable artifact 仍是自洽 fixture。** verifier 只重算字段 hash/schema/标签，没有调用 production transition/parser/outcome/oracle。要求 pinned executor 对 candidate production code 计算 byte-exact state/output/trace，并固定允许的 source path/symbol。
3. **CRITICAL：production entry closure 仍不完整且部分合成。** 472 个 root Rust source 仅 40 个进入 inventory；六个 profile 为合成组合，DeepSeek rows 手工转换而非调用真实 mapper。要求完整 source inventory、production config profiles 和真实 mapping pipeline。
4. **CRITICAL：rollback/revert 未证明真实 production restoration。** rollback inventory 没有独立重算；现有集成候选全为 preserve，未执行 restore/remove；revert 复用 promotion success evidence。要求 candidate production commit 上重算全 inventory，真实覆盖 add/modify/delete/mode，并使用绑定 promoted commit 的独立 typed failure evidence。
5. **HIGH：旧 terminal candidate 会阻断 successor promotion。** 每个 reverted/rejected candidate 都要求当前 authority 等于 baseline，即使已 superseded。要求 terminal record 只验证 causal history，由 global set verifier 检查当前 authority/pointer。
6. **HIGH：Git ref 原子但 shared index/worktree 非事务。** prospective commit 使用共享 index，CAS 失败后按 stale head reset；candidate source 只要求普通 ancestry。要求 private index、exact tree delta、CAS 后安全恢复和 first-parent candidate chain。
7. **HIGH：anchor 可自选 role path，执行依赖未固定。** launcher 只固定 role names，信任 anchor 自报 paths；closure 编译依赖未固定 workspace/toolchain/pwsh。要求 exact role-to-path map 与 digest-pinned execution image 或等价 transitive pin。
8. **HIGH：`workflow_dispatch` completion identity 不明确。** 任意 target input 的 check run 不一定附着于 target；workflow ref 仅 substring 检查。要求 target-associated check，或 exact 绑定 `GITHUB_SHA`、workflow ref/SHA/blob。

##### Non-blocking Risks

- attestation 无独立 schema/签名，artifact 30 天过期。
- PowerShell canonicalization 维护成本。
- `sha2`/`syn` 增大 closure generator 依赖面。
- CA-0 尚未证明 Agent 产品收益。

##### Round 1 Closure Matrix

| # | Status |
|---|---|
| 1 first-parent replay | reopened：candidate 普通 ancestry/transitive inputs |
| 2 ID/promotion binding | closed |
| 3 global pending/promoted | closed；新增 successor blocker |
| 4 hollow artifacts | reopened |
| 5 production closure | reopened |
| 6 closure inventory | reopened |
| 7 atomicity/rollback | reopened |
| 8 self-signed completion | reopened |
| 9 held-out/evaluator | held-out closed；provenance reopened |
| 10 workflow/revert | action SHA closed；dispatch/evidence reopened |
| 11 path/mode/I-JSON | strict JSON closed；anchor/run/rollback path-mode reopened |

##### Missing Tests / Logs

- successor lifecycle、forged verdict/event、duplicate run/artifact、oracle/Holm boundary。
- restore/remove/delete/chmod/symlink rollback。
- concurrent index/ref CAS/second-parent/merge。
- invalid revert evidence 与真实 target-associated GitHub run。
- exact `54165032e` lifecycle replay 与 branch-protection evidence。

##### Final Gate

Formal add-only anchor **不得执行**。

### Main Agent Response

| Finding | Severity | Decision | Evidence / Reason | Action | Follow-up |
|---|---|---|---|---|---|
| R2-1 evaluator self-authentication | critical | accept | 内容 hash 不能证明 runner/provenance；Holm 未参与 decision | 建立 trusted runner contract、typed events、oracle replay、unique artifact gate | Round 3 |
| R2-2 self-consistent executable fixtures | critical | accept | 当前 semantic linter 未执行生产实现 | 增加 pinned production executor 与 exact oracle | Round 3 |
| R2-3 synthetic/incomplete closure | critical | accept | binding-only inventory 与手工 DeepSeek projection 成立 | 全 source inventory + production profile/mappers | Round 3 |
| R2-4 rollback/revert proof | critical | accept | integration 未覆盖 CA-3/4 runtime delta，revert evidence 类型错误 | production commit inventory 重算、真实 restore/remove、typed failure evidence | Round 3 |
| R2-5 successor lifecycle | high | accept | per-candidate current authority check 职责越界 | terminal causal/local 与 global current state 分离 | Round 3 |
| R2-6 shared index transaction | high | accept | CAS 不保护 index/worktree，stale recovery 成立 | private index + exact delta + first-parent | Round 3 |
| R2-7 anchor path/transitive trust | high | accept | role-to-path 可替换；host toolchain 非确定 trust root | exact map + digest-pinned CA-0 image | Round 3 |
| R2-8 workflow dispatch identity | high | accept | dispatch check 与 supplied target 不同一身份 | 移除任意 target dispatch，exact push identity | Round 3 |
| attestation schema/retention | non-blocking | accept | 长期审计需要机械结构 | schema 化；retention 明确为 CI artifact 非永久 source | Round 3 |
| canonicalization/dependency surface | non-blocking | accept | 需被 pinned image 与 tests 包住 | 纳入 image digest 与回归 | Round 3 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Round 3: pending
- Allowed to proceed: no

## Updated Final Conclusion

Round 2 `REJECT`。CA-0 仍不得创建正式 anchor；必须完成 R2-1 至 R2-8 并由新的空白 session 执行 Round 3。
