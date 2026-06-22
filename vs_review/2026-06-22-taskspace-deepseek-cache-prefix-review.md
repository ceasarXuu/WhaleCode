# Subagent VS Review: TaskSpace DeepSeek Cache Prefix

- Created: 2026-06-22T15:55:00+08:00
- Updated: 2026-06-22T16:10:00+08:00
- Task: Review the TaskSpace DeepSeek official API cache-hit-rate repair.
- Report path: `vs_review/2026-06-22-taskspace-deepseek-cache-prefix-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Post-implementation adversarial review

### Review Input

#### Objective
Verify whether the TaskSpace cache prefix repair correctly addresses low DeepSeek official API input-cache hit rate by preserving a stable provider prompt prefix before dynamic TaskSpace developer context.

#### Review Target
Code implementation, test strategy, and diagnostic documentation for commit `178e6c24c Improve TaskSpace prompt cache prefix stability`.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
- `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
- `cargo test -p codex-core build_initial_context_ --lib`

#### Change Introduction
The implementation collects TaskSpace transition notice and TaskSpace developer context early, but appends them after the stable realtime, personality, apps, skills, plugin, and commit developer sections. The intent is to keep large stable developer sections, especially skills instructions, ahead of frequently changing TaskSpace content so DeepSeek prefix caching can reuse more input tokens. A regression test asserts skills instructions precede TaskSpace context in initial context when TaskSpace experiment mode and repo skills are enabled.

#### Risk Focus
- Whether moving TaskSpace developer context later changes model behavior or weakens TaskSpace instruction priority.
- Whether the fix only optimizes the initial context while later per-turn updates still break provider prefix caching.
- Whether the regression test proves the intended cache-stability invariant rather than only checking one narrow string order.
- Whether transition notices are still consumed once and delivered in the intended turn.
- Whether the COE and validation evidence distinguish structural proof from live provider cache-hit proof.

#### Verification Status
- `cargo fmt` completed.
- `cargo test -p codex-core build_initial_context_keeps_stable_skills_before_taskspace_context --lib` passed.
- `cargo test -p codex-core build_initial_context_consumes_action_map_transition_notice_once --lib` passed.
- `cargo test -p codex-core build_initial_context_ --lib` passed with 10 tests.
- Live DeepSeek benchmark rerun was not performed because recent provider calls hit `402 Payment Required`; runtime cache-hit validation remains pending after balance recovery.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return: summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs or observability, and evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | The change reorders developer context sections and may affect prompt priority and long-term context architecture. | Prompt ordering, module boundaries, cache invariant validity |
| test-validity-adversary | The repair depends on a regression test and provider cache behavior that cannot be directly validated until DeepSeek billing is restored. | Test strength, false confidence, missing black-box checks |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019eee64-9d4b-71c0-8123-7f9b9d3fc643` (`Boole`) | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless directly inspected from repo | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` | `019eee64-e805-7d20-9a49-6280046feb05` (`Zeno`) | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless directly inspected from repo | yes |

### Reviewer Outputs

#### architecture-adversary

##### Summary
The repair is directionally correct for the specific initial-context ordering bug: `build_initial_context` now captures the TaskSpace transition notice/context early but appends it after realtime, personality, apps, skills, plugins, and commit guidance. The regression test verifies the highest-value stable block, `<skills_instructions>`, precedes TaskSpace context.

The reviewer did not see a blocking correctness regression. The fix is not yet proven as a full cache-hit repair because steady-state per-turn TaskSpace projection updates still create changing developer items before each provider request, and live provider validation remains unavailable.

##### Blocking Findings
- None.

##### Non-blocking Risks
- TaskSpace instruction priority may be slightly weaker by order, but not by role. TaskSpace context is still in the aggregated developer message, but now appears after skills/plugins/commit guidance in `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3894-3900`.
- This repair mainly fixes the initial-context stable prefix shape, not all later prompt churn. Steady-state context injection still appends fresh TaskSpace developer context in `third_party/codex-cli/codex-rs/core/src/session/mod.rs:4008-4017`, and each sampling loop still rebuilds/removes/re-records a TaskSpace projection before constructing provider input in `third_party/codex-cli/codex-rs/core/src/session/turn.rs:498-512`.
- COE status slightly overstates "resolution" versus "structural repair." E-007/E-008 prove code/test structure only, not post-repair DeepSeek hit rate.

##### Required Fixes
- None required before acceptance from this review.

##### Missing Tests
- Add a full initial-context ordering test that checks TaskSpace appears after apps/plugins/commit guidance, not only after skills.
- Add a prompt-shape regression for steady-state: initial stable sections remain before the first dynamic TaskSpace projection after `record_context_updates_and_set_reference_context_item`.
- Add a transition notice order test.

##### Missing Logs / Observability
- No post-repair live provider cache-hit proof yet.
- Diagnostic artifacts should record stable-prefix length or first-dynamic-section offset.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3808-3815` - TaskSpace transition/context are collected before append.
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3840-3893` - stable sections are appended before TaskSpace.
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3894-3900` - TaskSpace append point is after stable developer sections.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:498-506` - later per-turn TaskSpace projection replacement still exists.
- `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md:197-219` - structural repair and tests are documented, but live provider validation is not.

#### test-validity-adversary

##### Summary
The implementation direction is sound: `build_initial_context` now collects TaskSpace transition/context early but appends them after realtime/personality/apps/skills/plugins/commit developer sections, so the initial developer bundle has a longer stable prefix before TaskSpace text appears.

##### Blocking Findings
- None found for the code change itself.

##### Non-blocking Risks
- The new regression test proves only one narrow order: `<skills_instructions>` appears before `"TaskSpace mode is now active"` in one initial-context setup. It does not prove a stable provider-prefix invariant across apps/plugins/no-skills permutations or serialized provider payloads.
- The fix addresses initial context order, but TaskSpace projections are still rebuilt and recorded immediately before each sampling request in the turn loop. That may be correct because they are appended after prior stable history, but no test measures the resulting repeated-request prefix.
- The COE wording says the repair "directly addresses H-001" and that the invariant is covered, but live cache-hit improvement remains unvalidated.

##### Required Fixes
- No mandatory code fix from this read-only review.
- Before calling this "validated" for cache behavior, tighten the COE language from "new cache-order invariant is covered" to "initial skills-before-TaskSpace ordering is covered; provider cache-hit validation remains pending."

##### Missing Tests
- Add a provider-visible prompt/payload test that builds two TaskSpace requests and asserts the common prefix includes stable developer sections before the first TaskSpace projection difference.
- Add parameterized initial-context order tests for no skills, apps enabled, plugins enabled, and commit guidance enabled.
- Strengthen the transition notice test to count occurrences, not just `contains` on first call and absence on second call.
- Add a repeated initial-context test that confirms the transition notice is consumed once while the non-notice TaskSpace developer context still appears in the expected late position.

##### Missing Logs / Observability
- No diagnostic currently records stable-prefix length, first-difference offset, or section ordering for provider payloads.
- Live DeepSeek validation is an acceptable pending gap because provider calls were blocked by 402, but it should stay explicitly pending and not be treated as resolved by unit tests alone.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3808` - TaskSpace context is collected early.
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3894` - TaskSpace context is appended after stable sections.
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs:1334` - current regression test covers one initial-context ordering case.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:498` - repeated request projection path still exists.
- `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md:197` - repair documentation before wording correction.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| architecture-adversary | No blocking findings. | n/a | accept | Both reviewers independently found no blocking correctness issue. | No code change required. | n/a |
| architecture-adversary | TaskSpace instruction priority may be slightly weaker by order. | minor | defer | TaskSpace remains in the same developer message and still uses developer role priority; no concrete conflict was found. | Recorded as a non-blocking risk. | Revisit if TaskSpace workflow compliance regresses in live runs. |
| architecture-adversary | Repair mainly fixes initial-context stable prefix, not all later prompt churn. | major | defer | `session/turn.rs:498-506` still refreshes TaskSpace projection; this is outside the immediate confirmed initial-prefix bug and needs live cache evidence. | Recorded as follow-up. | Add provider-visible common-prefix diagnostics and rerun DeepSeek benchmark after balance recovery. |
| architecture-adversary | COE overstates resolution versus structural repair. | major | accept | COE E-007/E-008 proved structural order only, not live provider recovery. | Updated COE conclusion, E-007, E-008, and added E-009. | Live provider validation remains pending. |
| architecture-adversary | Additional initial/steady-state/order tests are missing. | major | defer | Current tests cover the direct regression and one-shot notice behavior; broader prompt-shape tests are useful but not required to close this no-blocking review. | Recorded as follow-up. | Track with future cache observability work. |
| architecture-adversary | Stable-prefix length or first-dynamic-section offset is not logged. | major | defer | Existing diagnostics proved hashes and cache tokens, but not reusable-prefix offset. This is useful observability, not required for the structural repair. | Recorded as follow-up. | Add benchmark artifact fields for stable prefix length and first TaskSpace offset. |
| test-validity-adversary | Regression test covers one narrow order case. | major | defer | The current test directly covers the confirmed broken order from the artifact: skills after TaskSpace. Broader permutations are future hardening. | Recorded as follow-up. | Add provider-visible prompt common-prefix tests. |
| test-validity-adversary | COE wording should distinguish structural validation from cache validation. | major | accept | Same as architecture finding. | Updated COE language and evidence. | Live provider validation remains pending. |
| test-validity-adversary | Transition notice test should count occurrences and repeated context position. | minor | defer | Existing test proves one-shot consumption; counting occurrences would improve specificity. | Recorded as follow-up. | Add when expanding prompt-shape tests. |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

The adversarial review passed with no blocking findings. The main corrective action was documentation: the COE now distinguishes the locally tested structural prompt-order repair from the still-pending live DeepSeek cache-hit validation. Broader provider-visible prefix tests and stable-prefix diagnostics are deferred follow-up items.
