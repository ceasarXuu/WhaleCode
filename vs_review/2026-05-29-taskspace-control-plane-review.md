# Subagent VS Review: TaskSpace Control Plane

- Created: 2026-05-29T04:05:00+08:00
- Updated: 2026-05-30T03:13:00+08:00
- Task: Implement the second TaskSpace/runtime upgrade so agent actions are driven by task-map nodes, not only recorded after the fact.
- Report path: `vs_review/2026-05-29-taskspace-control-plane-review.md`
- Review mode: fresh internal subagents plus required `claude-ds-pro` CLI review
- Source session policy: no inherited main-agent context for internal subagents
- Status: closed

## Round 1: post-implementation adversarial review

### Review Input

#### Objective
Validate whether the TaskSpace control-plane implementation is sound enough to proceed: runtime must force ordinary agent actions to run on a task map/node, action classes must match node contracts, node lifecycle must converge, and validation must reflect real user flows rather than mocks.

#### Review Target
Code implementation, test strategy, and observability artifacts for the TaskSpace / Action Map second upgrade.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `scripts/run-action-map-real-user-e2e.ps1`
- `scripts/export-action-map-observability.ps1`
- latest real E2E report: `target/real-user-e2e/action-map-natural-user-order-pipeline/20260529-040133-508/artifacts/report.md`
- latest observability report: `target/real-user-e2e/action-map-natural-user-order-pipeline/20260529-040133-508/artifacts/action-map-observability.md`

#### Change Introduction
The implementation adds runtime `NodeKind`, `ActionClass`, static node contracts, pre-tool-call gate checks, tool action classification, node-kind-aware `taskspace_control` fields, atomic `finish_node` next-node creation/binding, main tool result action-class recording, blocked-action events, and E2E report checks based on node kind/action class rather than title regex. It also adds in-flight main-tool-call reservations so parallel tool batches cannot overrun the node tool-result budget before a maintenance barrier is raised.

#### Risk Focus
- Runtime gate may be too strict and block normal agent progress.
- Runtime gate may be too loose and let edit/test actions land in the wrong node.
- Tool action classification may misclassify shell or coordination tools.
- In-flight reservation accounting may leak or raise stale barriers.
- `taskspace_control` and subagent tooling may be accidentally blocked or incorrectly recorded.
- Real E2E may pass while still not proving natural map growth or multi-agent behavior.
- Observability may omit the events needed to debug failed TaskSpace runs.

#### Verification Status
- `cargo check -p codex-core --lib --locked`: passed.
- `cargo check -p codex-tools --lib --locked`: passed.
- `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2`: passed, 63 tests.
- `cargo test -p codex-core shell_action_classifier_identifies_core_taskspace_classes --lib --locked --jobs 2`: passed.
- `cargo test -p codex-app-server-protocol schema_fixtures --locked --jobs 2`: passed before the final reservation/update_plan tweaks; rerun is still required before closure.
- Real user E2E with current built binary: `scripts/run-action-map-real-user-e2e.ps1 -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe -TimeoutSeconds 1800`: passed, report path listed above.
- Known gap: latest local install to `C:\Users\77585\.whale\bin\whale.exe` failed because an existing whale process holds the file lock; E2E used the freshly built binary directly.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on findings that would invalidate the implementation, tests, or observability.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Multi-module runtime/state/tool-dispatch change. | correctness, state flow, concurrency, gate edge cases |
| test-validity-adversary | User explicitly requires real user E2E and rejects mock validation. | self-deceptive tests, missing black-box checks, weak pass criteria |
| observability-adversary | TaskSpace usefulness depends on seeing map growth, blocks, leases, and action ownership. | missing logs, report blind spots, viewer/debuggability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019e7030-cdae-70d1-8fdf-e0828baad2a0` / Nash | spawn tool result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` | `019e7030-e200-7a23-9916-bb6cbbfda6f4` / Heisenberg | spawn tool result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| observability-adversary | `multi_agent_v1.spawn_agent` | `019e7030-f887-74a3-99c3-7aa5d57d24f8` / Curie | spawn tool result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

Superseded. Round 1 findings led to the later hardening work and the fresh closure rounds below.

### Main Agent Response

Superseded by Round 2 and Round 3 responses.

### Closure Status

- Blocking findings found: superseded
- Accepted blocking findings fixed: superseded
- Blocking re-review completed: superseded
- Blocking re-review passed: superseded
- Rejected findings backed by evidence: superseded
- Deferred findings documented: superseded
- Allowed to proceed: superseded by final Round 3 closure

## Final Conclusion

Round 1 was superseded by later fixes and fresh closure review rounds below.

## Round 2: post-hardening closure review

### Review Input

#### Objective
Validate the current TaskSpace control-plane implementation after the latest hardening pass. The runtime must keep agent work map/node-driven, avoid stale or orphaned nodes, support real multi-agent node ownership, and prove the behavior through real Whale E2E runs rather than mocks.

#### Review Target
Code implementation, runtime state transitions, test strategy, and observability for the TaskSpace / Action Map second upgrade.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `third_party/codex-cli/codex-rs/tools/src/agent_tool.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `scripts/run-action-map-real-user-e2e.ps1`
- `scripts/run-action-map-growth-health-e2e.ps1`
- `scripts/export-action-map-observability.ps1`
- Natural real-user E2E report: `target/real-user-e2e/action-map-natural-user-order-pipeline/20260529-144001-701/artifacts/report.md`
- Growth/multi-agent E2E report: `target/real-user-e2e/action-map-growth-health-order-pipeline/20260529-143649-582/artifacts/report.md`
- Growth observability report: `target/real-user-e2e/action-map-growth-health-order-pipeline/20260529-143649-582/artifacts/action-map-observability.md`

#### Change Introduction
The current implementation adds hard-gated node kinds and action classes, static node contracts, main-tool preflight gates, node result action-class recording, blocked-action events, in-flight main-tool reservations, node-kind-aware `taskspace_control`, atomic `finish_node` next-node creation/binding, explicit next-node output, stricter node kind guidance, shell action classification fixes, full-history spawn override fixes, atomic create+bind failure behavior, and main-held-node handoff to subagents.

#### Risk Focus
- A node can still be created, bound, completed, or handed off in a non-atomic way that leaves orphaned ready/running nodes.
- Main-held-node handoff to subagents may violate the one-node-one-agent ownership rule.
- Tool action classification may be too strict or too loose, especially for shell commands that combine git operations and tests.
- Prompt/node-kind guidance may overfit the E2E scenario while failing natural user flows.
- E2E reports may still pass even when map growth is unhealthy or multi-agent behavior is superficial.
- Observability may omit enough detail to debug blocked actions, handoffs, lease state, or result ownership.

#### Verification Status
- `cargo build -p codex-cli --bin whale --locked --jobs 2`: passed.
- `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2`: passed, 66 tests.
- `cargo test -p codex-core tools::parallel --lib --locked --jobs 2`: passed, 2 tests.
- `cargo test -p codex-core multi_agent_v2_spawn_ --lib --locked --jobs 2`: passed, 13 tests.
- `cargo test -p codex-tools agent_tool --lib --locked --jobs 2`: passed, 8 tests.
- `cargo test -p codex-app-server-protocol schema_fixtures --locked --jobs 2`: passed.
- Natural real-user E2E: `scripts/run-action-map-real-user-e2e.ps1 -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe -TimeoutSeconds 1800`: passed, report path listed above.
- Growth/multi-agent E2E: `scripts/run-action-map-growth-health-e2e.ps1 -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe -TimeoutSeconds 2400`: passed, report path listed above; observed 1 map, 7 nodes, 2 agents, 2 `lease_attached`, parser and pricing subagent results, implementation node tools, validation pytest result, no empty terminal nodes.
- Known residual signal: the growth E2E still includes one blocked attempt when the agent tries pytest inside an inspect node before creating a smoke/regression node. This appears to be runtime steering rather than state corruption, but reviewers should challenge that assumption.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files and reports directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on findings that would invalidate the implementation, tests, observability, or claim that this is ready for local install/commit.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Multi-module runtime/state/tool-dispatch changes, including ownership handoff. | correctness, state flow, lease atomicity, gate edge cases |
| test-validity-adversary | User requires real user path and rejects mocks. | self-deceptive E2E, weak pass criteria, missing black-box checks |
| observability-adversary | TaskSpace value depends on seeing map growth, blocks, handoffs, leases, and action ownership. | missing logs, report blind spots, viewer/debuggability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| closure-runtime-adversary | `multi_agent_v1.spawn_agent` | `019e74fb-c766-76b3-b97d-e271b11776f4` / Copernicus | spawn tool result in current Codex thread | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

Superseded. Round 2 findings fed into the final Round 3 hardening and closure review.

### Main Agent Response

Superseded by Round 3 response.

### Closure Status

- Blocking findings found: superseded
- Accepted blocking findings fixed: superseded
- Blocking re-review completed: superseded
- Blocking re-review passed: superseded
- Rejected findings backed by evidence: superseded
- Deferred findings documented: superseded
- Allowed to proceed: superseded by final Round 3 closure

## Round 3: final closure review after runtime collaboration gate

### Review Input

#### Objective
Validate the final TaskSpace control-plane implementation after the last hardening pass. The main question is whether ordinary work is now structurally forced onto task-map nodes, whether subagent work is recorded on the parent node, and whether natural user E2E now proves healthy map growth plus real multi-agent collaboration without leaking internal task/map/node terminology in the user prompt.

#### Review Target
Implementation, tests, scripts, and design documentation for:
- action class + node kind runtime gates
- parent-runtime child tool gate
- node-bound nested `spawn_agent` blocking
- multiple-ready-inspect structural gate forcing `spawn_agent node_id`
- real E2E scripts and observability reports

#### Target Locations
- `docs/plans/2026-05-28-map-driven-runtime-control-plane-design.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/agent/control.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `scripts/action-map-real-user-e2e-lib.ps1`
- `scripts/run-action-map-real-user-e2e.ps1`
- `scripts/run-action-map-natural-multi-agent-e2e.ps1`
- `scripts/run-action-map-growth-health-e2e.ps1`

#### Change Introduction
The latest implementation added two important hardening changes after earlier review findings:

1. Child sessions now fail closed if the parent TaskSpace runtime cannot be found, and node-bound child sessions cannot call nested `spawn_agent`.
2. If a map contains multiple ready `inspect_code_context` nodes, `bind_main_node` rejects sequential main-agent binding and tells the model to use explicit `spawn_agent node_id` delegation. This is a structural gate based only on node kind/status/lease, not semantic routing.

The E2E harness was also tightened around real binary identity and then adjusted to distinguish unrecovered runtime failures from normal baseline test failures, recovered patch/read retries, and expected contract blocks that demonstrate the runtime gate working.

#### Verification Status
- `rustup run stable cargo fmt --all`: passed with existing stable rustfmt warning about `imports_granularity`.
- `cargo test -p codex-core developer_context_is_experiment_only_and_exposes_basemap_without_active_map --lib --locked --jobs 2`: passed.
- `cargo test -p codex-core bind_main_node_rejects_sequential_claim_when_multiple_inspect_nodes_are_ready --lib --locked --jobs 2`: passed.
- `cargo build -p codex-cli --bin whale --locked --jobs 2`: passed; tested binary `D:\BuildCache\whalecode\cargo-target\debug\whale.exe`, SHA256 `6337A5CB820C4D96C8D7B426C0EC0491AA268840DAE356D2E05385E9F4E0C1CE`.
- Natural user E2E: `target/real-user-e2e/action-map-natural-user-order-pipeline/20260530-021324-815/artifacts/report.md`, PASS.
- Natural multi-agent E2E: `target/real-user-e2e/action-map-natural-multi-agent-order-pipeline/20260530-021748-931/artifacts/report.md`, PASS, 2 agents, 2 `spawn_agent`, 18 subagent results.
- Growth health E2E: `target/real-user-e2e/action-map-growth-health-order-pipeline/20260530-021024-828/artifacts/report.md`, PASS, 2 agents, 2 `spawn_agent`, 6 nodes, parser/pricing investigation nodes both used, no unexpected gate failures.
- Full action-map regression: `target/test-reports/action-map-20260530-022004-474/report.md`, PASS, 142 passed, 0 failed.

#### Reviewer Instructions
- Fresh internal subagent session.
- Do not inherit this main-agent context.
- Read the target files and reports directly.
- Do not modify files.
- Cite evidence paths and line numbers where possible.
- Focus on blocking issues that would invalidate readiness to commit/install: runtime bypasses, overbroad gate behavior, false E2E pass criteria, missing observability, or inconsistency between docs and implementation.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| closure-runtime-adversary | Accepted blocking findings were fixed and a new structural gate was added. | correctness, bypasses, overblocking, doc/test consistency |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| closure-runtime-adversary | `multi_agent_v1.spawn_agent` | `019e74fb-c766-76b3-b97d-e271b11776f4` / Copernicus | subagent notification in current Codex thread | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions | yes |
| final-closure-adversary | `multi_agent_v1.spawn_agent` | `019e7517-ad61-78c3-8673-2abc11ca3f29` / McClintock | subagent notification in current Codex thread | no | post-fix closure review packet | main-agent history, reasoning, drafts, conclusions | yes |
| external-cli-adversary | `claude-ds-pro` | `vs_review/2026-05-30-taskspace-control-plane-claude-ds-pro-final.md` and `vs_review/2026-05-30-taskspace-control-plane-claude-ds-pro-postfix.md` | local CLI output captured to Markdown files | no | neutral read-only review prompt | main-agent hidden context | yes |

### Reviewer Outputs

#### Copernicus closure review

Copernicus reported four blocking readiness gaps:

1. `spawn_agent` could bypass the current main node contract: `FinalSynthesis` disallowed `Spawn`, but `prepare_spawn_assignment` did not check the current main node contract before assignment.
2. Round 3 review artifact was still pending and not auditable.
3. Growth E2E still mixed runtime claims with title-regex scenario checks.
4. Installed-binary readiness was unproven because final E2E evidence used `D:\BuildCache\whalecode\cargo-target\debug\whale.exe`.

Non-blocking risks included blank natural multi-agent observability paths and nested child spawn blocks not being emitted as ordinary blocked-action observability.

#### `claude-ds-pro` final review

The first CLI review reported five findings:

1. Build/check/lint commands were classified as `Test`, which overblocked implementation nodes.
2. `git stash` was classified as `Edit`.
3. Natural multi-agent E2E report referenced undefined observability path variables.
4. Missing direct gate test for `FinalSynthesis + Edit/Test/Spawn`.
5. Maintenance barriers were keyed by map only, not by node.

The post-fix CLI review concluded `Blocking Findings: none`, but still recommended improving growth E2E artifact reporting and kind/action checks. It also noted that the installed-binary path was now validated.

#### McClintock final closure review

McClintock concluded `Blocking Findings: none`.

Evidence cited by the reviewer:

- `runtime.rs` enforces current node contract before ordinary tool calls and before spawn assignment.
- `contracts.rs` keeps `FinalSynthesis` without `Spawn`.
- `parallel.rs` distinguishes `Build` from `Test`.
- Natural multi-agent E2E report exposes `observability_json`, `observability_md`, and `observability_html`.
- Installed-binary E2E used `C:\Users\77585\.whale\bin\whale.exe` and the same SHA256 as the built binary.

### Main Agent Response

| Finding | Response | Action |
|---|---|---|
| `spawn_agent` bypasses current main node contract | accept | Added current-main-node `ActionClass::Spawn` contract validation inside `prepare_spawn_assignment`; added `final_synthesis_node_blocks_edit_test_and_spawn` regression. |
| Build/check/lint commands overblocked as `Test` | accept | Added `ActionClass::Build`; classified `cargo build/check/clippy`, `npm run build/lint/typecheck`, `tsc --noEmit`, `cargo fmt --check` as `Build`; allowed `Build` in inspect/implementation/test/custom contracts while keeping `Test` blocked in implementation nodes. |
| Natural multi-agent observability paths blank | accept | Defined `observability_json/md/html` paths in `run-action-map-natural-multi-agent-e2e.ps1`; reran natural multi-agent E2E and confirmed paths appear in report. |
| Missing FinalSynthesis gate test | accept | Added direct runtime test covering `FinalSynthesis` blocking `Edit`, `Test`, and `Spawn`. |
| Round 3 review artifact pending | accept | Updated this report with launch records, reviewer outputs, main-agent responses, validation evidence, and closure status. |
| Installed-binary readiness unproven | accept | Installed `D:\BuildCache\whalecode\cargo-target\debug\whale.exe` to `C:\Users\77585\.whale\bin\whale.exe`; verified SHA256 `0C4BB6139083C62398754D759C0C662985C2DC47807A0615455A9F6237A27F82`; reran real-user E2E against installed path. |
| Growth E2E artifact/report and kind/action checks weak | accept | Added report artifact paths and hard checks for inspect/implementation/test kind coverage and unknown action results; kept business title checks as scenario health checks rather than runtime gate claims. |
| `git stash` classified as `Edit` | reject | `git stash` mutates worktree/index state and can hide edits during test nodes; first version intentionally blocks it outside implementation/control flow. Test isolation should use the sandbox repo or an explicit implementation/control node. |
| Maintenance barrier keyed by map only | reject for first version | The current maintenance barrier is intentionally map-level to stop ordinary work on an overgrown task path until main splits or routes the task. Existing tests cover routing to another task and explicit separate ready-node spawn while a barrier exists. Per-node barriers can be reconsidered later if map-level pressure proves too coarse. |
| Nested child spawn block not emitted as ordinary blocked-action event | defer | The behavior is blocked fail-closed. Adding a separate observability event for child nested spawn blocks is useful but not required for this control-plane closure. |
| `problematic_successful_tool_results` metric can show shell non-terminating errors | defer | The metric is retained as diagnostic. It did not indicate TaskSpace gate failure; latest growth E2E has `problematic_successful_tool_results: 0`. |

Post-fix validation:

- `rustup run stable cargo fmt --all`: PASS, with existing stable rustfmt `imports_granularity` warning.
- `cargo test -p codex-core shell_action_classifier_identifies_core_taskspace_classes --lib --locked --jobs 2`: PASS.
- `cargo test -p codex-core final_synthesis_node_blocks_edit_test_and_spawn --lib --locked --jobs 2`: PASS.
- `cargo test -p codex-core validation_node_allows_test_but_blocks_edit --lib --locked --jobs 2`: PASS.
- `cargo build -p codex-cli --bin whale --locked --jobs 2`: PASS; built SHA256 `0C4BB6139083C62398754D759C0C662985C2DC47807A0615455A9F6237A27F82`.
- Natural multi-agent E2E: `target/real-user-e2e/action-map-natural-multi-agent-order-pipeline/20260530-024310-236/artifacts/report.md`, PASS.
- Growth health E2E after stricter report checks: `target/real-user-e2e/action-map-growth-health-order-pipeline/20260530-030549-184/artifacts/report.md`, PASS.
- Natural user E2E against installed binary: `target/real-user-e2e/action-map-natural-user-order-pipeline/20260530-025355-824/artifacts/report.md`, PASS.
- Full action-map regression: `target/test-reports/action-map-20260530-024948-734/report.md`, PASS, 143 passed, 0 failed.

### Closure Status

- Blocking findings found: yes, in Copernicus and first `claude-ds-pro` review.
- Accepted blocking findings fixed: yes.
- Blocking re-review completed: yes, McClintock fresh internal subagent and post-fix `claude-ds-pro`.
- Blocking re-review passed: yes, no remaining blocking findings.
- Allowed to proceed: yes.

## Final Conclusion

TaskSpace control-plane upgrade is ready to commit and install. The remaining deferred items are observability polish, not correctness blockers.
