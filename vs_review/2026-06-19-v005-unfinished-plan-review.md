# Subagent VS Review: v0.0.5 Unfinished Engineering Plan

- Created: 2026-06-19 00:00:00 +08:00
- Updated: 2026-06-19 00:00:00 +08:00
- Task: 对 v0.0.5 未完成项工程方案执行对抗性审查
- Report path: `vs_review/2026-06-19-v005-unfinished-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: plan adversarial review

### Review Input

#### Objective
用户要求继续完善 v0.0.5，代码实际全部完成前禁止真实 E3 / 真实 agent 调用。v0.0.5 的目标是把 TaskSpace 成本控制从 artifact/report 推进到 active runtime path，同时不明显降低正确率。

#### Review Target
v0.0.5 未完成项工程方案、实验制度、release/start gate、runtime 成本控制设计。

#### Target Locations
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `third_party/codex-cli/codex-rs/core/src`

#### Change Introduction
当前方案把 v0.0.5 从误收口状态退回继续开发，要求补齐 provider request lifecycle、active budget、active context replacement、state_commit displacement、spawn/node budget、harness eligibility 和 release/start gate。正式 `terminal-bench_E3-P0_3_5` 只能在代码完成、非 agent gates、用户批准和 start gate 全部通过后执行。

#### Risk Focus
- 是否把观测、报告或 fixture success 错当实际成本控制。
- release/diagnostic/formal E3 的边界是否仍可能误导版本判断。
- 指标是否能被 early abort、跳过验证、no-spawn 伪造改善。
- provider lifecycle producer、ActionMap runtime、benchmark scripts 的事实源和职责是否清晰。
- active context replacement 是否真正作用于 provider-visible history composition。
- marker、attestation、sample identity 和 evidence level 是否仍可被复制、伪造或错样本复用。

#### Verification Status
- 本轮是方案审查，不运行真实 E3。
- 已知最近实现包括 release gate identity/cost/attestation hardening 和 provider budget state transition。
- 仍未确认完成的 P0 runtime 项包括 spawn/node hard budget、state_commit displacement、active context replacement 的 provider-visible proof。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Output in Chinese using: Summary; Blocking Findings; Non-blocking Risks; Required Fixes; Missing Tests; Missing Logs/Observability; Evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-logic-adversary / Socrates | 挑战方案是否真的达成 v0.0.5 产品目标，而非继续制造观测产物 | goal fit, release boundary, correctness/cost tradeoff |
| architecture-adversary / Goodall | 挑战 runtime/provider/script 职责边界和长期可实施性 | ownership, source of truth, active execution path |
| test-validity-adversary / Dirac | 挑战 fixture、marker、attestation、实验命名和 release gate 是否自欺 | experiment validity, anti-spoofing, start gate |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Socrates | `multi_agent_v1.spawn_agent` explorer | `019edf29-4452-7900-bc2a-47de76ad1d91` | spawn tool result | no | Round 1 Review Input plus product-logic risk focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| Goodall | `multi_agent_v1.spawn_agent` explorer | `019edf29-826e-7922-ab84-c1d9cd90a032` | spawn tool result | no | Round 1 Review Input plus architecture risk focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| Dirac | `multi_agent_v1.spawn_agent` explorer | `019edf29-c76e-7270-a737-a383b42658e5` | spawn tool result | no | Round 1 Review Input plus test-validity risk focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Socrates / product-logic-adversary

##### Summary
方案总体方向是对的：它已经把“观测不是成本控制”写成核心前提，并要求 provider lifecycle、active context replacement、budget response、quality impact、release/start gate 都进入 producer-owned evidence。当前不认为方案完全错配 v0.0.5 目标，但仍有 2 个阻塞级产品逻辑风险：一个是 walltime 目标没有进入 re-entry/diagnostic 门禁，另一个是现有报告聚合脚本仍可能把 `E3-candidate` 混进 “E3*” 口径，和文档的防误导目标冲突。

##### Blocking Findings
- 正式 E3 re-entry 漏掉 walltime，可能先跑无效实验。Release Target 明确要求 `agent walltime ratio <= 2.0x Standard`，但 Engineering Re-entry Target 和 Phase 6 targeted diagnostic 没有 walltime/latency gate。
- 实验报告聚合仍有 `E3*` 误分类风险，削弱 release/diagnostic/formal E3 边界。

##### Non-blocking Risks
- `TaskSpace solved >= Standard solved - 1` 必须禁止在 `_1_1` 或 `_3_1/_3_2` diagnostic 上复用。
- `BudgetQualityImpactV1` 只有在 release decision 对 `score_eligible=false`、`blocked_by_budget`、`validation_skip` 做硬 blocker 时才成立。
- active replacement 的真实价值仍取决于 session/history composition 边界是否真的改掉。
- Phase 5 harness gate 过晚发现样本 eligibility 问题时，前面 runtime work 缺少可靠外部目标反馈。

##### Required Fixes
- 在 Engineering Re-entry Target 和 Phase 6 targeted diagnostic 增加 walltime/latency 门禁。
- 把所有报告聚合里的 `E3*` 匹配改为显式等级集合：正式 E3 只能等于 `E3`；`E3-candidate` 单独分组。
- 增加 “E3-candidate 不得进入 E3 aggregate / release proof” 负例。
- 要求 diagnostic 输出 walltime breakdown。

##### Missing Tests
- `E3-candidate` 被 summary/aggregate 错归为 E3 的负例。
- diagnostic request/token 达标但 walltime 超标时，禁止进入 formal P0 的 start-gate 负例。
- `BudgetQualityImpactV1.score_eligible=false` 但 pair report solved=true 时，release-decision 必须 fail 的负例。
- active replacement exact scan 通过但 protected evidence miss 的 release fail 负例。

##### Missing Logs / Observability
- 缺少 re-entry 级 walltime phase attribution。
- 缺少 post-budget walltime 计数。
- 缺少 diagnostic-only 报告中 `not_release_proof` 的聚合层强制展示。
- 缺少 per-sample `budget_quality_impact_summary` 与 solved/score eligibility 的 join 表。

##### Evidence
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:72`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:83`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:1232`
- `docs/v0.0.5/17-unfinished-work-inventory.md:18`
- `docs/experiments/taskspace-evidence-levels-and-samples.md:25`
- `docs/experiments/taskspace-evidence-levels-and-samples.md:44`
- `scripts/taskspace-benchmark/lib/aggregate-report.ps1:103`
- `scripts/taskspace-benchmark/lib/aggregate-report.ps1:339`
- `scripts/taskspace-benchmark/lib/report-summary.ps1:29`

#### Goodall / architecture-adversary

##### Summary
v0.0.5 方案方向正确：它已经把问题从 report/projection artifact 拉回 provider request lifecycle、provider-visible history composition、runtime budget gate 和 release/start gate。阻塞风险在于：文档契约比当前代码/脚本边界更严格，但实现路径仍有多处会把“派生 artifact”误当成 canonical producer，或者把 active replacement 简化成字符串过滤。

##### Blocking Findings
- Provider lifecycle producer 仍有“脚本二次生产事实源”的架构风险。脚本侧 `New-TaskspaceProviderRequestArtifacts` 直接从 `BudgetEvents` 派生 `provider-request-events.jsonl`。
- Active context replacement 的代码落点过窄，容易破坏 correctness 或漏掉真实高成本历史。
- Exact payload scan 目前是启发式文本扫描，不足以作为 release-grade active replacement proof。
- Budget manager 有三套状态口径，存在双写/伪事实源风险。
- Release gate 读取的 budget quality summary 文件名与生成文件名不一致。

##### Non-blocking Risks
- `provider_request_id` 的生成与 session/turn 边界是否稳定仍需单测证明。
- thin/verification-first 默认 `spawn=0` 需要显式 escalation event。
- `state_commit` displacement 不能只看 legacy action 减少，还要看 follow-up requests 和 rejection retry pressure。

##### Required Fixes
- 让 `provider-request-events.jsonl` 由 provider lifecycle 或其直接记录器输出；脚本只能校验/汇总。
- 把 active replacement 提升为明确的 history composer。
- 把 exact scan 从 marker 检查升级为结构化 negative/positive checks。
- state_commit displacement 和 spawn/node budget artifact 必须来自 runtime counters/gate events。
- 修正 budget quality summary 文件命名，并加 fixture 防止路径漂移。

##### Missing Tests
- Provider lifecycle mock 覆盖 HTTP/WebSocket/retry/cancel/error/blocked。
- Active history composer fixture。
- Negative release fixtures：脚本派生 provider event、hash-only payload、projection-only proof、missing exact scan、missing runtime spawn gate。
- State commit displacement fixture：legacy action 减少但 follow-up request 增加时不得 pass。
- Spawn/node correctness fixture。

##### Missing Logs / Observability
- Provider request event 需要直接记录 `producer=provider_lifecycle`、transport、logical_request_id、attempt_seq、terminal_status、payload_hash、budget_before/after。
- Active replacement report 需要 omission breakdown。
- Runtime budget gate 需要记录每次阻断的 recovery path、quality impact、是否允许 final/validation recovery。
- Spawn/node budget 需要记录 blocked spawn reason、route mode、decision target、adoption/rejection/defer result。
- State commit displacement 需要记录 adoption rate、rejection rate、retry follow-up request count、legacy action class breakdown。

##### Evidence
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:15`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:189`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:237`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:919`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:452`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1322`
- `third_party/codex-cli/codex-rs/core/src/client.rs:745`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:287`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:931`
- `scripts/taskspace-benchmark/write-release-decision.ps1:171`

#### Dirac / test-validity-adversary

##### Summary
方案方向是正确的：文档明确禁止代码完成前真实 E3，并把 `terminal-bench_E3-P0_1_1/_3_1/_3_2` 降为 diagnostic-only。但当前 gate 仍存在阻塞级测试有效性缺口：多个关键证明仍可由同一 run 目录内的 JSON 自声明拼出来，release gate 更像“字段一致性检查”，还没有充分证明这些字段来自真实 runner、真实 provider payload、真实 3 samples x 5 repeats。

##### Blocking Findings
- marker / attestation 仍可被手工伪造，缺少不可伪造的外部锚点。
- release proof 没有把 15 个 pair 绑定到 3 个正式样本各 5 次。
- budget / provider gate 仍可被“少做验证 + 自声明 summary”绕过。
- one-pair smoke gate 把 classified invalid_harness 当作 pass，语义危险。
- exact payload scan 只校验 scan event 与 report 的 id/hash 一致，不证明 scan 发生在 provider path。

##### Non-blocking Risks
- `Get-TaskspaceE3SampleSetDerivation` 只按 sample 名称和 repeats 推导 formal P0，不验证 task_dir 固定源版本或任务内容 hash。
- 文档里 BudgetQualityImpactV1 字段定义重复两段，后一段较弱。
- release decision 仍保留 `projectionPass`、`mapPass`、`routingPass` 等 report-only gate，输出上可能制造认知噪声。

##### Required Fixes
- formal E3 pair evidence 增加强校验：每个 `pair_completed` 必须包含 `sample_id`、`sample_repeat_index`、`standard_run_id`、`taskspace_run_id`。
- marker/attestation 改为 producer-owned，至少绑定 git HEAD、dirty state、runner hash、parent suite receipt hash、不可回写 final receipt hash。
- provider events、request phase summary、active replacement report、exact payload scan 必须按 request_id join。
- `BudgetQualityImpactV1` 必须按 sample 和预算动作全覆盖，不能只信 summary 自声明。
- one-pair smoke 的 `classified_invalid_harness` 改为 blocked/diagnostic-pass，不能满足 `full_e3_allowed=true`。

##### Missing Tests
- 伪造 `suite-runner-attestation.json` 和 hash chain 但未由 runner 生成，release gate 应失败。
- 15 个 pair 全部来自同一个 sample，runStatus 仍声明 3 samples，release gate 应失败。
- provider events 只有 1 条但 summary 自称 99% coverage，release gate 应失败。
- budget action 有 hard_stop / validation_skip 但缺少对应 sample-level quality impact event，release gate 应失败。
- exact scan event 与 active replacement report 匹配，但 provider request event 中没有同 request_id/hash，release gate 应失败。

##### Missing Logs / Observability
- 缺少 append-only suite receipt 的外部最终锚点。
- 缺少 per-sample/per-repeat identity ledger。
- 缺少 provider lifecycle join report。
- 缺少 budget action coverage report。

##### Evidence
- `scripts/taskspace-benchmark/write-release-decision.ps1:278`
- `scripts/taskspace-benchmark/write-release-decision.ps1:344`
- `scripts/taskspace-benchmark/write-release-decision.ps1:407`
- `scripts/taskspace-benchmark/write-release-decision.ps1:520`
- `scripts/taskspace-benchmark/write-release-decision.ps1:573`
- `scripts/taskspace-benchmark/write-release-decision.ps1:614`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:176`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:135`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:343`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1:313`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:267`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:404`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Socrates | Re-entry/diagnostic 缺少 walltime gate | blocking | accept | v0.0.5 release target 包含 walltime，但工程 re-entry 未覆盖 | 记录为方案阻塞修正项 | 修订 `18-unfinished-work-engineering-design.md`，并补 start-gate / release-gate 负例 |
| Socrates | `E3*` 聚合会混淆 `E3-candidate` | blocking | accept | `E3-candidate` 不是 E3，脚本通配匹配有误导风险 | 记录为实现阻塞修正项 | 改显式 evidence level 枚举，补聚合负例 |
| Goodall | provider lifecycle artifact 由脚本派生，违反 canonical producer | blocking | accept | 方案要求 provider path 是事实源，脚本只能消费 | 记录为架构阻塞修正项 | runtime 输出 provider-owned events；release gate 拒绝脚本派生 provider event |
| Goodall | active context replacement 仍是字符串过滤 | blocking | accept | 当前落点不足以证明 provider-visible history 被正确重组 | 记录为架构阻塞修正项 | 设计并实现 structured history composer |
| Goodall | exact payload scan 语义太弱 | blocking | accept | marker/string scan 不能证明 prohibited history 与 protected evidence 完整性 | 记录为架构阻塞修正项 | 升级结构化 scan 字段和 request lifecycle join |
| Goodall | budget manager 多事实源 | blocking | accept | 脚本仍后验计算 displacement/spawn/node budget | 记录为架构阻塞修正项 | runtime counters/gate events 成为 producer；脚本只校验 |
| Goodall | budget quality summary 文件名不一致 | blocking | accept | 生成 hyphen 文件，release 读取 underscore 文件 | 记录为必须立即修复的工程缺陷 | 修正文件名并补 fixture |
| Dirac | marker/attestation 缺少外部锚点 | blocking | accept | 同目录 JSON 自洽仍不足以证明真实 runner | 记录为实验制度阻塞修正项 | 增加 suite ledger/final receipt anchor/git state/script hash 等校验 |
| Dirac | 15 pair 未绑定 3 samples x 5 repeats | blocking | accept | 当前结构检查不足以防同样本重复冒充正式 P0 | 记录为 release proof 阻塞修正项 | pair evidence ledger 绑定 sample/repeat/run ids |
| Dirac | budget/provider gate 可被少做验证绕过 | blocking | accept | summary 自声明不等于 per-sample/action coverage | 记录为质量保护阻塞修正项 | budget action coverage 从原始事件派生 |
| Dirac | invalid_harness smoke pass 语义危险 | blocking | accept | classified failure 不能证明 full E3 scoring path 可用 | 记录为 start gate 阻塞修正项 | classified_invalid_harness 只能 diagnostic-pass，不能解锁 full E3 |
| Dirac | exact payload scan 未 join provider event | blocking | accept | matching scan/report 仍可伪造 | 记录为 release proof 阻塞修正项 | release gate 强制 request_id/hash/stage/timestamp join |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - required after plan and implementation fixes
- Blocking re-review launch records:
  - required after plan and implementation fixes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Final Conclusion

Round 1 found accepted blocking findings. The v0.0.5 continuation plan cannot be treated as review-passed yet. The next action is to revise the plan and implementation backlog around walltime re-entry, evidence-level aggregation, provider-owned lifecycle events, structured active history composition, runtime-owned budget gates, pair/sample/repeat identity ledger, and stronger release/start gate anti-spoofing. Formal E3 remains forbidden.
