# Subagent VS Review: TaskSpace Viewer E2E

- Created: 2026-06-05T12:37:43.2796680+08:00
- Updated: 2026-06-05T13:22:00+08:00
- Report schema: adversarial-v1
- Task: Execute the TaskSpace cognitive-state engineering plan by closing the `/task-show` viewer interaction validation gap.
- Report path: `vs_review/2026-06-05-taskspace-viewer-e2e-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Implementation And Test Validity Review

### Review Input

#### Objective
Review the TaskSpace `/task-show` viewer E2E upgrade and regression integration for implementation risk, false positives, and evidence gaps.

#### Review Target
Code implementation, test strategy, documentation, and validation report.

#### Target Locations
- `scripts/run-tui-taskspace-viewer-e2e.ps1`
- `scripts/run-action-map-regression.ps1`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/testing/2026-05-08-action-map-real-user-e2e.md`
- `target/test-reports/action-map-20260605-123331-226/report.md`
- `target/test-reports/action-map-20260605-123331-226/run-tui-taskspace-viewer-e2e.ps1/artifacts/report.md`

#### Change Introduction
`run-tui-taskspace-viewer-e2e.ps1` now supports `-OutputDir`, installs/loads `playwright-core` under `target/pty-tools`, launches installed `whale.exe`, opens `/task-show`, and uses local Chrome/Edge headless for real DOM interactions. The browser probe validates expanded details state, selected meta text preservation, and graph pan/zoom transform across auto-refresh. `run-action-map-regression.ps1` gained `-IncludeTuiViewerE2E` so this long real viewer path can be included in the unified action-map regression report without making it default.

#### Risk Focus
- Windows PowerShell and Node path handling, module resolution, browser executable assumptions.
- Async races or failure paths that could falsely pass or falsely fail.
- Regression wrapper compatibility with existing script tests and `-OutputDir`.
- Whether the browser test proves real previous UI risks or just implementation strings.
- Whether the real viewer path can pass with stale or low-value TaskSpace state.
- Whether documentation overclaims evidence beyond what the E2E proves.

#### Assumptions To Attack
- `require(<absolute playwright-core dir>)` works from generated artifact JS on Windows.
- A local Chrome/Edge executable is always an acceptable dependency for this optional long test.
- Auto-refresh is genuinely exercised, not bypassed by a static page state.
- DOM state preservation assertions are strong enough to catch the previous user-visible failure modes.
- Making viewer E2E opt-in from the regression wrapper does not create a misleading default evidence story.

#### Adversarial Lenses
- implementation
- testing
- observability
- maintenance
- release-ops

#### Verification Status
- `git diff --check` passed.
- `scripts/run-tui-taskspace-viewer-e2e.ps1 -TimeoutSeconds 180` passed with `browser_interaction_ok=true`.
- `scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E` passed.
- Unified report: `target/test-reports/action-map-20260605-123331-226/report.md`, `total_passed_tests=218`, `total_failed_tests=0`, `script_run_count=4`.
- Viewer sub-report: `target/test-reports/action-map-20260605-123331-226/run-tui-taskspace-viewer-e2e.ps1/artifacts/report.md`, `browser_interaction_ok=true`, `detail_state_ok=true`, `selection_state_ok=true`, `graph_transform_ok=true`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return Summary, Blocking Findings, Non-blocking Risks, Required Fixes, Missing Tests, Missing Logs / Observability, and Evidence.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | The patch touches PowerShell orchestration, generated JS, browser launch, and regression wrapper behavior. | implementation correctness, environment compatibility, maintainability |
| test-validity-adversary | The work is primarily a test/evidence upgrade, so false positives and overclaiming are the main risks. | test validity, evidence quality, observability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9611-caef-78a3-871f-da58522b7c03` (`Kierkegaard`) | tool result in main thread | `fork_context=false` | Round 1 Review Input plus implementation-specific risk focus | main-agent history, reasoning, drafts, conclusions, hidden context | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9612-271a-75e3-8ba2-e74245512f2b` (`Faraday`) | tool result in main thread | `fork_context=false` | Round 1 Review Input plus test-validity-specific risk focus | main-agent history, reasoning, drafts, conclusions, hidden context | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| implementation-round-1 | implementation-adversary | 1 | `019e9611-caef-78a3-871f-da58522b7c03` | completed | completed | reviewer returned blocking implementation/test-orchestration findings | fixes accepted |
| test-validity-round-1 | test-validity-adversary | 1 | `019e9612-271a-75e3-8ba2-e74245512f2b` | completed | completed | reviewer returned blocking evidence-validity findings | fixes accepted |

### Reviewer Outputs

#### implementation-adversary

Blocking findings:

- The completion marker check could be satisfied by prompt echo because the full marker token was present in the user prompt.
- Browser interaction did not prove refresh really happened while state was open or selected.
- Command flow had a timing race around `/task-reborn` and task creation.
- The regression wrapper calculated the crash-event window before script tests finished.
- The regression report did not parse and lift viewer metrics, so the top-level PASS hid the key evidence.

Non-blocking risks:

- Browser probing had limited retry behavior.
- The first version could pass on transform change without separating zoom from pan.

#### test-validity-adversary

Blocking findings:

- Auto-refresh was not truly verified; the test only relied on page script shape and elapsed wait.
- Meaningful live viewer state growth was not hard asserted.
- Pan/zoom were collapsed into one transform assertion, allowing pan regressions to hide behind zoom changes.
- Details and selection assertions did not prove a refresh occurred during the open/selected state.
- Viewer metrics were not available in the wrapper report, so reviewers had to inspect nested artifacts manually.

Required fixes:

- Count real `/snapshot.json` browser responses and record status/timestamp/hash.
- Save raw snapshots and a browser summary artifact.
- Separate zoom and pan assertions.
- Prove refresh after details open, after graph interaction, and during text selection.
- Lift viewer metrics into the unified regression report.

### Main Agent Response

| Finding | Decision | Fix |
|---|---|---|
| Marker prompt echo can satisfy completion | accepted | Prompt now asks the model to assemble `TASKSPACE_VIEWER_OK + "_" + run_id`; `user_prompt_contains_marker=false` is reported and `assistantMarkerObserved()` rejects prompt-contained full markers. |
| Refresh not proven | accepted | Browser response listener records every `/snapshot.json` response with status, timestamp, hash and snapshot stats; refresh-specific gates require response after details, graph, and selection actions. |
| Meaningful live state not proven | accepted | Browser probe waits for experiment snapshot with active map, at least one node, and at least one result before page interaction. Raw active snapshots and browser snapshots are saved. |
| Pan/zoom conflated | accepted | E2E now reports `graph_zoom_ok`, `graph_pan_ok`, and `graph_transform_ok` separately. |
| Details/selection refresh not proven | accepted | E2E now reports `refresh_during_detail_ok`, `refresh_during_graph_ok`, and `refresh_during_selection_ok`. |
| Wrapper hides viewer evidence | accepted | `run-action-map-regression.ps1 -IncludeTuiViewerE2E` now parses the nested viewer report and lifts refresh, graph, selection, snapshot and error-count fields into the main report. |
| Crash window ends before script tests | accepted | `$finished` is now captured after script tests, and crash matching includes `whale.exe`, `node.exe`, `chrome.exe`, and `msedge.exe`. |
| `/task-reborn` treated as task creation | accepted | The E2E now treats `/task-reborn` as reset/reborn only; the natural coding request is what triggers task/map/node/result growth. |

Final verification after fixes:

- `git diff --check`: PASS.
- Standalone viewer E2E: `scripts/run-tui-taskspace-viewer-e2e.ps1 -TimeoutSeconds 180`: PASS, latest standalone evidence `target/tui-taskspace-viewer-e2e/20260605-130426-955/artifacts/report.md`.
- Unified regression: `scripts/run-action-map-regression.ps1 -IncludeTuiViewerE2E`: PASS, final report `target/test-reports/action-map-20260605-132009-632/report.md`.
- Final unified report: `total_passed_tests=218`, `total_failed_tests=0`, `script_run_count=4`, `relevant_crash_events=0`.
- Final viewer report: `browser_interaction_ok=true`, `browser_refresh_count=4`, `browser_snapshot_status_ok=true`, `browser_snapshot_active_ok=true`, `refresh_during_detail_ok=true`, `refresh_during_graph_ok=true`, `refresh_during_selection_ok=true`, `graph_zoom_ok=true`, `graph_pan_ok=true`, `snapshot_map_count=1`, `snapshot_node_count=1`, `snapshot_result_count=3`, `assistant_marker_observed=true`, `user_prompt_contains_marker=false`.
- Final browser summary: saved all browser `/snapshot.json` responses and showed result count growth during refresh; every response status is 200; `consoleErrors=[]`; `networkFailures=[]`; favicon 404 is counted separately.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2 closure review by `019e9632-2f1f-77a2-8224-56155cc4e8b2`
- Blocking re-review launch records:
  - `019e9632-2f1f-77a2-8224-56155cc4e8b2` (`Avicenna`)
- Rejected findings backed by evidence: none
- Deferred findings documented: none after hardening; favicon 404 is separately counted and no longer contributes to `console_error_count`
- Blocked reason: none
- Allowed to proceed: yes

## Round 2: Blocking Closure Review

### Review Input

Fresh read-only subagent review targeted at the accepted Round 1 blocking issues and final artifacts.

Target files:

- `scripts/run-tui-taskspace-viewer-e2e.ps1`
- `scripts/run-action-map-regression.ps1`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/testing/2026-05-08-action-map-real-user-e2e.md`
- `target/test-reports/action-map-20260605-132009-632/report.md`
- `target/test-reports/action-map-20260605-132009-632/run-tui-taskspace-viewer-e2e.ps1/artifacts/report.md`
- `target/test-reports/action-map-20260605-132009-632/run-tui-taskspace-viewer-e2e.ps1/artifacts/snapshots/browser-summary.json`

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Read-only |
|---|---|---|---|---|---|
| closure-reviewer | `multi_agent_v1.spawn_agent` explorer | `019e9632-2f1f-77a2-8224-56155cc4e8b2` (`Avicenna`) | tool result in main thread | `fork_context=false` | yes |

### Reviewer Output

Summary:

第二轮只读审查认为第一轮 6 个 blocking 技术问题已由脚本断言和最终报告证据关闭；没有发现会让 viewer E2E 结果变成假阳性的 blocking 缺口。

Blocking Findings:

- 无。

Non-blocking risks and follow-up:

- 审查报告仍是 pending：已在本节写回并关闭。
- 每个 `/snapshot.json` status 还未纳入 hard assertion：已补 `browser_snapshot_status_ok`，并纳入 `browserInteractionOk`。
- favicon 404 与真实 console error 混在一起：已拆为 `favicon_console_error_count`，真实 `console_error_count=0`。

Evidence:

- Auto-refresh 使用真实 `/snapshot.json` 响应监听，最终 `browser-summary.json` 记录 4 次 status 200。
- Meaningful live viewer state 有硬断言：experiment mode、active map、node、result，并在 browser refresh window 中出现 active node/result stats。
- Graph zoom/pan 已拆开，最终报告显示 `graph_zoom_ok=true`、`graph_pan_ok=true`、`graph_transform_ok=true`。
- Details/selection 在刷新期间保持，最终报告显示 `refresh_during_detail_ok=true`、`refresh_during_selection_ok=true`。
- Marker echo 关闭，最终报告显示 `user_prompt_contains_marker=false`、`assistant_marker_observed=true`、`marker_count=1`。
- Regression wrapper 已 lift viewer 指标，最终主报告 `target/test-reports/action-map-20260605-132009-632/report.md` 的 `TUI Viewer E2E` section 可直接复核。

Verdict:

- Allowed to proceed: yes.

## Final Conclusion

Round 2 passed. The TaskSpace `/task-show` viewer E2E hardening is allowed to proceed.
