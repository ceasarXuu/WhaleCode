# Subagent VS Review: Action Map Growth Health E2E

- Created: 2026-05-28T04:20:00+08:00
- Task: Design and implement a complex real-user E2E for Action Map growth health.
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1

### Review Input

Review `scripts/run-action-map-growth-health-e2e.ps1`, the design doc, and the then-latest run artifacts. Challenge whether the E2E is a real-path test and whether it can catch false healthy map growth.

### Reviewer Launch Record

| Reviewer | Mechanism | Session ID | Context Forked | Read-only |
|---|---|---|---|---|
| e2e-test-adversary | `multi_agent_v1.spawn_agent` explorer | `019e6b27-9a02-72b0-ae11-38eb95e6a921` | no | yes |

### Reviewer Output

Blocking findings:

- Subagent-node checks could pass when a node had a subagent lease and any result, even if that result was written by the main agent.
- Latest evidence was stale against the current script: prompt and `finish_node` metric did not match the updated script.

Non-blocking findings:

- The scenario intentionally over-prompts the desired map shape. This is acceptable for a regression harness.
- Validation node ownership needed stronger evidence than a plain result count.

### Main Agent Response

- Accepted the subagent-result finding. Updated parser/pricing investigation assertions to require a result whose `sourceThreadId` matches the node's attached `agentThreads`.
- Accepted stale evidence finding. Regenerated current real runs and added `script_sha256` to reports.
- Accepted validation ownership risk. Added validation-node ownership assertions, then strengthened them again in later rounds.
- Accepted business false-pass risk. Added a hidden oracle that directly imports final source and verifies README behavior independently of editable repo tests.

## Round 2

### Review Input

Fresh closure review of `scripts/run-action-map-growth-health-e2e.ps1`, latest run `20260528-044138-673`, and related observability. Focus on whether Round 1 blockers were closed.

### Reviewer Launch Record

| Reviewer | Mechanism | Session ID | Context Forked | Read-only |
|---|---|---|---|---|
| closure-reviewer | `multi_agent_v1.spawn_agent` explorer | `019e6b2e-cdd9-70b1-8a28-984999dd20b0` | no | yes |

### Reviewer Output

Blocking finding:

- Validation-node closure was still under-enforced. The script accepted any validation-titled node with any `main_tool_call`; this could pass on unrelated commands.

### Main Agent Response

- Accepted. Reworked validation checks to correlate node result bodies with rollout `function_call` records through `call_id`.
- Required the validation node result body to contain a passing marker and the associated function call arguments to contain `python -m pytest tests -q`.
- Reran the real E2E. Intermediate run `20260528-044935-767` failed because the new detector initially looked for the command text inside the result body; this exposed that command text lives in `function_call.arguments`, while result body stores only `call_id` and output.
- Fixed the detector to build a `call_id -> arguments` map from rollout JSONL and then validate ownership by joining through `call_id`.

## Round 3

### Review Input

Fresh closure review of the remaining validation-node command ownership issue using latest run `20260528-045550-749`.

### Reviewer Launch Record

| Reviewer | Mechanism | Session ID | Context Forked | Read-only |
|---|---|---|---|---|
| closure-reviewer | `multi_agent_v1.spawn_agent` explorer | `019e6b3a-cb07-7ba3-b9df-123a6e74dbe9` | no | yes |

### Reviewer Output

Blocking Findings: None.

Evidence cited by reviewer:

- Script builds `$toolCallArgs` from rollout function calls and joins validation node results by `call_id`.
- Latest report `20260528-045550-749` is PASS.
- `validation_node_has_pytest_result: True`.
- Rollout shows the validation node result uses the same `call_id` as a `shell_command` whose arguments contain `python -m pytest tests -q`, and the result body contains `7 passed`.

### Main Agent Response

- Accepted closure. No further blocking changes required.
- Deferred non-blocking observability improvement: markdown observability could include per-result command/call previews in a later viewer/export pass. The E2E already keeps the rollout and JSON artifacts needed for proof.

## Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Fresh closure review completed: yes
- Unresolved blocking findings: none
- Allowed to proceed: yes
