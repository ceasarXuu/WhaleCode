# Subagent VS Review: TaskSpace Natural E2E Graph Control

- Created: 2026-05-30T14:20:00+08:00
- Updated: 2026-05-30T15:25:00+08:00
- Report schema: adversarial-v1
- Task: Fix TaskSpace E2E design so natural users do not prompt parallelism, while runtime still drives multi-agent node growth and graph edges.
- Report path: `vs_review/2026-05-30-taskspace-natural-e2e-graph-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: Natural E2E And Runtime Gate Review

### Review Input

Objective: verify that the TaskSpace changes address two critiques:

1. Natural user E2E must not ask the user to request parallel investigation.
2. E2E evidence must validate node relationships, dependency edges, and execution order, not only node count.

Review target:

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `scripts/run-action-map-growth-health-e2e.ps1`
- `scripts/run-action-map-natural-multi-agent-e2e.ps1`
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/export-action-map-observability.ps1`
- `docs/plans/2026-05-30-taskspace-e2e-correction.md`
- prior E2E reports under `target/real-user-e2e/`

Risk focus:

- Green E2E might still depend on model luck rather than runtime structure.
- Default dependency inference might over-connect unrelated nodes or hide poor planning.
- Graph health might be too weak, too scenario-specific, or confuse direct and transitive dependency.
- Final answer could leave the final map node running.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | The largest risk is a green E2E that does not prove natural multi-agent graph behavior. | requirements, testing, observability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e7789-d837-7662-8695-4ccd8111288b` / Locke | current Codex thread spawn record | false | Round 1 packet | main-agent history, reasoning, drafts, conclusions | yes |

### Reviewer Output Summary

Blocking findings accepted from Round 1:

1. The natural E2E still did not validate enough graph relationships and ordering.
2. A run could pass while leaving a `final_synthesis` node open after the assistant had already answered.
3. Graph-health checks were mixing semantic direct dependencies with reachable/transitive dependencies.
4. Scenario-specific title checks could turn a valid map into a false failure or a weak map into a false pass.

### Main Agent Response

- `accept`: E2E graph checks were expanded to validate edge count, ordered edge count, order violations, parallel inspect tracks, independence between those tracks, direct implementation dependencies, direct validation dependency, and open terminal node count.
- `accept`: runtime now records the final assistant message into the current running `final_synthesis` node, releases the main lease, and marks the node completed.
- `accept`: graph health now reports direct and transitive dependency checks separately.
- `accept`: growth E2E no longer requires specific parser/pricing titles to carry subagent results; it validates the stronger general constraint that subagent-owned inspect tracks exist, hold results, remain independent, and feed implementation directly.
- `accept`: blocked leaf nodes are no longer classified as open leaves because `blocked` is a recorded terminal state, not an unfinished lease.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review required: yes
- Blocking re-review status: running

## Round 2: Blocking Closure Review

### Review Input

Objective: independently verify the closure fixes after Round 1.

Review target:

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `scripts/run-action-map-natural-multi-agent-e2e.ps1`
- `scripts/run-action-map-growth-health-e2e.ps1`
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/test-action-map-graph-health.ps1`
- `scripts/export-action-map-observability.ps1`
- `docs/plans/2026-05-30-taskspace-e2e-correction.md`
- `target/real-user-e2e/action-map-natural-multi-agent-order-pipeline/20260530-151327-074/artifacts/report.md`
- `target/real-user-e2e/action-map-growth-health-order-pipeline/20260530-152220-801/artifacts/report.md`

Verification status:

- `scripts/test-action-map-graph-health.ps1`: PASS.
- `scripts/run-action-map-natural-multi-agent-e2e.ps1`: PASS, thread `019e77bb-0ea3-7c01-9f86-c945376573af`.
- `scripts/run-action-map-growth-health-e2e.ps1`: PASS, thread `019e77c3-3399-7493-abda-a62d342de363`.
- Installed binary used by both E2E reports: `C:\Users\77585\.whale\bin\whale.exe`, SHA256 `CCEF6DFED3550F06C3A0CAAAA7277BC0FD16168FAE1C5F3CC7098E8F189268FF`.

Risk focus:

- Prompt guard could still allow explicit user parallel encouragement.
- Runtime might still rely on prompt-only behavior instead of enforceable gates/default edges.
- Graph-health checks might miss bad graphs or confuse direct vs transitive edges.
- Final response auto-completion might close the wrong node or leave state split.
- E2E might not really call real Whale, real tools, real subagents, and real pytest.

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| closure-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e77c6-7bef-7fc3-9be8-bc652fb81336` / Nietzsche | current Codex thread spawn record | false | Round 2 packet | main-agent history, reasoning, drafts, conclusions | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round-2-output | closure-adversary | 1 | `019e77c6-7bef-7fc3-9be8-bc652fb81336` | completed | completed | no blocking findings | record output and close |

### Reviewer Outputs

#### Summary

The reviewer found no blocking issue. The checked prompts do not explicitly request parallelism, runtime now provides system-side collaboration pressure plus graph/default-edge gates, and E2E validates edges, direct dependencies, order, final closure, real `whale.exe`, real subagent leases, real commands, and real `pytest`.

#### Blocking Findings

None.

#### Non-Blocking Risks

1. Prompt guard caught obvious internal/parallel terms but not synonyms such as "simultaneously", "delegate", "fan out", or "multiple agents".
2. Multi-agent behavior is system-encouraged through developer context and budget/barrier logic, but not a universal hard guarantee for all under-budget tasks.
3. Graph health direct inspect dependency was checked against any implementation node, so a dead-end implementation could satisfy the check while the validated path used another implementation.
4. Snapshot export could overwrite an original node result timestamp with a later snapshot timestamp.

#### Verdict

No blocking findings. The fix is credible for the two stated claims in the E2E runs. The reviewer recommended tightening prompt guard and graph-health anchoring before treating the harness as reusable for more complex multi-implementation maps.

### Main Agent Response

- `accept`: prompt guard expanded to cover collaboration-strategy synonyms including `simultaneously`, `delegate`, `delegation`, `multiple agents`, `multi-agent`, `split ... agents`, and `fan out`.
- `defer`: V1 runtime intentionally proves "can drive healthy multi-agent TaskSpace behavior", not "must always use multiple agents for every under-budget task". Hard multi-agent guarantees require a separate product decision because simple tasks should still be cheap.
- `accept`: graph health now anchors implementation dependency checks to implementation nodes that can reach validation/final nodes; dead-end implementations no longer satisfy direct inspect dependency checks.
- `accept`: observability result updates preserve the first non-empty timestamp and only fill missing timestamps later.
- `accept`: E2E now distinguishes all failed collaboration tool calls from unexpected failed collaboration tool calls; a recovered stale spawn assignment remains visible but no longer fails the scenario.
- Added self-tests:
  - `scripts/test-action-map-graph-health.ps1`
  - `scripts/test-action-map-observability-lib.ps1`
  - `scripts/test-action-map-real-user-e2e-lib.ps1`
- Re-ran real E2E after the fixes:
  - `scripts/run-action-map-natural-multi-agent-e2e.ps1`: PASS, thread `019e77d3-0a47-75c2-ac30-7ab3e791d091`.
  - `scripts/run-action-map-growth-health-e2e.ps1`: PASS, thread `019e77d4-5797-7aa1-b136-71bf46a56e8c`.

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes, from Round 1
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, universal hard multi-agent guarantee deferred as a product policy decision
- Allowed to proceed: yes

## Final Conclusion

Closed. The revised TaskSpace E2E no longer depends on user-provided parallelism wording, validates dependency edges and execution order, verifies final node closure, and runs through real Whale CLI/subagent/tool/test paths.
