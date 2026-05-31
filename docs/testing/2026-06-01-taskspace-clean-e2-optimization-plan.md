# TaskSpace Clean E2 Optimization Plan

## Problem

The previous E2 matrix reached evidence readiness but not clean utility readiness:

```text
e2_evidence_readiness: True
e2_clean_readiness: False
utility_warning_gaps: single-file-fast-fix: warning_pairs_2
```

The L1 scenario succeeded functionally, but TaskSpace sometimes over-expanded a simple single-file repair by creating extra ready investigation work or calling `spawn_agent`. This is not a business failure, but it prevents a clean utility claim because simple tasks should not pay unnecessary coordination cost.

## Target

Keep the E2 evidence gate unchanged while improving TaskSpace behavior so:

- Simple work stays on a narrow main-agent task path.
- Multi-module work can still create independent investigation nodes and delegate to subagents.
- Reports continue to separate evidence readiness from clean utility readiness.
- If clean readiness remains false, the report must make the reason visible.

## Engineering Design

### Runtime Prompt Policy

TaskSpace runtime should inject a minimal-sufficient-map rule before the proactive parallelism rule:

- For simple single-file or single-failure tasks, prefer:
  `inspect_code_context -> implement_solution -> smoke_test/regression_test -> final_synthesis`.
- Do not create extra ready inspect nodes or call `spawn_agent` for simple work.
- Upgrade to multiple ready inspect nodes only when new evidence shows independent tracks that materially reduce risk or context load.

Runtime still does not decide semantic complexity. The agent decides, while runtime supplies a stronger operating policy.

### BaseMap Policy

BaseMap candidates are a decomposition menu, not a checklist. The prompt should say:

- Start with the minimum sufficient map.
- Simple tasks should not expand all candidate nodes.
- Multiple inspect nodes are for independent evidence tracks with distinct source surfaces.

### Evidence Policy

Do not weaken E2 gates to make reports pass. Keep:

- `e2_evidence_readiness`
- `e2_clean_readiness`
- warning pairs
- utility outcomes

The desired clean result is `e2_clean_readiness: True`, but the system must remain honest if it is false.

### Node Completion Evidence Gate

The first optimization pass showed that prompt-only policy is insufficient. In the L1 rerun, TaskSpace still inflated cost because a node could be completed by a free-form text result even when its required tool evidence was missing:

- `implement_solution` could claim "fixed" without any successful edit tool result.
- `smoke_test` could claim validation progress without any successful test/build result.

This is not a quality score and does not judge whether the solution is correct. It is a typed evidence gate that checks whether the node's declared work category has at least one matching successful recorded action before completion:

- `implement_solution` requires a successful `edit` action.
- `smoke_test` and `regression_test` require a successful `test` or `build` action.
- `inspect_code_context`, `final_synthesis`, and `custom` remain free-form at this layer.

If validation fails, the agent should not complete the test node as success. It should block the node or create/bind a follow-up implementation node. This keeps the graph honest and prevents later duplicate "repair" nodes caused by earlier fake completion.

### Ready-Node-Only Subagent Assignment

The L1 rerun after the evidence gate reduced node growth to the expected four-node chain, but one run still delegated the current implementation node to a subagent. That path is too permissive for clean low-complexity behavior and weakens the manager/coordinator model:

- A node already bound to the main agent is running work, not an available work item.
- A subagent should claim an explicit ready node, not inherit the main agent's half-started node.
- If the main agent wants a subagent to own work, it must create that node as ready with `bind_current=false`, or finish/block the current node first.

Runtime therefore rejects spawn assignment for nodes with an active lease, including the current main-held node. This keeps simple tasks on the main path and still permits real parallelism through explicitly created ready nodes.

### Inspect Budget Calibration

The next L1 rerun showed another false positive: a small repository repair can reasonably spend six inspect actions just to list files, read README/source/tests/config, and run one failing diagnostic test. The previous inspect split hint at six results treated this normal small-task exploration as broad investigation pressure and pushed the agent toward unnecessary delegation.

Raise the `inspect_code_context` main-tool split hint from 6 to 10. This does not disable delegation; it prevents the runtime from forcing split behavior before a simple task has enough room to finish ordinary context gathering. Broad tasks can still create independent ready nodes proactively from the injected methodology.

### Pre-Fix Diagnostic Test Placement

One remaining L1 warning came from modeling the expected failing baseline test as its own `smoke_test` node before the implementation node. That is methodologically too heavy for a simple bug fix. The intended BaseMap discipline is:

- Pre-fix failing tests used to prove or localize the bug are evidence gathering and belong in `inspect_code_context`.
- `smoke_test` and `regression_test` are post-implementation validation nodes.

The runtime and BaseMap prompts now state this explicitly so the common L1 path stays:

```text
inspect_code_context -> implement_solution -> smoke_test/regression_test -> final_synthesis
```

## Tests

Required focused tests:

- Rust unit test that TaskSpace developer context contains the minimal-sufficient-map rule.
- Rust unit tests that reject completing implementation/test nodes without matching successful tool evidence.
- Rust unit test that rejects handing off the current main-held node to a subagent.
- Benchmark harness self-tests for all three scenarios.
- Full E2 matrix after reinstalling the local Whale binary.

Required E2 matrix command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-e2-matrix.ps1 -Repeats 3 -Model deepseek-v4-flash -TimeoutSeconds 1200
```

## Acceptance

Clean E2 optimization succeeds if:

- all focused Rust tests pass,
- benchmark harness self-tests pass,
- full E2 matrix reports `e2_evidence_readiness: True`,
- full E2 matrix reports `e2_clean_readiness: True`.

If the final item is false, the implementation still counts as progress, but the conclusion must say clean E2 was not reached and identify the remaining warning source.
