# TaskSpace Runtime Cognitive Gates Review

Date: 2026-06-05
Status: second closure review in progress

## Round 1

### Review Target

- Target type: code implementation, runtime state machine, E2E observability, regression tests
- Objective: make TaskSpace force healthy map-driven behavior for broad inspect tasks, ensure accepted result evidence supports final-artifact audit chains, and prove the path through real Whale E2E.
- Changed areas:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
  - `scripts/action-map-final-artifact-audit-lib.ps1`
  - `scripts/action-map-real-user-e2e-lib.ps1`
  - `scripts/run-action-map-growth-health-e2e.ps1`
  - `scripts/run-action-map-natural-multi-agent-e2e.ps1`
  - `scripts/run-action-map-real-user-e2e.ps1`
  - `scripts/test-action-map-observability-lib.ps1`

### Verification Before Review

- `cargo fmt --all`: pass, with existing stable rustfmt `imports_granularity` warnings.
- `cargo test -p codex-core action_map --lib --locked`: pass, 137 tests.
- `.\scripts\test-action-map-observability-lib.ps1`: pass.
- `.\scripts\run-action-map-regression.ps1`: pass, latest report `D:\whalecode-alpha\target\test-reports\action-map-20260605-055125-056\report.md`.
- Local install: `C:\Users\77585\.whale\bin\whale.exe`, SHA-256 `98575FB4DA19E3F749CFB79A77C22B4580A3DBB0524DA20B03F9FF09AA235841`.
- Real E2E: `.\scripts\run-action-map-growth-health-e2e.ps1 -TimeoutSeconds 1200`: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-055747-016\artifacts\report.md`.

### Verification After Fixes

- `cargo fmt --all`: pass, with existing stable rustfmt `imports_granularity` warnings.
- `cargo test -p codex-core action_map --lib --locked`: pass, 144 tests.
- `.\scripts\test-action-map-observability-lib.ps1`: pass.
- `.\scripts\test-action-map-real-user-e2e-lib.ps1`: pass.
- `.\scripts\run-action-map-regression.ps1`: pass, latest report `D:\whalecode-alpha\target\test-reports\action-map-20260605-081227-961\report.md`.
- Local install: `C:\Users\77585\.whale\bin\whale.exe`, SHA-256 `E2A8FE52A3759E773EFEA76C3FD4BE74627B72CBE84EDC134EFD04472A18CC06`, `whale --version` = `whale 0.1.0`.
- Real E2E growth-health: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-074442-388\artifacts\report.md`.
- Real E2E natural multi-agent: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-multi-agent-order-pipeline\20260605-074944-524\artifacts\report.md`.
- Real E2E natural user: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-user-order-pipeline\20260605-080750-459\artifacts\report.md`.

### Reviewer Launch Records

#### Reviewer A

- Role: runtime state-machine reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e94a9-7903-7873-ac8a-b353750148bb`
- Nickname: `Gauss`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes
- Context explicitly excluded: full chat history, main-agent reasoning, patch persuasion brief

Navigation packet:

```text
Objective: Review recent TaskSpace/action-map runtime changes in D:\whalecode-alpha for correctness and maintainability. The product goal is to make TaskSpace force healthy map-driven behavior for broad inspect tasks: once an inspect_code_context node exhausts its main-tool budget, it must not go straight into implement_solution without subagent investigation; accepted implementation/test results must support final-artifact audit chains.

Review target: Rust runtime state-machine changes.

Target files:
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\action_map\basemap.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\tools\src\taskspace_tool.rs
- Related tests in runtime.rs and multi_agents_tests.rs/session tests if needed.

Risk focus:
- Could validate_broad_inspect_finish_transition block legitimate simple tasks or trap the main agent after a barrier?
- Does removing the current-node early return from validate_broad_inspect_delegation create over-blocking?
- Are cognitive preflight and result-review gates ordered correctly around ordinary tools and spawn_agent?
- Are error messages actionable enough for the model to recover by creating ready inspect nodes and spawning subagents?
- Any state mutation before validation error that could leave inconsistent map/lease state?

Verification already run by main agent:
- cargo fmt --all
- cargo test -p codex-core action_map --lib --locked: 137 passed
- scripts/run-action-map-regression.ps1: PASS
- installed whale.exe and ran scripts/run-action-map-growth-health-e2e.ps1 -TimeoutSeconds 1200: PASS, 7 nodes, 2 agents, 2 spawn_agent, cognitive hard gate true.

Reviewer output contract: return summary, blocking findings, non-blocking risks, required fixes, missing tests, missing observability/logs. Cite file paths and line numbers where possible. Do not rely on this prompt as proof; inspect files directly.
```

#### Reviewer B

- Role: E2E and final-artifact audit reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e94a9-cbc7-7053-8562-915b52004424`
- Nickname: `Poincare`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes
- Context explicitly excluded: full chat history, main-agent reasoning, patch persuasion brief

Navigation packet:

```text
Objective: Review recent TaskSpace/action-map E2E and observability audit changes in D:\whalecode-alpha for correctness and anti-self-deception. The product goal is to make the real-user E2E prove healthy map growth, subagent ownership, dependency edges, accepted result evidence, final artifact why-chain, and no internal prompt leakage.

Review target: PowerShell E2E scripts and final-artifact audit logic.

Target files:
- D:\whalecode-alpha\scripts\action-map-final-artifact-audit-lib.ps1
- D:\whalecode-alpha\scripts\action-map-real-user-e2e-lib.ps1
- D:\whalecode-alpha\scripts\run-action-map-growth-health-e2e.ps1
- D:\whalecode-alpha\scripts\run-action-map-natural-multi-agent-e2e.ps1
- D:\whalecode-alpha\scripts\run-action-map-real-user-e2e.ps1
- D:\whalecode-alpha\scripts\test-action-map-observability-lib.ps1

Risk focus:
- Does the audit now over-credit artifacts by linking accepted validator results too broadly?
- Is deriving final artifacts from accepted implementation evidenceRefs intersected with successful edit output sound, especially with apply_patch escaped output?
- Does treating final_synthesis blocked edit/test/build/spawn as expected hide real failures?
- Do growth-health metrics still prove subagent-owned independent tracks and implementation dependency edges, or can they be gamed?
- Are line-count/script complexity and test coverage acceptable?

Verification already run by main agent:
- scripts/test-action-map-observability-lib.ps1: PASS
- scripts/run-action-map-regression.ps1: PASS
- installed whale.exe and ran scripts/run-action-map-growth-health-e2e.ps1 -TimeoutSeconds 1200: PASS, report at D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-055747-016\artifacts\report.md

Reviewer output contract: return summary, blocking findings, non-blocking risks, required fixes, missing tests, missing observability/logs. Cite file paths and line numbers where possible. Do not modify files and do not trust the prompt as proof; inspect files and reports directly.
```

### Reviewer Outputs

#### Reviewer A Findings

Blocking:

- Final answers bypassed `validate_lifecycle_result_reviewed`, allowing unreviewed node results to remain hidden until ordinary tool/spawn calls.
- Broad inspect delegation could be satisfied by unrelated or questioned subagent work because the check treated subagent activity too loosely.
- Accepted implementation/test result evidence fields were described by prompt/schema but not enforced by runtime.
- `finish_node` could mutate current state before later rejecting an invalid next-node bind/create path.

Non-blocking:

- Broad-task heuristics and maintenance barrier logic need continued E2E pressure because they rely on observed structure, not semantic certainty.

#### Reviewer B Findings

Blocking:

- Final user output could leak hidden orchestration concepts; the E2E only checked user prompt leakage.
- Growth-health PASS could be achieved without parser/pricing-specific independent proof.
- Final-artifact audit could over-credit artifacts through broad validator linking.

Non-blocking:

- Treating final_synthesis blocked actions as expected could hide a regression.
- Edit-output fallback could over-associate all edits in a node if accepted results were vague.
- Some scripts were close to the 500-line project limit.

### Main-Agent Responses

Accepted and fixed:

- Final response now runs cognitive preflight and lifecycle-result review gates before completing a `final_synthesis` node.
- Broad inspect handoff now requires two related, completed, accepted subagent inspect results; questioned results do not unlock implementation.
- Broad accepted main inspect results with enough structural evidence are treated as broad even if the tool-budget barrier did not fire.
- Starting a new task is blocked when the current active task still has unresolved broad-delegation debt, preventing task-switch bypass.
- Accepted `implement_solution` results require `changed_artifacts`; accepted `smoke_test`/`regression_test` results require validator evidence.
- `finish_node` validates pending-ready inspect conflicts and next-node drafts before mutating the current node.
- Final-synthesis prompts and tool descriptions now forbid hidden orchestration terms in user-facing final answers.
- Real-user, natural-multi-agent, and growth-health E2E scripts check final output leakage.
- Growth-health now verifies parser/pricing independent investigation use and dependency edges into implementation.
- Final-artifact audit derives final artifacts from explicit changed artifacts or accepted implementation evidence intersected with successful edit output. Broad validator-to-artifact linking was removed.
- final_synthesis blocked action failures are no longer blanket-expected; only recovered runtime hard gates are excluded from unexpected-failure counts.
- Expected recovered collaboration gates are limited to stale/completed-node spawn, lifecycle-review gate, active lease gate, and narrow single-track inspect gate.

Tests added or updated:

- Runtime tests for broad direct-implementation block, broad accepted-result block, questioned subagent result rejection, start-task bypass rejection, final-response lifecycle rejection, implementation changed-artifact enforcement, test validator-evidence enforcement, and pending inspect prevalidation.
- Session and multi-agent handler tests updated for cognitive preflight requirements and result review.
- PowerShell self-tests updated for artifact-audit and recovered collaboration-gate filters.

### Closure

Round 1 closure found additional blocking issues; see Round 2.

## Round 2

### Closure Review Launch Records

#### Reviewer C

- Role: runtime closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e9522-3b4c-7a13-8c4a-eaed6e95c8c8`
- Nickname: `Averroes`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes

#### Reviewer D

- Role: E2E/audit closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e9522-980c-78a2-bb4e-4ee8e3ac6fe1`
- Nickname: `Sartre`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes

### Reviewer Outputs

#### Reviewer C Findings

Blocking:

- `final_synthesis` could still close broad investigation debt and could close after edits without an accepted validation result.
- `route_task` could bypass unresolved broad-inspect delegation debt by routing away from the active task.

Non-blocking:

- Broadness detection remains heuristic and may need continued pressure testing.
- Prompt-level “after accepted validation” needed runtime enforcement.

#### Reviewer D Findings

Blocking:

- Final-output leakage check under-matched. A real natural multi-agent run said it “delegated two independent evidence tracks to parallel explorers”, but the report still showed `final_output_leaks_internal_concepts: False`.

Non-blocking:

- Final-artifact audit can still over-credit per-artifact why-chains in multi-file results.
- Recovered collaboration-gate filtering is text allowlist based and does not yet prove later recovery per call.
- Parser/pricing identification is title-regex based, though dependency checks are real and direct-edge-aware.

### Main-Agent Responses

Accepted and fixed:

- Added a shared `validate_no_unresolved_broad_delegation_debt` guard and applied it to `start_task`, `route_task`, and `record_main_final_response`.
- Added final-synthesis validation debt guard: after any successful edit action in the active map, final response is rejected until there is an accepted `smoke_test` or `regression_test` result.
- Added runtime tests:
  - `final_response_rejects_unresolved_broad_delegation_debt`
  - `final_response_rejects_missing_validation_after_edit`
  - `route_task_rejects_broad_delegation_debt_bypass_without_mutation`
- Centralized the internal orchestration leak regex in `action-map-real-user-e2e-lib.ps1`.
- Expanded final-output leak detection to catch `delegated`, `parallel`, `explorer`, `evidence track`, and `fan-out`.
- Added leak excerpts to all real-user E2E reports.
- Added E2E lib self-test coverage for leaked orchestration phrasing.
- Strengthened final_synthesis prompts in runtime, BaseMap metadata, and taskspace tool docs to forbid `evidence track` and `fan-out` in user-facing final answers.

Deferred with rationale:

- Per-artifact why-chain matching and per-call recovered gate proof are useful audit refinements, but they are non-blocking for this closure because the current blocking leak and runtime bypass paths are fixed and covered. Track these as future audit-hardening work rather than blocking this runtime gate change.

### Verification After Round 2 Fixes

- `cargo fmt --all`: pass, with existing stable rustfmt `imports_granularity` warnings.
- `cargo test -p codex-core action_map --lib --locked`: pass, 147 tests.
- `.\scripts\test-action-map-observability-lib.ps1`: pass.
- `.\scripts\test-action-map-real-user-e2e-lib.ps1`: pass.
- `.\scripts\run-action-map-regression.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-20260605-084022-205\report.md`.
- Rebuilt and installed Whale: `C:\Users\77585\.whale\bin\whale.exe`, SHA-256 `ECF7FB0B8AB0495F555BD3169B29BD5756EA110AB35CDE0F43A61A1670F657CE`, `whale --version` = `whale 0.1.0`.
- Real E2E growth-health: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-084746-181\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, `parser_pricing_independent: True`, `implementation_depends_on_parser_and_pricing: True`, `spawn_agent: 2`.
- Real E2E natural multi-agent: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-multi-agent-order-pipeline\20260605-085205-656\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, empty `final_output_leak_excerpt`, `nodes: 7`, `agents: 2`, `unexpected_failed_collab_tool_calls: 0`.
- Real E2E natural user: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-user-order-pipeline\20260605-085623-159\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, empty `final_output_leak_excerpt`, `nodes: 6`, `agents: 2`, `pytest_owned_by_validation_node: True`, `hidden_oracle_exit_code: 0`.

### Closure

Round 2 uncovered additional closure findings in later review passes:

- Final-answer gate errors must not be swallowed after the assistant has already produced a final message.
- Completed `final_synthesis` nodes must still gate the final assistant message for hidden orchestration leakage.
- Real-user E2E leakage detection must avoid broad false positives while still catching internal orchestration phrasing.
- E2E reports must include enough script/library/binary identity to replay exactly what was tested.
- Whale-owned script files must remain under the 500-line project limit.

Accepted and fixed:

- `Session::record_action_map_main_final_response` now returns an error instead of swallowing runtime final-gate failures.
- Turn completion now forces follow-up when the TaskSpace final gate rejects a completed assistant message, and injects a developer correction into conversation history for model recovery.
- `record_main_final_response` now validates hidden orchestration terms even after a `final_synthesis` node was completed by `finish_node` and the current main node lease has been cleared.
- `finish_main_node_with_next` also validates hidden orchestration terms when directly completing a `final_synthesis` node.
- Leakage checks are centralized in `action-map-real-user-e2e-lib.ps1`, report matched excerpts, include the shared E2E library hash, and no longer flag ordinary phrases such as "map every function".
- `run-action-map-growth-health-e2e.ps1`, `run-action-map-natural-multi-agent-e2e.ps1`, and `run-action-map-real-user-e2e.ps1` are all under 500 physical lines.

Verification after these fixes:

- `cargo fmt --all`: pass, with existing stable rustfmt `imports_granularity` warnings.
- `cargo test -p codex-core completed_final_synthesis_still_gates_final_response_terms --lib --locked`: pass, 1 test.
- `cargo test -p codex-core internal_orchestration_terms --lib --locked`: pass, 2 tests.
- `cargo test -p codex-core action_map --lib --locked`: pass, 151 tests.
- `.\scripts\test-action-map-real-user-e2e-lib.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-real-user-e2e-lib\report.md`.
- `.\scripts\test-action-map-observability-lib.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-observability-lib\report.md`.
- `.\scripts\run-action-map-regression.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-20260605-105642-466\report.md`.
- Rebuilt and installed Whale: `C:\Users\77585\.whale\bin\whale.exe`, SHA-256 `ED828F6911DA28468BA63057A8A7625809078DA1AFA2800E41CC32D1DE7C4904`; built and installed binary hashes match.
- CLI isolation check: pass.
- Build profile policy check: pass.
- Real E2E natural multi-agent: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-multi-agent-order-pipeline\20260605-110405-620\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, `nodes: 8`, `agents: 2`, `spawn_agent_calls: 2`, `implementation_depends_on_parallel_inspect_tracks: True`, `test_depends_on_implementation: True`.
- Real E2E growth-health: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-110903-269\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, `nodes: 10`, `agents: 6`, `spawn_agent: 6`, `parser_pricing_independent: True`, `implementation_depends_on_parser_and_pricing: True`, `validation_node_has_pytest_result: True`.
- Real E2E natural-user: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-user-order-pipeline\20260605-111535-549\artifacts\report.md`; key metrics include `prompt_leaks_internal_concepts: False`, `final_output_leaks_internal_concepts: False`, `nodes: 6`, `agents: 2`, `pytest_owned_by_validation_node: True`, `hidden_oracle_exit_code: 0`.

## Round 3

### Closure Review Launch Records

#### Reviewer E

- Role: runtime/session final-gate closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e95cd-3d7c-7433-9fc3-8ab4071dd312`
- Nickname: `Parfit`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes
- Context explicitly excluded: full chat history, main-agent reasoning, patch persuasion brief

Navigation packet:

```text
Objective: Fresh closure review for TaskSpace/action-map runtime gates after latest fixes in D:\whalecode-alpha. Read-only. Do not edit files. Do not inherit or assume the main conversation; inspect the files and reports directly.

Review target: runtime/session final gate and map-driven lifecycle enforcement.

Target files:
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\session\mod.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\session\turn.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\session\tests.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\tools\handlers\multi_agents_tests.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\core\src\action_map\basemap.rs
- D:\whalecode-alpha\third_party\codex-cli\codex-rs\tools\src\taskspace_tool.rs

Risk focus:
- Can a final assistant answer still bypass final_synthesis gates after the final node was already completed through taskspace_control finish_node?
- Does Session turn handling really prevent rejected final assistant messages from being sent as final user output, and does it feed a developer correction back to the model?
- Do final_synthesis gates cover unresolved broad delegation debt, unreviewed prior results, missing validation after edits, and hidden orchestration term leakage?
- Are any validations performed after state mutation in a way that can leave inconsistent node/map/lease state?
- Are errors actionable enough for the agent to recover without exposing internal TaskSpace concepts to the user?
```

#### Reviewer F

- Role: E2E harness and observability closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e95cd-a0fb-79c3-9d4a-708937b96e80`
- Nickname: `Erdos`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes
- Context explicitly excluded: full chat history, main-agent reasoning, patch persuasion brief

Navigation packet:

```text
Objective: Fresh closure review for TaskSpace/action-map E2E harness and observability evidence after latest fixes in D:\whalecode-alpha. Read-only. Do not edit files. Do not inherit or assume the main conversation; inspect files and reports directly.

Review target: PowerShell real-user E2E harness, leak detection, graph health, report evidence, and project constraints.

Target files:
- D:\whalecode-alpha\scripts\action-map-final-artifact-audit-lib.ps1
- D:\whalecode-alpha\scripts\action-map-real-user-e2e-lib.ps1
- D:\whalecode-alpha\scripts\run-action-map-growth-health-e2e.ps1
- D:\whalecode-alpha\scripts\run-action-map-natural-multi-agent-e2e.ps1
- D:\whalecode-alpha\scripts\run-action-map-real-user-e2e.ps1
- D:\whalecode-alpha\scripts\test-action-map-observability-lib.ps1
- D:\whalecode-alpha\scripts\test-action-map-real-user-e2e-lib.ps1

Risk focus:
- Does final-output internal orchestration leakage detection catch the prior leak class without obvious false positives that would invalidate natural code tasks?
- Do reports include enough replay identity: runner hash, shared lib hash, whale hash, rollout/jsonl, last-message, observability, and hidden oracle/test outputs?
- Does growth-health still prove healthy graph growth: multiple nodes, ordered edges, independent inspect tracks, implementation depending on inspect tracks, tests depending on implementation, no open final nodes, no edit/test ownership violations?
- Are expected failed gate allowlists too broad, hiding real failure?
- Are scripts under the 500-line project limit?
```

### Reviewer Outputs

#### Reviewer E Findings

Blocking:

- Completed `final_synthesis` path only rechecked hidden terms, not the full final gate. A post-final mutation could invalidate lifecycle or validation state while a clean final answer still passed.
- Runtime hidden-term gate was narrower than the product/E2E forbidden set; it missed `node-1`, `map-1`, `task-1`, `task map`, `delegating`, `multi agent`, and related variants.

Non-blocking:

- Rejected final assistant text is removed from `TurnComplete.last_agent_message` and forces follow-up, but streamed deltas may already have been emitted before the final gate runs.
- Add structured final-gate rejection events in a future observability pass.

#### Reviewer F Findings

Blocking:

- None for E2E harness behavior or evidence closure.

Non-blocking:

- Leak detection caught the prior leak class, but the standalone `parallel/concurrent/simultaneous` terms risked false positives for ordinary code tasks.
- Self-test and regression reports are less forensic than the real E2E reports, though real E2E replay identity is strong.

### Main-Agent Responses

Accepted and fixed:

- Added shared `validate_final_response_ready` and routed all final-answer close paths through it:
  - `finish_node` completion of a running `final_synthesis`
  - final assistant response while `final_synthesis` is running
  - final assistant response after a `final_synthesis` node was already completed and current main lease was cleared
- Completed-final path now reruns cognitive preflight, lifecycle result review, unresolved broad-delegation debt, post-edit validation, and hidden orchestration leakage checks.
- Lifecycle review now has an explicit completed-final exclusion, so the completed final answer's own result does not require another `mark_result_validity`, while non-final unreviewed results still block final response.
- Runtime hidden-term gate now rejects hard IDs and variants: `map-*`, `node-*`, `task-*`, `task map`, `delegating`, `multi agent`, `split agents`, `fan out`.
- Runtime hidden-term matching now uses boundary-aware matching rather than raw substring checks.
- Runtime and E2E leakage checks no longer treat ordinary standalone `parallel/concurrent/simultaneous` technical terms as leaks. They still reject those terms when tied to internal orchestration actors such as agents, subagents, explorers, or evidence tracks.
- E2E shared leak regex and self-test were updated with both positive prior-leak cases and negative ordinary-concurrency cases.

Tests added or updated:

- `completed_final_synthesis_rechecks_unreviewed_results_before_final_response`
- `completed_final_synthesis_rechecks_validation_after_edit_before_final_response`
- `final_answer_hidden_term_gate_rejects_ids_and_variants`
- Updated final synthesis leakage tests to use hard ID and delegation/explorer language.
- Updated E2E lib self-test with ordinary concurrency negative fixture.

Verification after Round 3 fixes:

- `cargo fmt --all`: pass, with existing stable rustfmt `imports_granularity` warnings.
- `cargo test -p codex-core final_answer_hidden_term_gate_rejects_ids_and_variants --lib --locked`: pass.
- `cargo test -p codex-core completed_final_synthesis_rechecks --lib --locked`: pass, 2 tests.
- `cargo test -p codex-core internal_orchestration_terms --lib --locked`: pass, 2 tests.
- `cargo test -p codex-core action_map --lib --locked`: pass, 154 tests.
- `.\scripts\test-action-map-real-user-e2e-lib.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-real-user-e2e-lib\report.md`.
- `.\scripts\test-action-map-observability-lib.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-observability-lib\report.md`.
- `.\scripts\run-action-map-regression.ps1`: pass, report `D:\whalecode-alpha\target\test-reports\action-map-20260605-115054-540\report.md`.
- Rebuilt and installed Whale: `C:\Users\77585\.whale\bin\whale.exe`, SHA-256 `6DF903A74E42860C4C257EC39E3BD69AA17D13817481FF8E709F6BBC8EA8EA4B`; built and installed hashes match.
- CLI isolation check: pass.
- Build profile policy check: pass.
- Real E2E natural multi-agent: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-multi-agent-order-pipeline\20260605-115756-120\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, `nodes: 8`, `agents: 2`, `spawn_agent_calls: 2`, `implementation_depends_on_parallel_inspect_tracks: True`, `direct_test_depends_on_implementation: True`.
- Real E2E growth-health: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260605-120302-221\artifacts\report.md`; key metrics include `final_output_leaks_internal_concepts: False`, `nodes: 7`, `agents: 2`, `ordered_edges: 8`, `parallel_inspect_tracks_independent: True`, `implementation_depends_on_parallel_inspect_tracks: True`, `validation_node_has_pytest_result: True`.
- Real E2E natural-user: pass, report `D:\whalecode-alpha\target\real-user-e2e\action-map-natural-user-order-pipeline\20260605-120744-876\artifacts\report.md`; key metrics include `prompt_leaks_internal_concepts: False`, `final_output_leaks_internal_concepts: False`, `nodes: 6`, `agents: 2`, `pytest_owned_by_validation_node: True`, `hidden_oracle_exit_code: 0`.
- File-size check: `run-action-map-growth-health-e2e.ps1` 460 lines, `run-action-map-natural-multi-agent-e2e.ps1` 387 lines, `run-action-map-real-user-e2e.ps1` 386 lines, `action-map-real-user-e2e-lib.ps1` 384 lines.

### Closure

Round 3 accepted blocking findings were fixed. Final fresh closure review completed.

## Round 4

### Final Closure Review Launch Record

#### Reviewer G

- Role: final closure reviewer for Round 3 accepted blocking fixes
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e95fb-e9be-7ef1-a848-fb05b3494d9d`
- Nickname: `Socrates`
- Agent type: `explorer`
- Model context inheritance: `fork_context=false`; fresh session with only navigation packet
- Read-only instruction: yes
- Context explicitly excluded: full chat history, main-agent reasoning, patch persuasion brief

Navigation packet:

```text
Objective: Final fresh closure review for accepted blocking fixes in TaskSpace/action-map runtime and E2E harness in D:\whalecode-alpha. Read-only. Do not modify files. Start from this packet only; do not inherit main-agent context.

Specific closure questions:
1. Does completed final_synthesis final-response handling now run the full final gate, not just hidden-term checking?
2. Does the completed-final exclusion only ignore the final synthesis result itself, while still rejecting unreviewed non-final results?
3. Does hidden-term matching now catch node-1, map-1, task-1, task map, delegating, multi agent, split agents, fan out, and scheduled-agent phrases?
4. Does it avoid obvious false positives for ordinary map verb and standalone concurrent/parallel code terms?
5. Are tests and latest validation reports sufficient for closure?
```

### Reviewer Output

Blocking:

- None.

Closure answers:

- Completed `final_synthesis` final-response handling now calls shared `validate_final_response_ready(..., true)`.
- The shared gate reruns cognitive preflight, lifecycle result review, broad-debt check, post-edit validation, and hidden-term checks.
- Completed-final exclusion ignores completed `FinalSynthesis` node results, and runtime regression tests prove later unreviewed non-final results still block final response.
- Hidden-term detection covers ID forms, task/map/node concepts, delegation variants, multi-agent variants, fan-out/split wording, and scheduled-agent phrases.
- Boundary-aware matching and scheduled-agent phrase logic avoid obvious false positives for ordinary `map` verbs and standalone `concurrent/parallel` code terms.
- Latest cargo regression, script self-tests, and three real E2Es are sufficient for closure.

Non-blocking:

- Completed-final exclusion is node-scoped, not result-id-scoped. Current runtime transitions make this acceptable for normal flow, but result-id scoping would be tighter if restored or synthetic states become important.
- Final-gate rejection is enforced after streaming completion. Turn completion clears the final message and forces follow-up, but structured final-answer-gate rejection events remain future observability work.
- Optional tests could add explicit table cases for `concurrent agents` and `simultaneous evidence tracks`.

### Final Closure

Closed with no unresolved blocking findings.
