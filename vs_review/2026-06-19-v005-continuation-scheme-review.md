# Subagent VS Review: v0.0.5 continuation scheme

- Created: 2026-06-19 18:22:58 +08:00
- Updated: 2026-06-19 18:22:58 +08:00
- Task: 对 v0.0.5 未完成项继续开发方案执行对抗性审查，确认方案是否能防止错误 E3、伪收口和成本控制假落地。
- Report path: `vs_review/2026-06-19-v005-continuation-scheme-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: scheme adversarial review

### Review Input

#### Objective

检查 v0.0.5 继续开发方案是否真实覆盖用户目标：v0.0.5 不能关闭，必须继续实现实际成本控制；代码完成前禁止真实 E3；后续只有在 non-agent gates、code-complete、user approval、start gate 都通过后才能运行 formal `terminal-bench_E3-P0_3_5`。

#### Review Target

方案、实验制度、release/start gate、runtime cost-control 架构边界。

#### Target Locations

- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/09-e3-validation-plan.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`

#### Change Introduction

当前方案把 v0.0.5 从“可收口候选”改回“继续开发”，要求把成本治理从 report-only 推到 active runtime path，并建立 formal E3 的严格准入制度。当前工作树还有未提交 runtime/cost-instrumentation 改动，本轮只审查方案和明显设计风险，不把未验证代码视为完成。

#### Risk Focus

- 是否仍可能把 diagnostic-only 或内部 fixture 误当 formal E3/release proof。
- 是否把“可观测”伪装成“实际成本控制”。
- provider request budget 是否被错误放在 ActionMap 事后推断，而非 provider lifecycle。
- active provider-visible history replacement 是否有结构化边界。
- runtime budget 是否只是 trace/report，而不是真正阻断 spawn/node/request。
- state_commit displacement 是否有 runtime-owned denominator，而非脚本推断。
- release/start gate 是否能拒绝 spoofed marker、伪造 attestation、样本对数不足和 runner 未调度前中止。

#### Verification Status

- No real E3 or real agent benchmark was run in this review.
- Reviewers were read-only.
- Current local worktree contained uncommitted runtime/cost-instrumentation changes.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-logic-adversary | 判断方案是否仍能满足 v0.0.5 产品目标和 E3 禁令 | 产品目标闭环、误收口 |
| architecture-adversary | 检查 runtime/provider/session/gate 边界是否能承载实际成本控制 | 架构边界、active control |
| test-validity-adversary | 检查实验制度和 release/start gate 是否能防止伪证据 | 测试有效性、证据防伪 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| product-logic-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf65-2acf-7541-a013-13d504ca719b` | spawn/wait/close tool transcript | no | Round 1 product logic packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf65-67d6-7d90-a42b-e0475b8f5215` | spawn/wait/close tool transcript | no | Round 1 architecture packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf65-a76d-7c93-b9a4-6c883551249f` | spawn/wait/close tool transcript | no | Round 1 test validity packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### product-logic-adversary

##### Summary

方案总体覆盖了目标闭环，尤其是“v0.0.5 不能关闭”“代码完成前禁止真实 E3”“formal `terminal-bench_E3-P0_3_5` 必须经过 non-agent gates/code-complete/user approval/start gate”。

##### Blocking Findings

- None.

##### Non-blocking Risks

- `e3-start-gate.ps1` 可能出现顶层 `status=pass`、`run_validity=valid`，同时 `full_e3_allowed=false` 的混合状态，容易被人读报告时误判。
- `docs/v0.0.5/09-e3-validation-plan.md` 虽有历史警告，但仍保留旧样本范围和旧运行矩阵，深链引用时仍可能误导。

##### Required Fixes

- start gate 顶层状态应支持 `blocked_for_full_e3` 或等价不可误读状态。
- 历史 E3 文档旧样本矩阵前应增加硬提示：仅历史保留，不得作为当前 E3 样本或 release proof 依据。

##### Missing Tests

- 已在方案列出：缺 v0.0.5 gate 阻断 full E3、marker spoof/stale/hash mismatch、`full_e3_allowed=false` 不得 schedule samples、formal pair ledger 防止 15 pairs 来自单一样本。

##### Missing Logs / Observability

- No new blocking gap reported.

##### Evidence

- `docs/v0.0.5/17-unfinished-work-inventory.md:10` - v0.0.5 明确不能关闭。
- `docs/v0.0.5/17-unfinished-work-inventory.md:43` - 成本控制必须是执行期预算，不是事后报告。
- `docs/v0.0.5/17-unfinished-work-inventory.md:291` - 代码完成前禁止真实 E3。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:6` - formal E3 放行条件完整。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:31` - diagnostic-only 不得 release proof。
- `scripts/taskspace-benchmark/write-release-decision.ps1:768` - release decision 要求 formal identity、receipt、attestation、approval、code-complete。

#### architecture-adversary

##### Summary

方案方向基本正确，但当前方案和切分仍有几个会让“成本控制看似落地、实际可绕过或误判”的阻塞点。代码完成前不应跑真实 E3。

##### Blocking Findings

- Provider request attribution 仍可能从 ActionMap runtime 状态反推 node/phase，违背方案自己的边界。
- Active provider-visible history replacement 还缺结构化删除边界。
- `state_commit_displacement` 证据仍不足以证明 displacement。
- Spawn/node budget gate 把“正确阻断”当成失败。

##### Non-blocking Risks

- `client.rs` 已经有 provider lifecycle hook 和 hard stop 形态，但方案仍应把 `client.rs` 明确列入 implementation target，避免只改 ActionMap/脚本。
- release gate 对 active replacement 的 request_id/hash join 方向正确，但如果 replacement proof 只来自 tags 而不是 pre-redaction exact scan artifact，仍需 fixture 防伪。
- start gate 方向正确，风险主要在 suite runner 是否严格 honor gate before scheduling。

##### Required Fixes

- provider request context 必须显式传入；缺 node/phase 时写 missing reason 并 fail coverage，不得 fallback ready node。
- `turn.rs` prompt composition 需要 typed replacement layer：保留用户需求、当前必要证据、失败验证证据、protected items；删除 raw TaskSpace control replay、stale node、rejected subagent、large output。
- `state_commit_displacement` 改为 runtime-owned denominator：记录 legacy action attempts、blocked legacy actions、model-visible state_commit、runtime-synthesized state_commit、accepted/rejected commit，并计算 adoption/displacement rate。
- spawn/node budget artifact 拆成两类 gate：正常 run 要求未超预算；negative fixture 要求超预算时出现 blocked event 且 `budget_response_action_taken=true`。

##### Missing Tests

- Provider context missing fixture：不得从 ready node 反推 node_id。
- Prompt reconstruction fixture：旧 TaskSpace history 存在于 session history，但 active provider payload 不包含它，且 protected user/evidence item 不丢。
- State commit displacement negative fixture：只有一次 `state_commit`、没有 legacy denominator 时必须 fail。
- Spawn/node budget negative fixture：超过预算应 pass “enforcement worked”，不是因 blocked event 失败。
- Suite runner fixture：`full_e3_allowed=false` 时必须在 sample scheduling 前中止。

##### Missing Logs / Observability

- provider lifecycle event 需要明确 `provider_request_context_missing_reason`。
- active replacement 需要 exact scan artifact 指向同一 `request_id` / payload hash，并记录 protected item count/missing list。
- state_commit displacement 需要按 task/map/node 输出 denominator 和 rate，而不是单次 commit counter。
- spawn/node budget 需要区分 `within_budget_observed`、`over_budget_blocked`、`post_budget_spawn_count`、`post_budget_request_count`。

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1561` - `provider_request_budget_snapshot()` 缺 current node 时 fallback 到 ready node。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:154` - 方案禁止用 snapshot 反推 provider request phase/node。
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:443` - turn loop 的 provider input 组装边界需与 typed replacement 对齐。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:969` - active profile 必须省略 raw TaskSpace control history 等高成本历史。
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3703` - state commit displacement event 当前固定写单次 commit counters。
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:310` - 脚本只要求 stateCommitCount > 0 且 legacy <= budget。
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:355` - spawn/node summary 将 blocked events 汇总为失败条件。

#### test-validity-adversary

##### Summary

当前方案比旧制度强很多，已经明确禁止 code-complete 前真实 E3，并覆盖 dataset/subset/sample/repeats/runner/evidence level、marker freshness/hash、formal P0 3x5 样本账本、suite receipt hash chain、runner scheduling 前中止等关键风险。但 release-decision 仍有 producer proof 防伪缺口。

##### Blocking Findings

- release gate 对 `suite-runner-attestation.json` 的防伪仍停留在字段/字符串层，不能证明 artifact 真由 suite runner 产生。
- release-decision 对 `v005-non-agent-gates.json` 的二次校验弱于 start gate，缺少每个 gate 的 `task_list_hash` / `source_version` 绑定校验。

##### Non-blocking Risks

- `docs/v0.0.5/09-e3-validation-plan.md` 仍保留 v004-clean 三样本和旧 partial 口径，后续 fresh agent 仍可能引用旧正文。
- start gate 允许 `speed_claim_allowed=false` 但 `full_e3_allowed=true`，报告层必须确保 speed/cost claim 不被顺手写入 release claim。

##### Required Fixes

- 给 `suite-runner-attestation.json` 增加 runner receipt 绑定：由 runner 在进程启动时写入 nonce，后续 `suite-receipt.jsonl`、`run-status.json`、attestation 共同引用，并要求 event hash chain 包含 attestation nonce、runner pid、command argv、script hash。
- release-decision 必须像 start gate 一样逐项校验 `v005-non-agent-gates.json` 的 `task_list_hash`、`source_version`、`profile_hash`。
- release-decision 也应校验 `v005-code-complete.json` 的 schema_version、generated_at freshness、git_commit、test_outputs、unfinished_p0_items。

##### Missing Tests

- 伪造但字段看似真实的 suite-runner-attestation 负例必须失败。
- non-agent gates 中任一 gate 的 `task_list_hash` 或 `source_version` 与 run-status 不一致，release-decision 必须失败。
- code-complete marker 缺 `test_outputs`、旧 `generated_at`、错误 `git_commit`、非 schema v1，release-decision 必须失败。
- 保留现有 runner 前中止测试。

##### Missing Logs / Observability

- runner-attestation 生成事件需要进入 `suite-receipt.jsonl` hash chain，而不是只作为最终 JSON 文件存在。
- release-decision summary 应输出 attestation failure 的具体子原因。

##### Evidence

- `scripts/taskspace-benchmark/write-release-decision.ps1:532` - attestation 校验仍主要依赖字段和字符串。
- `scripts/taskspace-benchmark/test-release-decision.ps1:437` - 现有负例只证明带 fixture 字符串的 synthetic attestation 会失败。
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:223` - start gate 逐项校验 task/source/profile/evidence hash。
- `scripts/taskspace-benchmark/write-release-decision.ps1:705` - release gate 对 non-agent gates 的二次校验弱于 start gate。
- `scripts/taskspace-benchmark/write-release-decision.ps1:281` - formal pair ledger 强制 sample_id、repeat、run id、E3 evidence level。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-logic-adversary | start gate 顶层 pass 但 full_e3_allowed=false 容易误读 | major | accept | 混合状态不会放行 formal E3，但会误导人工阅读 | 记录为方案修正项 | 后续把聚合状态改为 blocked_for_full_e3 或等价状态，并加测试 |
| product-logic-adversary | `09-e3-validation-plan.md` 旧正文仍可能误导 | major | accept | 历史警告存在，但深链引用仍可绕过新制度入口 | 记录为文档修正项 | 后续压缩旧正文或在旧样本章节前增加硬阻断提示 |
| architecture-adversary | provider request attribution 可能 fallback ready node | blocking | accept | 方案禁止 snapshot 推断；代码路径仍可能产生误归因 | 本轮只记录，未修复 | 必须改成显式 request context，缺失时写 missing reason 并 fail coverage；修后 fresh closure review |
| architecture-adversary | active replacement 缺 typed deletion boundary | blocking | accept | 方案要求 active provider-visible replacement；实现/方案需保证 protected/removable 分类可验证 | 本轮只记录，未修复 | 补 typed composer contract、payload reconstruction fixture；修后 fresh closure review |
| architecture-adversary | state_commit displacement 只证明 commit 发生 | blocking | accept | 当前 event/cost summary 不含 denominator，无法证明 legacy displacement | 本轮只记录，未修复 | 改为 runtime-owned denominator 和 adoption/displacement rate；修后 fresh closure review |
| architecture-adversary | spawn/node budget 把正确阻断当失败 | blocking | accept | 正确 over-budget block 应能作为 negative fixture 的 pass evidence | 本轮只记录，未修复 | 拆分 within-budget run gate 与 over-budget enforcement gate；修后 fresh closure review |
| test-validity-adversary | suite-runner-attestation 可被自洽 JSON 伪造 | blocking | accept | 当前 release gate 的 producer proof 仍偏字段/字符串校验 | 本轮只记录，未修复 | 加 runner nonce/hash-chain receipt 绑定和伪造负例；修后 fresh closure review |
| test-validity-adversary | release-decision 对 non-agent gates 二次校验弱于 start gate | blocking | accept | 可能复用同 profile 但不同 task/source evidence | 本轮只记录，未修复 | release-decision 逐项校验 task_list_hash/source_version/profile_hash；修后 fresh closure review |
| test-validity-adversary | code-complete marker 校验不够强 | major | accept | release-decision 需要校验 schema/freshness/git/test_outputs/unfinished_p0_items | 本轮只记录，未修复 | 加 marker schema/freshness/git/test output 负例 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Final Conclusion

本轮方案对抗性审查结论为 `blocked`。方案方向正确，但不能按当前方案/实现口径进入验证收口，也不能运行真实 E3。下一步必须先修正 provider request attribution、active replacement typed boundary、state_commit displacement denominator、spawn/node enforcement gate、runner attestation 防伪、release-decision marker 二次校验，然后对这些 accepted blocking fixes 再做 fresh closure review。
