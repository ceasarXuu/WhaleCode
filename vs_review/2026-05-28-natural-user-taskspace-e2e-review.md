# Subagent VS Review: Natural User TaskSpace E2E

- Created: 2026-05-28T14:30:00+08:00
- Task: Design and validate a natural-user E2E for TaskSpace map growth.
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1

### Review Input

Review `scripts/run-action-map-real-user-e2e.ps1`, the natural-user design doc, and the then-latest run artifacts under:

```text
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-143702-106/artifacts/
```

Challenge whether the prompt is truly natural, whether the test validates mechanism effectiveness rather than no-crash behavior, whether the observed failure is valid, and what false pass/fail risks remain.

### Reviewer Launch Record

| Reviewer | Mechanism | Session ID | Context Forked | Read-only |
|---|---|---|---|---|
| natural-user-e2e-adversary | `multi_agent_v1.spawn_agent` explorer | `019e6d4f-c819-7cd3-942c-22a3937d3e78` | no | yes |

### Reviewer Output

Blocking findings:

- The natural-user failure is substantively valid: the agent fixed the business problem but created only coarse nodes, so the run does not prove healthy TaskSpace decomposition.
- The validation-node check was mixed: pytest did run and pass, but it was not owned by a validation node. The report needed to distinguish business validation from TaskSpace ownership.

Non-blocking risks:

- Raw `command_execution` counting over-counted started/completed events.
- Prompt guard is literal and can false fail if a natural prompt uses words such as "map out".
- Hidden oracle is useful but does not by itself prove TaskSpace effectiveness.

Required fixes:

- Split `agent_ran_passing_pytest` from `pytest_owned_by_validation_node`.
- Add ordering assertion so ordinary tools cannot appear before successful task/node binding.
- Count unique completed command executions.
- Report enough details to audit pytest ownership.

### Main Agent Response

- Accepted. Updated `scripts/run-action-map-real-user-e2e.ps1` to split agent-run pytest from validation-node ownership.
- Accepted. Replaced raw command count with unique completed command executions and failed command count.
- Accepted. Added an ordering check and later hardened it to use successful `lease_created` evidence instead of a mere `taskspace_control` request.
- Accepted. Added pytest owner details: node id/title, result id, call id, and command arguments.
- Accepted. Kept the semantic-node threshold strict; the test should fail when the agent only creates coarse workflow nodes.

Validation after response:

```text
.\scripts\run-action-map-real-user-e2e.ps1 -PlanOnly -WhaleBin 'D:\whalecode-alpha\target-test\debug\whale.exe'
.\scripts\run-action-map-real-user-e2e.ps1 -WhaleBin 'D:\whalecode-alpha\target-test\debug\whale.exe' -TimeoutSeconds 1200
```

Latest real run:

```text
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-150009-462/artifacts/report.md
```

Result: `FAIL`, intentionally. Business validation passed, hidden oracle passed, task/node binding happened before ordinary tools, and pytest ownership details were recorded. The remaining failures are TaskSpace effectiveness gaps: too few nodes, incomplete semantic coverage, and previous node left `ready`.

## Round 2

### Review Input

Review the updated script, updated design doc, latest report, latest observability, and latest natural prompt:

```text
scripts/run-action-map-real-user-e2e.ps1
docs/plans/2026-05-28-natural-user-taskspace-e2e.md
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-144604-123/artifacts/report.md
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-144604-123/artifacts/action-map-observability.md
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-144604-123/artifacts/user-prompt.txt
```

Challenge prompt naturalness, pytest ownership split, ordering check, latest failure validity, command metrics, node status, title coverage, and doc accuracy.

### Reviewer Launch Record

| Reviewer | Mechanism | Session ID | Context Forked | Read-only |
|---|---|---|---|---|
| closure-reviewer | `multi_agent_v1.spawn_agent` explorer | `019e6d58-fa75-7f33-91c0-cc0cfd737d68` | no | yes |

### Reviewer Output

Blocking findings:

- None against the E2E asset itself.
- The latest `FAIL` remains valid for TaskSpace acceptance: only the final node completed, and semantic titles lacked parser/pricing coverage.

Non-blocking risks and required hardening:

- Ordering should use successful binding evidence rather than the existence of a `taskspace_control` function call.
- Report should include concrete pytest ownership details, not just booleans.
- Observability markdown still lacks command previews, so deep manual audit requires opening `rollout.jsonl`.

### Main Agent Response

- Accepted ordering hardening. The script now uses the first `lease_created` event as the successful binding timestamp.
- Accepted ownership reporting. The report now includes `pytest_owner_node_id`, `pytest_owner_node_title`, `pytest_owner_result_id`, `pytest_owner_call_id`, and `pytest_owner_command`.
- Deferred richer observability markdown command previews to the viewer/export layer. The E2E report and rollout artifacts now include enough proof for this test.
- Reran the natural-user E2E after hardening. Latest report:

```text
target/real-user-e2e/action-map-natural-user-order-pipeline/20260528-150009-462/artifacts/report.md
```

The test still fails for valid mechanism reasons, not because the E2E asset is broken.

## Closure Status

- Blocking findings against the test asset: none remaining
- Accepted hardening changes implemented: yes
- Fresh independent review used: yes
- Latest real E2E executed: yes
- Latest E2E result: FAIL by design, exposing current TaskSpace effectiveness gaps
- Allowed to proceed: yes
