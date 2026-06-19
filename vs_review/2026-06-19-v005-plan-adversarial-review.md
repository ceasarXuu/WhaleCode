# Subagent VS Review: v0.0.5 Plan Adversarial Review

- Created: 2026-06-19T00:00:00+08:00
- Updated: 2026-06-19T00:00:00+08:00
- Task: 对 v0.0.5 继续开发方案执行对抗性审查，判断方案是否足以支撑成本控制目标和可信 E3/release 判断。
- Report path: `vs_review/2026-06-19-v005-plan-adversarial-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: Plan And Release Gate Review

### Review Input

#### Objective

审查 `docs/v0.0.5` 下 v0.0.5 未完成项工程方案和实验制度，确认它是否能真正完成 v0.0.5 成本控制目标，并避免再次把 diagnostic/internal matrix 误判为正式 E3。

#### Review Target

- v0.0.5 未完成项工程方案
- v0.0.5 实验制度、start gate、release decision gate
- formal E3 准入和 release proof 可信度

#### Target Locations

- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `docs/v0.0.5/08-observability-and-budget-metrics.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`

#### Change Introduction

当前方案把 v0.0.5 从“收口候选”改回“继续开发”，要求成本控制进入 provider/model request、provider-visible context replacement、runtime budget、state_commit displacement、spawn/node budget 和 release/start gates。最新代码还加入了 provider request lifecycle id、request coverage denominator、payload replacement scan、start marker hardening、suite receipt provenance 等门禁增强。

#### Risk Focus

- 方案是否仍把 report-only、shadow-only、warn-only 模块当作 v0.0.5 产品目标完成。
- formal P0 release proof 是否包含 request/token/time/accuracy 的完整成本目标，而不是只看 token 和 walltime。
- budget 阈值是否从 Standard baseline 或 scenario expected budget 派生，还是只做绝对止血阈值。
- diagnostic-only 是否仍可能被当成 formal E3/release proof。
- `terminal-bench_E3-P0_3_5` 是否由 task list 内容、sample 数和 repeats 强校验。
- `release_pass` 是否仍可由 synthetic/copy/fake JSON tree 触发。

#### Verification Status

- 本轮未运行真实 E3，未调用真实 agent benchmark。
- 本轮是方案/门禁对抗性审查。
- 本地复核了 `docs/v0.0.5/18-unfinished-work-engineering-design.md:48-65`、`scripts/taskspace-benchmark/write-release-decision.ps1:380-430`、`scripts/taskspace-benchmark/test-release-decision.ps1:358-374`。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Hubble | 审查成本控制方案是否进入真实执行路径 | provider/runtime budget、context replacement、state_commit、spawn/node budget |
| Hegel | 审查实验制度和 release/start gate 是否仍会误导发布判断 | diagnostic-only、formal E3 identity、synthetic/copy artifact provenance |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Hubble | `multi_agent_v1.spawn_agent` explorer | `019edeff-b859-7fb3-bc1f-b18054373498` | spawn_agent result in current Codex thread | no | Round 1 cost-control review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| Hegel | `multi_agent_v1.spawn_agent` explorer | `019edeff-f03a-7a40-bd40-921b16a4b21b` | spawn_agent result in current Codex thread | no | Round 1 experiment/release-gate review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Hubble

##### Summary

方案方向基本正确：它没有停留在观测层，已经把成本控制明确推到 provider dispatch、provider-visible context composition、runtime budget、state_commit displacement、spawn/node budget、release/start gates 这些真实执行路径上。整体能显著降低“只做 report/shadow 就宣称完成”的风险。

但还不能直接判定“足以完成 v0.0.5 成本控制目标”。主要缺口是：formal P0 release proof 没有把 request count 接近 2x 作为正式验收门槛，budget 阈值仍偏工程保护阈值而非从 Standard baseline 派生，且 provider hook 精确落点仍被列为开放问题。

##### Blocking Findings

- Formal P0 验收缺少 request count 近 2x 硬门槛。`18-unfinished-work-engineering-design.md` 的 release target 只有 solved、direct input+output、walltime，request count 只在 re-entry/diagnostic 阶段用 `<=2.5x`。
- Provider hook 精确位置仍是开放问题。文档要求 provider lifecycle 是 canonical producer，并要求在 `client.rs` / `ModelClientSession` dispatch 前阻断，但后文仍把 hook 精确位置列为开放问题。
- Budget 默认阈值不是从 Standard baseline/2x 目标派生。固定请求、spawn、node 上限能防止百倍膨胀，但不能证明接近 Standard 的 2x。

##### Non-blocking Risks

- `warn-only` / shadow fallback 仍可能被误用为“可继续推进”，必须确保只能产生 diagnostic/blocked_partial，不能 clean pass。
- Correctness gate 粒度偏粗，建议补充 per-sample regression、budget-induced unsolved、validator skip 分类。
- `state_commit_adoption_rate >= 80%` 可能被 runtime-synthesized commit 稀释，必须区分真正 model-visible displacement。

##### Required Fixes

- formal P0 release proof 增加正式样本集 request count gate：`<=2.0x` 为 clean pass，`<=2.5x` 至多为 blocked_partial。
- route budget 阈值绑定 Standard baseline 或 scenario expected budget；固定上限只能作为保护阈值。
- Phase 0A 先提交精确 hook 落点清单，覆盖 HTTP、WebSocket、retry/fallback、cancellation、blocked request。
- 明确 `warn-only`、shadow profile、manual override、hash-only payload proof 不能进入 `release_pass`。

##### Missing Tests

- targeted diagnostic pass 但 formal P0 request ratio fail 的 release gate fixture。
- Standard=1 request、TaskSpace=10 request 的 baseline-derived budget fixture。
- provider hook negative fixture：trace 有记录但 provider dispatch 未被阻断时 gate 必须 fail。
- fallback misuse fixture：warn-only、shadow-only、manual override solved 都不能 clean pass。

##### Missing Logs / Observability

- formal P0 request-count regression output。
- baseline-derived budget decision output。
- provider hook coverage must prove before-dispatch blocking, not only trace visibility。

##### Evidence

- `docs/v0.0.5/18-unfinished-work-engineering-design.md:48-65` - release target and re-entry target；request count 不在 formal release target。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:135-148` - before-dispatch provider hook and canonical lifecycle producer。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:161-189` - active budget fields and fixed route thresholds。
- `docs/v0.0.5/17-unfinished-work-inventory.md:45-65` - request explosion and `model_request_count_ratio <= 2.5x` before formal E3。
- `docs/v0.0.5/08-observability-and-budget-metrics.md:80-94` - primary 2x token/walltime and auxiliary request/input gates。

#### Hegel

##### Summary

v0.0.5 policy is directionally strong: diagnostic-only is explicitly barred from release proof, `terminal-bench_E3-P0_3_5` is defined as exactly 3 named P0 samples x 5 repeats, and the suite runner invokes the start gate before sample scheduling.

Blocking issue: release decision gate can still be satisfied by an internally consistent synthetic artifact tree. The current self-test constructs fake artifacts and expects `release_pass`, so JSON shape/provenance claims can still substitute for runner-owned execution proof.

##### Blocking Findings

- `release_pass` can be produced from synthetic/copy JSON artifacts. `test-release-decision.ps1` builds a complete fake run tree in `New-FixtureRun`, writes `artifact_origin = "real_suite"` and matching manifest/receipt/status hashes itself, and asserts this fixture produces `release_pass`.

##### Non-blocking Risks

- Release decision does not recompute formal P0 identity from the task list file. Start gate derives from task-list content, but release decision mainly trusts `run-status.json`, `suite-manifest.json`, and receipt fields.
- Diagnostic-only appears separated in policy and normal runner path, but release tests should include a direct diagnostic-only artifact fixture.

##### Required Fixes

- Add runner-owned receipt authenticity that cannot be self-authored by arbitrary run artifacts; decision script must verify receipt chain against suite-created marker, command line, runner script hash, task list hash, current git commit, and a start-gate artifact produced in the same suite root.
- Change release-decision self-test so synthetic fixtures can test individual gates but must not assert final `release_pass`; real pass fixture should come only from controlled suite runner invocation or a clearly non-production fixture mode.
- In `write-release-decision.ps1`, reread `TaskListPath` or require verifiable task-list file path and recompute `Get-TaskspaceE3SampleSetDerivation`; compare it to `terminal-bench_E3-P0_3_5`, sample names, repeats, `task_list_sha256`, and `task_list_hash`。

##### Missing Tests

- diagnostic-only run with `terminal-bench_E3-P0_3_2`, `reported_evidence_level=diagnostic-only`, and plausible metrics must fail with `formal_e3_identity_gate_failed`。
- fake/copy tree with internally consistent hashes and `artifact_origin=real_suite` must fail。
- `run-status.sample_names` claims P0 set but task list content contains different samples must fail。

##### Missing Logs / Observability

- Release decision should emit whether formal sample identity was derived from task-list content or trusted from run artifacts。

##### Evidence

- `docs/experiments/README.md:13` - rules forbid non-E3 evidence being called E3。
- `docs/experiments/taskspace-evidence-levels-and-samples.md:72` - formal P0 sample set id。
- `docs/experiments/taskspace-evidence-levels-and-samples.md:82` - diagnostic-only variants and prohibited release usage。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:988` - Phase 6 separates diagnostic, formal P0 proof, and v0.0.4 comparison。
- `scripts/taskspace-benchmark/lib/e3-identity.ps1:98` - formal sample id derived from task list content。
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:321` - start gate validates derived sample set。
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:248` - start gate runs before scheduling。
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:280` - `full_e3_allowed=false` aborts before sample scheduling。
- `scripts/taskspace-benchmark/write-release-decision.ps1:419` - formal E3 identity gate。
- `scripts/taskspace-benchmark/test-release-decision.ps1:362` - synthetic fixture currently expected to pass release。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Hubble | Formal P0 release proof lacks request count ratio hard gate | blocking | accept | Local check confirms release target only lists solved, direct input+output, walltime in `docs/v0.0.5/18-unfinished-work-engineering-design.md:48-52`; request ratio appears only in re-entry target at lines 59-62. | None in this review turn. | Amend design and release gate so request ratio is clean-pass/blocking criterion. |
| Hubble | Provider hook exact location remains open | blocking | accept | Design says provider dispatch is canonical producer but open question still asks exact hook location. Without this, Phase 0A is not closed enough for implementation sequencing. | None in this review turn. | Update Phase 0A with concrete hook map or mark as current implementation evidence if already known. |
| Hubble | Budget thresholds are absolute protection, not baseline-derived 2x target | blocking | accept | Current fixed route budgets can stop runaway behavior but do not prove ratio to Standard. Product target is relative cost. | None in this review turn. | Add baseline-derived budget contract and tests. |
| Hubble | warn-only/shadow fallback may be misused | non-blocking | accept | Current docs mostly bar shadow from release, but fallback rows can be misread. | None in this review turn. | Clarify fallback taxonomy and release blockers. |
| Hegel | Synthetic/copy JSON tree can still produce `release_pass` | blocking | accept | Local check confirms `scripts/taskspace-benchmark/test-release-decision.ps1:358-363` creates `New-FixtureRun "pass"` and asserts `release_pass`; `write-release-decision.ps1:394-400` checks internally consistent provenance fields. | None in this review turn. | Implement runner-owned authenticity or make synthetic fixtures non-closeable; add negative copy-tree test. |
| Hegel | Release decision does not recompute formal P0 identity from task list file | non-blocking | accept | Start gate derives sample set from task list; release decision primarily checks run-status/manifest/receipt fields. | None in this review turn. | Add release-side task-list derivation and output `sample_identity_source=derived_from_task_list`. |
| Hegel | Missing diagnostic-only release negative fixture | non-blocking | accept | Existing policy should block it, but explicit regression fixture is missing. | None in this review turn. | Add direct diagnostic-only plausible artifact fixture. |

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

本轮对抗性审查阻塞通过。v0.0.5 方案方向正确，但必须先修正 formal P0 request ratio release gate、baseline-derived budget、provider hook 落点闭环、synthetic/copy artifact release pass 风险，以及 release-side task-list derivation。修复后需要新增一轮 fresh closure review。
