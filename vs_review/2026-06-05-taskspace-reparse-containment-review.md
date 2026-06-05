# Subagent VS Review: TaskSpace Reparse Containment

- Created: 2026-06-05T13:34:00+08:00
- Updated: 2026-06-05T13:47:00+08:00
- Report schema: adversarial-v1
- Task: Harden TaskSpace final-artifact audit containment for Windows reparse points.
- Report path: `vs_review/2026-06-05-taskspace-reparse-containment-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Implementation And Test Validity Review

### Review Input

#### Objective

Review the ArtifactRoot containment hardening for TaskSpace cognitive/final-artifact audit.

#### Review Target

Code implementation, Windows path semantics, regression integration, and documentation.

#### Target Locations

- `scripts/action-map-final-artifact-audit-lib.ps1`
- `scripts/test-action-map-reparse-containment.ps1`
- `scripts/run-action-map-regression.ps1`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/testing/2026-05-08-action-map-real-user-e2e.md`
- `target/test-reports/action-map-20260605-134247-920/report.md`
- `target/test-reports/action-map-20260605-134247-920/script-test-test-action-map-reparse-containment.ps1.stdout.log`

#### Change Introduction

`Resolve-FinalArtifactPath` now calls a reparse-aware resolver that walks path segments, follows junction/symlink targets, appends remaining path segments to the real target, and only then applies ArtifactRoot containment. A new Windows junction test creates a junction inside `ArtifactRoot` pointing outside the root and verifies the final-artifact audit refuses to hash or accept the escaped target. The test is now part of the default action-map regression script matrix.

#### Risk Focus

- Windows path behavior: drive roots, relative targets, junction target arrays, nested reparse points, recursion bounds.
- False positives: the new test might pass without proving the resolver rejects the escaped physical target.
- False negatives: legitimate in-root artifacts, normal absolute paths, and relative paths should still resolve.
- Security boundary: escaped reparse target must not receive a hash or an accepted resolved path.
- Regression wrapper should include the new test without making the script exceed project file-size constraints.

#### Verification Status

- `git diff --check`: PASS.
- `scripts/test-action-map-reparse-containment.ps1`: PASS.
- `scripts/run-action-map-regression.ps1`: PASS.
- Latest regression report: `target/test-reports/action-map-20260605-134247-920/report.md`, `total_passed_tests=218`, `total_failed_tests=0`, `script_run_count=4`, `skipped_script_runs=0`, `relevant_crash_events=0`.

#### Reviewer Instructions

- Fresh internal subagent session.
- Do not inherit main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return Summary, Blocking Findings, Non-blocking Risks, Required Fixes, Missing Tests, Missing Logs / Observability, Evidence, and Verdict.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| windows-security-adversary | The change is a Windows containment/security boundary around filesystem reparse points. | path semantics, containment bypass, regression validity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| windows-security-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9645-f0e8-7031-8f44-bd6b38f8ead3` (`Laplace`) | tool result in main thread | `fork_context=false` | Round 1 Review Input plus Windows containment risk focus | main-agent history, reasoning, drafts, conclusions, hidden context | yes |

### Reviewer Outputs

#### windows-security-adversary

Summary:

The reviewer confirmed the initial junction escape test was meaningful, but found one blocking fail-open edge: when `ArtifactRoot` itself fails reparse-aware resolution, `Resolve-FinalArtifactPath` left `$rootFull` empty and therefore skipped containment before hashing absolute artifact refs.

Blocking findings:

- `Resolve-FinalArtifactPath` did not fail closed when `$rootFull = Resolve-ReparseAwarePath $ArtifactRoot` returned empty. If root resolution failed because of cycle/depth/empty target, an absolute artifact ref could bypass containment and be hashed.

Non-blocking risks:

- The first junction test covered only a single-layer junction escape.
- Non-Windows skip needed to be distinguishable from real Windows coverage.
- Wrapper was close to the 500-line limit.

Required fixes:

- Fail closed immediately if non-empty `ArtifactRoot` cannot be resolved.
- Add a negative test for unresolved reparse root.
- Distinguish Windows-only skip from PASS in wrapper/report.

### Main Agent Response

| Finding | Decision | Fix |
|---|---|---|
| Empty `$rootFull` skips containment | accept | Added `if ([string]::IsNullOrWhiteSpace($rootFull)) { return "" }` immediately after resolving `ArtifactRoot`. |
| Missing root-unresolved negative test | accept | Extended `scripts/test-action-map-reparse-containment.ps1` with an 18-level junction chain that triggers depth fail-closed, then verifies an absolute outside artifact is not accepted or hashed. |
| Non-Windows skip indistinguishable from PASS | accept | Reparse test now emits `Overall: SKIP` on non-Windows; wrapper treats SKIP as non-failing and reports `skipped_script_runs`. |
| Wrapper line budget risk | accept | Kept wrapper at 497 lines after the skip handling change. |

Final verification after fixes:

- `git diff --check`: PASS.
- `scripts/test-action-map-reparse-containment.ps1`: PASS, outputs `cognitive-audit-artifact-root-reparse-containment: PASS` and `cognitive-audit-artifact-root-unresolved-fail-closed: PASS`.
- `scripts/run-action-map-regression.ps1`: PASS, final report `target/test-reports/action-map-20260605-134247-920/report.md`.
- Final report: `total_passed_tests=218`, `total_failed_tests=0`, `script_run_count=4`, `skipped_script_runs=0`, `relevant_crash_events=0`.
- Additional full regression: `scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E`: PASS, report `target/test-reports/action-map-20260605-134926-545/report.md`, `script_run_count=5`, `skipped_script_runs=0`, viewer `browser_interaction_ok=true`.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Rejected findings backed by evidence: none
- Deferred findings documented: missing broader reparse variants remain non-blocking future coverage
- Allowed to proceed: pending

## Round 2: Blocking Closure Review

### Review Input

Fresh read-only closure review after accepting and fixing the Round 1 blocking fail-open finding.

Target files and artifacts:

- `scripts/action-map-final-artifact-audit-lib.ps1`
- `scripts/test-action-map-reparse-containment.ps1`
- `scripts/run-action-map-regression.ps1`
- `target/test-reports/action-map-20260605-134247-920/report.md`
- `target/test-reports/action-map-20260605-134247-920/script-test-test-action-map-reparse-containment.ps1.stdout.log`

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Read-only |
|---|---|---|---|---|---|
| closure-reviewer | `multi_agent_v1.spawn_agent` explorer | `019e9650-9fea-7fc3-a79f-5c3c36f55dc4` (`Rawls`) | tool result in main thread | `fork_context=false` | yes |

### Reviewer Output

Summary:

第一轮 blocking 已关闭。`ArtifactRoot` 非空但 reparse-aware 解析失败时，现在会立即 fail-closed；测试覆盖了 unresolved/deep reparse root + absolute outside artifact 的旧 bypass 形态；wrapper 能区分 `SKIP` 和 `PASS`，且本次 Windows 报告 `skipped_script_runs=0`。

Blocking findings:

- None.

Non-blocking risks:

- 负测覆盖的是深层 junction 触发的 unresolved root；cycle、空 target、多 target、symlink 变体仍属于后续加固覆盖面，但不影响这次 blocking closure。
- wrapper 将 `SKIP` 作为非失败处理；当前 Windows 报告明确为 0 skipped，所以不是本轮阻塞。

Evidence:

- `scripts/action-map-final-artifact-audit-lib.ps1`: `$rootFull = Resolve-ReparseAwarePath $ArtifactRoot` 后，在空值时直接 `return ""`。
- `scripts/test-action-map-reparse-containment.ps1`: 创建 absolute outside artifact、19 层 junction，以 `deep-link-0` 作为 `ArtifactRoot`，断言 hard gate fail、missing hash、`artifactFound=false`、`artifactHash=""`。
- `scripts/run-action-map-regression.ps1`: 默认 script matrix 包含 reparse containment test，区分 `Overall: PASS` 和 `Overall: SKIP`，并输出 `skipped_script_runs`。
- `target/test-reports/action-map-20260605-134247-920/report.md`: `overall=PASS`、`script_run_count=4`、`skipped_script_runs=0`，reparse containment script 为 `PASS`。
- `target/test-reports/action-map-20260605-134247-920/script-test-test-action-map-reparse-containment.ps1.stdout.log`: 两个 reparse containment case 均 PASS，`Overall: PASS`。

Verdict:

- Allowed to proceed: yes.

## Final Conclusion

Round 2 passed. The TaskSpace final-artifact audit reparse containment hardening is allowed to proceed.
