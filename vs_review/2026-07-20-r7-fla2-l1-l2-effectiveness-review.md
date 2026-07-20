# Subagent VS Review: R7 FLA-2 L1/L2 Effectiveness

- Created: 2026-07-20T20:46:49+08:00
- Updated: 2026-07-20T21:08:13+08:00
- Report schema: adversarial-v1
- Task: 对 FLA-2 TaskSpace L1/L2 的实际有效性和 Whale Agent 执行路径进行独立对抗性审查
- Report path: `vs_review/2026-07-20-r7-fla2-l1-l2-effectiveness-review.md`
- Review mode: fresh internal subagent
- Source session policy: no inherited main-agent context; reviewer receives only the neutral review packet
- Status: open

## Round 1: FLA-2 Production And Runtime Evidence Review

### Review Input

#### Objective

独立判断 R7 FLA-2 是否真正完成了 TaskSpace L1/L2 的生产装配，以及这些内容是否有效、清晰、无冲突地
帮助 Whale Agent 使用 Map。重点寻找 provider context、工具合同、运行路径和性能结论中的反例或异常。

#### Review Target

- TaskSpace L1/L2 文本、ownership、生产装配、DeepSeek wire 映射与观测实现。
- FLA-0 与 FLA-2 两轮 Docker 对照中的 Agent 请求、工具、控制反馈、Map 和缓存路径。
- FLA-2 结果文档对正确性、工程收益、行为效果和遗留问题的表述是否受证据支持。

#### Target Locations

- `third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md`
- `third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v2.md`
- `third_party/codex-cli/codex-rs/core/src/context/taskspace_contract.rs`
- `third_party/codex-cli/codex-rs/core/src/context/base_instructions_profile.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs`
- `benchmarks/taskspace/r7/base-instructions-contract.json`
- `benchmarks/taskspace/r7/five-layer-contract-authority-v1.json`
- `docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md`
- `docs/v0.0.5/build-R7/24-r7-taskspace-five-layer-concrete-contract-draft.md`
- `docs/v0.0.5/build-R7/25-r7-five-layer-executable-spec.md`
- `docs/v0.0.5/build-R7/28-r7-five-layer-fla2-result.md`
- `benchmarks/taskspace/r7/five-layer-fla2-result.json`
- `target/r7-five-layer/fla0-smoke/single-file-fast-fix/20260720-183056-155/`
- `target/r7-five-layer/fla0-smoke/subscription-billing-repair/20260720-183107-387/`
- `target/r7-five-layer/fla2-smoke/single-file-fast-fix/20260720-190153-169/`
- `target/r7-five-layer/fla2-smoke/subscription-billing-repair/20260720-190210-285/`

#### Change Introduction

FLA-2 将 TaskSpace Base 更新到 v2.0.0，只保留 Map 的宏观模型和 Agent/Runtime 责任边界；把具体工作循环
提取为固定的 `taskspace-core-v2`，装配为现有 developer bundle 第一段。DeepSeek Chat wire 将 Base 和该
bundle 映射为相邻 system messages。Tool schema、状态机、projection policy 和控制结果合同在本阶段不变。

#### Risk Focus

- L1/L2 是否只是字节存在，而在真实首请求中被更长 Base、工具 schema、bootstrap handle 或其他 system
  section 淹没、冲突或误导。
- L1 与 L2 是否重复、遗漏关键协议，或把设计者意图误当成 Agent 可执行方法。
- L2 要求初始化/交接与普通动作同响应，但现有 L4 的 `required_next_call`、provider parallel tool calls 和
  Runtime preflight 是否共同形成不一致或难以遵守的合同。
- `no_task_path`、重复 `initialize_map`、`taskspace_required_next_call_missing`、`invalid_arguments`、额外
  transition/read_map/terminal 请求是否说明执行路径存在机制性异常。
- 结果聚合是否遗漏 gate failures、重复调用、失败后恢复成本，或把历史与同轮波动错误归因于 FLA-2。
- L2 作为 DeepSeek 第二条 system message 第一 section 的实现，是否在其他 wire API、resume、fork、
  compaction 或 base override 下缺失、重复或位置失效。

#### User-Perspective Review Focus

- Agent 是否能够从当前 L1/L2 和工具反馈中自然理解 Map 的价值、初始化顺序、节点边界、组合调用和恢复方法。
- 协议是否要求 Agent 做实际 carrier/schema 无法清楚表达的动作，导致“先犯错、再靠拒绝学习”。
- 失败反馈是否忠实且足够让 Agent 自主纠正，还是存在语义缺失、扭曲或重复污染。

#### Implementation Completeness Focus

- L1/L2 是否从版本化 artifact 经唯一生产 owner 进入每个 TaskSpace provider 请求，且 Standard 零注入。
- Chat/Responses 等真实适配入口、session 初始化、base override、测试和 trace 是否覆盖产品声明。
- 65/65 TaskSpace 与 56/56 Standard wire 统计是否由正确 logical-mode side 构成，是否存在统计盲区。
- Docker 样本、公开/隐藏 validator、Map 健康、control 失败分类与 provider request 计数是否互相一致。

#### Target Benefit Focus

- 确定性声明：固定 TaskSpace system 内容从 22,700 降到 21,666 bytes/request，约减少 258 个估算 token；
  Standard 只少 132 bytes。核对计算和 carrier 边界。
- 行为观察：两个样本 Request/Input 放大缩小，复杂样本 wall 接近 Standard，但简单样本 wall 变差。判断是否
  存在冷缓存、样本方差、异常路径或统计口径导致的伪收益。
- 明确区分 production wiring、上下文成本、协议遵循度和任务质量，不把六组 smoke 当作统计收益证明。

#### Assumptions To Attack

- system message 中的存在和精确 hash 等同于 Agent 能有效使用协议。
- Base 缩短和 L2 提取不会降低显著性、破坏缓存或引入跨层冲突。
- `required_next_call_missing` 不下降只是 L4 后续问题，而不是 L2 文本或装配失败。
- aggregate 中的 `control_failures` 足以代表 Agent 实际遭遇的所有 TaskSpace gate/control 失败。
- 所有 Map 闭合且 validator 通过足以排除低效、偶然或错误恢复路径。
- 当前 ChatCompletions 证据足以支持生产层面的通用装配声明。

#### Adversarial Lenses

- requirements
- state
- failure
- usability
- ease-of-use
- comprehension
- implementation-completeness
- target-benefit
- testing
- observability
- maintenance

#### Verification Status

- FLA-0/1/2 PowerShell contracts passed。
- Rust tests passed：L2/context 5、base profile 3、provider wire trace 11、terminal integration 2。
- 两个 Docker 样本各 3 pair；Standard 与 TaskSpace 全部通过公开/隐藏 validator，engineering clean。
- 候选 TaskSpace wire 65/65 L1/L2 exact match；Standard 56/56 L2 absent。
- 未运行完整 Cargo workspace suite；FLA-2 没有覆盖正式 FLA-8 统计矩阵。

#### Reviewer Instructions

- 使用全新内部 subagent session；不继承主 Agent 对话、推理、草稿或结论。
- 直接读取目标源码、文档和运行 artifact；不要把本报告中的风险提示当作结论。
- 只读审查，不修改任何文件，不提交代码。
- 优先寻找高影响反例；不要把纯风格偏好升级为 blocking finding。
- 对每个主要 finding 给出 broken assumption、触发条件、影响、证据路径/行号和关闭该 finding 所需证明。
- 输出必须包含：Summary、Blocking Findings、Non-blocking Risks、User-Perspective Checks、
  Implementation Completeness Checks、Target Benefit Checks、Required Fixes、Missing Tests、
  Missing Logs / Observability、Evidence。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one bounded 10-minute extension | 2 | accepted blocking finding requires fix and fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace protocol and runtime-evidence adversary (`gpt-5.6-sol`, xhigh) | 用户指定空白高推理模型；目标横跨 prompt、provider wire、Tool/Runtime 与 trace | L1/L2 有效性、合同一致性、异常执行路径和证据口径 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace protocol and runtime-evidence adversary | `multi_agent_v1.spawn_agent` | `019f7f92-4713-7413-b334-541eddca94f0` (`Ptolemy`) | spawn tool call and completion notification | `fork_context=false` | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions and persuasive diff summary | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| `round1-ptolemy` | TaskSpace protocol and runtime-evidence adversary | 1 | `019f7f92-4713-7413-b334-541eddca94f0` | 10 minutes + about 6 minutes extension | completed_after_extension | first wait returned no final status; session remained healthy and completed during its single bounded extension | completed |

### Reviewer Outputs

#### `round1-ptolemy`

##### Summary

**Verdict: block `active_verified`.** L1/L2 bytes are correctly selected and observable on the DeepSeek Chat path, and
all scenario validators passed. However, the production carrier violates the highest-authority wire contract, while active L2
instructs recovery from fields that current rejection outputs do not provide. These are contract failures, not merely stochastic
model behavior.

##### Blocking Findings

1. **Production wire violates the authoritative carrier contract.**
   - Broken assumption: FLA-2's live wire matches the governing two-system-message layout.
   - Failure scenario: every TaskSpace request contains a third system message carrying a bootstrap Map handle before user/history
     messages.
   - Trigger: `map-request`; observed in all 65 TaskSpace requests.
   - Impact: the governing spec explicitly says there is no third system message, and the authority conflict policy blocks the
     affected phase. The 65/65 check validates L2 identity/position but not the complete carrier shape.
   - Proof needed: resolve the contradictory documents, update authoritative hashes, assert the complete role/order/count shape,
     and rerun all wire checks.
   - Evidence: `docs/v0.0.5/build-R7/25-r7-five-layer-executable-spec.md:55`,
     `docs/v0.0.5/build-R7/24-r7-taskspace-five-layer-concrete-contract-draft.md:45`,
     `benchmarks/taskspace/r7/five-layer-contract-authority-v1.json:7`,
     `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3712`, and the first FLA-2 TaskSpace
     `provider-wire-trace.jsonl:1`.

2. **L2's rejection-recovery contract is impossible against current L5 outputs.**
   - Broken assumption: on rejection, the Agent can inspect the action, submitted values, observed canonical values, revision, and
     `state_commit`.
   - Failure scenario: sequence-preflight rejection returns only an error and request counts; handler rejection returns partial R6
     fields. Neither supplies the complete L2 recovery tuple.
   - Trigger: all six TaskSpace runs; 11 missing-next-call rejections, five invalid arguments, and two state-machine rejections.
   - Impact: L2 is active before a compatible feedback contract exists. Recovery requires extra guess-and-retry requests, so L2
     implementation completeness and effectiveness are not established.
   - Proof needed: amend active L2 to reference only guaranteed R6 fields, or activate a common rejection envelope across preflight
     and handler paths, then test every rejection class.
   - Evidence: `third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v2.md:12`,
     `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs:27`, and FLA-2 TaskSpace raw rollouts at
     `pair-001/right/artifacts/rollout.jsonl:50`, `pair-001/right/artifacts/rollout.jsonl:158`, and
     `pair-002/left/artifacts/rollout.jsonl:188` under the simple run root.

##### Non-blocking Risks

1. **Aggregation blind spots.** Reported tables contain seven handler failures but omit 11 preflight failures. All six
   `taskspace-control-usage.json` files also report zero state commits despite 25 committed controls. This can make request-path
   quality and observability gates appear healthier than raw rollouts show. Closing proof requires call-ID reconciliation across
   provider calls, preflight outputs, handler outputs, and Runtime commits.

2. **Stale high-priority bootstrap handle.** The same 405-byte system message continues to say `map_id: none` and
   `bootstrap_required: true` after initialization commits. This creates contradictory high-priority context and can induce duplicate
   initialization, although this smoke did not contain a second committed Map. Evidence appears at the initial simple TaskSpace
   `rollout.jsonl:8` and the unchanged message shape throughout its provider wire trace.

3. **Wire observability is Chat-specific.** Responses keeps base instructions in top-level `instructions`, while the observer scans
   only `input`. L1 identity and L2 position can therefore be unavailable or misindexed on `WireApi::Responses`. DeepSeek currently
   uses Chat, so this does not falsify the tested DeepSeek path.

4. **Two claims exceed the evidence.** The smoke cannot prove non-regression, and a run-local `warmup candidate` does not prove the
   zero-cache request was caused by the L1/L2 version switch. Both FLA-0 and FLA-2 simple runs contain one warmup candidate.

##### User-Perspective Checks

- Ordinary tools ran before initialization seven times across four of six runs, producing `no_task_path`.
- Every run produced at least one missing required sibling.
- Six maps required 13 initialization attempts but only six commits. These were pre-commit retries, not duplicate committed maps.
- There were five invalid arguments, two state rejections, two `read_map` calls, and no extra terminal request.
- All 12 sides passed public and hidden validators, but TaskSpace still cost +26.1% requests/+51.9% input on simple and
  +9.1%/+21.6% on complex versus same-round Standard.

##### Implementation Completeness Checks

| Plan Item | Status | Evidence / Gap |
|---|---|---|
| Production L1/L2 exact identity | landed | TaskSpace 65/65 exact; Standard 56/56 L2 absent |
| Candidate/Docker/validator traceability | landed | Binary attestation, side randomization, public and hidden exits reconcile |
| Complete DeepSeek carrier shape | partial | Live third system contradicts highest-authority two-system contract |
| L2 rejection recovery | partial | Active R6/preflight output does not provide the fields L2 tells the Agent to read |
| Complete policy/lifecycle coverage | partial | Only `map-request`; no resume/fork/compaction smoke; synthetic wire test omits third system |

##### Target Benefit Checks

| Claimed Benefit | Result | Status | Regression / Side Effect |
|---|---|---|---|
| Fixed-context reduction | 22,700 to 21,666 bytes, about -4.6% | proven | none detected in byte calculation |
| Standard isolation | 56/56 captured DeepSeek Chat requests omit L2 | proven for tested path | other wire APIs unverified |
| Single ownership | Exact L1/L2 source ownership verified | partial | full carrier ownership contradicted by third system handle |
| Lifecycle effectiveness | 25 observed FLA-2 failures vs 24 FLA-0; missing-next 11 vs 11 | not demonstrated | no adherence gain |
| Performance | request/input amplification decreased in both scenarios | weak evidence | stochastic; not attributable to L1/L2 |

##### Required Fixes

- Resolve the authoritative two-system/dynamic-carrier contradiction and make production, docs, manifest, and wire assertions agree.
- Align L2 rejection guidance with the active result algebra, or activate the common rejection envelope before retaining
  `active_verified`.
- Reconcile control attempts, all failure classes, and state commits by call ID; regenerate result tables and failure taxonomy.
- Downgrade FLA-2 from `active_verified` until the two blockers are corrected and rerun.
- Replace “proved correctness did not regress” and cache cold-start attribution with evidence-limited wording.

##### Missing Tests

- Exact end-to-end DeepSeek Chat snapshot asserting every message role, position, count, and dynamic carrier.
- Rejection-envelope tests for preflight, invalid arguments, stale revision, invalid transition, and state-machine rejection.
- Bootstrap behavior tests covering ordinary-first calls, missing siblings, retry initialization, and post-commit duplicate
  initialization.
- `map-always`, `map-append`, resume, fork, and compaction carrier-preservation tests.
- Artifact-aggregator reconciliation tests for calls, failures, commits, taxonomy, and projection identity.

##### Missing Logs / Observability

- `failure-taxonomy-summary.json` is empty despite 25 raw failure outputs.
- `state_commit_count` and `runtime_state_commit_count` are zero despite 25 committed controls.
- All 65 TaskSpace requests report `active_projection_missing` even though a Map handle is present.
- `taskspace_contract_manifest_identity` is inferred from the recognized base rather than observed on wire.

##### Evidence

- Candidate commit `2ea8b4d24`; candidate SHA256 matches `target/r7-five-layer/fla2/whale-candidate.build-attestation.json`.
- 121 FLA-2 provider requests reconciled: 65 TaskSpace Chat requests with three systems/21,666 bytes; 56 Standard Chat requests
  with two systems/21,534 bytes.
- Six FLA-2 TaskSpace rollouts reconciled: 45 controls, 25 commits, two successful reads, 18 rejected controls, and seven
  ordinary-tool gate failures.
- Historical FLA-0 rollouts reconciled: 11 missing-next, six no-path, three invalid-argument, and four uncoded state failures.
- Reviewer remained read-only and changed no repository file.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Ptolemy | B1: live third system contradicts authoritative two-system carrier | blocking | accept | Spec lines 55-66 forbid a third system; raw wire has system indices 0, 1, 2 in every captured TaskSpace request | Review status changed to blocked; no product remediation in this review-only turn | Reconcile carrier ownership, eliminate or correctly version the stale handle, add full-wire assertion, then fresh closure review |
| Ptolemy | B2: L2 requests rejection fields absent from active preflight/R6 outputs | blocking | accept | L2 line 12 names the tuple; `sequence_preflight.rs:29-43` emits only status/error/request counts | Review status changed to blocked; no product remediation in this review-only turn | Align L2 with guaranteed fields or land common result envelope, test every rejection branch, then fresh closure review |
| Ptolemy | N1: aggregate omits preflight failures and state commits | major | accept | Seven summarized handler failures do not reconcile with 18 rejected controls; usage artifacts show zero commits while runtime has 25 | Recorded as required observability repair | Add call-ID reconciler and regenerate FLA-2 evidence before re-acceptance |
| Ptolemy | N2: bootstrap handle remains factually stale after Map initialization | major; reinforces B1 | accept | `build_initial_context()` records one static handle; steady state appends only setting diffs, while raw system message remains `map_id:none` | Recorded as semantic-context defect, not Agent intelligence failure | Repair with B1; assert post-commit context contains no stale bootstrap fact |
| Ptolemy | N3: Responses API identity observation is incomplete | minor | defer | Whale's tested DeepSeek provider uses Chat; finding does not falsify current DeepSeek path | Scope limitation recorded | Cover or explicitly scope Responses before a provider-generic claim, no later than FLA-8 |
| Ptolemy | N4: non-regression and cache-cause wording exceeds smoke evidence | target-benefit warning | accept | Evaluation contract forbids smoke proof; `warmup candidate` is run-local classification, not causal attribution | Wording correction required in remediation | Replace with “no regression detected” and remove causal cache attribution |
| Ptolemy | RF1/RF4: reconcile carrier and withdraw `active_verified` | blocking | accept | Direct consequence of B1/B2 and authority conflict policy | This review report blocks proceeding; authority/result status not rewritten during review-only work | First remediation commit must downgrade status until closure passes |
| Ptolemy | RF2: align L2 and result algebra | blocking | accept | Direct consequence of B2 | None in review-only turn | Required before closure |
| Ptolemy | RF3: reconcile attempts/failures/commits | major | accept | Direct consequence of N1 | None in review-only turn | Required before regenerated benchmark report |
| Ptolemy | RF5: evidence-limited result wording | target-benefit warning | accept | Direct consequence of N4 | None in review-only turn | Correct with regenerated result document |
| Ptolemy | MT1: exact complete DeepSeek Chat wire snapshot | blocking coverage | accept | Existing test proves L2 uniqueness/position but cannot detect system index 2 | Gap recorded | Add before carrier closure review |
| Ptolemy | MT2: complete rejection-envelope tests | blocking coverage | accept | Existing tests do not prove the L2-named field tuple for all rejection classes | Gap recorded | Add before L2 closure review |
| Ptolemy | MT3: bootstrap misuse/retry/post-commit duplicate tests | major | accept | Raw runs show ordinary-first and missing-sibling retries; stale post-commit handle is untested | Gap recorded | Add during carrier remediation |
| Ptolemy | MT4: three policies and lifecycle carrier preservation | major | accept | L1/L2 are common to all policies, but real smoke covered only map-request | Gap recorded | Static three-policy wire checks in remediation; resume/fork/compaction coverage may land with FLA-7 |
| Ptolemy | MT5: artifact reconciliation tests | major | accept | N1 proves current summaries can disagree with raw evidence | Gap recorded | Add with observer repair |
| Ptolemy | ML1: empty failure taxonomy | major | accept | Raw failures exist while sample taxonomy is empty | Gap recorded | Populate from reconciled call lineage |
| Ptolemy | ML2: zero state-commit counters | major | accept | Runtime graph revisions and committed controls contradict zero counters | Gap recorded | Repair commit lineage accounting |
| Ptolemy | ML3: `active_projection_missing` while handle exists | minor | reject as literal failure; accept naming risk | `map-request` contract lines 149-156 specifies no direct projection emission; a handle is not an active projection | No correctness change required | Rename/classify as `expected_absent_for_map_request` to avoid misleading telemetry |
| Ptolemy | ML4: manifest identity inferred from base | minor | accept | Manifest body is intentionally not model-visible, so `count=1` is selected-contract metadata, not observed wire content | Gap recorded | Add provenance such as `identity_source=selected_base_contract` or rename the field |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links: n/a until remediation is implemented
- Blocking re-review launch records: n/a until remediation is implemented
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: authoritative carrier and active L2 feedback contract do not match production behavior
- Allowed to proceed: no

## Final Conclusion

The adversarial review completed successfully, but FLA-2 cannot retain `active_verified` and must not serve as the FLA-3 entry
baseline. Production remediation, targeted validation, regenerated evidence, and a new fresh closure review are required. This round
did not modify production code because the user requested review; it records the blockers without silently choosing a repair design.
