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

Terminology is intentionally split:

- `e2_evidence_readiness` means paired runs are comparable and meet the E2 evidence gate.
- `e2_clean_readiness` means E2 mechanism warnings are clean: no non-E2 reports, no scenario warning pairs, and all required levels are covered.
- `e2_utility_clean_readiness` means the mechanism is clean and no pair exceeded the configured TaskSpace cost warning thresholds.

E2 is a constructed-regression mechanism target, not an E3 utility-superiority proof. A report may therefore show `e2_clean_readiness: True` while `e2_utility_clean_readiness: False`; that is an honest E2 mechanism pass with visible cost drag, not a utility claim.

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

### Product Contract Alignment

The full matrix exposed one L2 failure where TaskSpace passed the public tests but failed the hidden oracle. The scenario intentionally had a product-rule conflict:

- README said Premium customers receive 10 percent off.
- An existing public invoice test expected the old fixed `$10` behavior.

The failed TaskSpace run trusted the existing public test and preserved the stale implementation. The standard paired run updated the invoice test and implementation to match README, which matched the hidden oracle.

TaskSpace methodology now explicitly requires inspect nodes to reconcile README/spec docs, tests, and implementation before editing. If explicit product rules conflict with existing tests, the agent should treat those tests as potentially stale, update code and tests together, and record the rationale. This is a general coding rule, not a benchmark-specific answer key.

### Completion Evidence Hardening

Adversarial review found two real closure gaps after the first clean matrix run:

- Subagent final messages could complete typed implementation/validation nodes without matching edit/test/build evidence.
- Tool success was inferred from result text, so a failed tool preview containing `success: true` could fake completion evidence.

The runtime now stores tool success as structured result metadata and applies the same typed completion evidence rule to subagent-owned nodes. If a subagent reports completion without the required evidence, the node is recorded as blocked instead of completed. This keeps the node context available for main-agent recovery while preserving the existing node status and result mechanisms.

The structured success flag is also included in the task snapshot so context restore does not degrade completion evidence after compaction or session recovery.

### Low-complexity Main-agent Continuation

The hardened full matrix exposed a remaining L1 cost warning: after a broad inspect node reached the tool-result budget, the runtime continued to block main-agent edits even after the agent had finished inspect and bound a concrete implementation node. That forced a subagent for a one-line fix.

The broad-inspect delegation guard now applies only while the current main node is still an inspect_code_context node. Once the map has moved to an explicit implement_solution, smoke_test, regression_test, or final_synthesis node, the main agent can continue normally under that node's own contract and evidence gate. This preserves the anti-sprawl guard for ongoing broad investigation without making simple fixes pay unnecessary coordination cost.

### Single-track Inspect Delegation Guard

The next L1 rerun showed a different low-complexity failure: the agent sometimes created an additional inspect node and spawned an explorer only to read one known test file. That is not parallel investigation; it is single-track outsourcing and adds coordination cost without reducing context pressure.

Runtime now treats `inspect_code_context` subagent spawn as a parallel investigation mechanism:

- If the main agent is already holding a running inspect track, one extra ready inspect node is not enough for `spawn_agent`; the main agent must either finish its current inspect node or create at least two ready independent inspect nodes and act as coordinator. The exception is maintenance-barrier recovery: when the current inspect node is barriered for broadness, assigning a different ready inspect node remains a valid recovery path.
- If a completed narrow inspect node already exists, a single serial follow-up inspect node is not assigned to an explorer; the main agent should finish that known-file or known-path follow-up itself.
- If at least two independent inspect tracks exist, the first explorer can claim one ready inspect node and the second explorer can still claim the remaining ready node while the first lease is active.
- Non-inspect ready nodes keep the existing ready-node-only assignment behavior.

The TaskSpace prompt and BaseMap metadata also make the simple-read rule explicit: path correction, re-reading, and one known-file follow-up reads stay inside the current inspect node. The runtime still does not guess semantic task complexity; it prevents mechanically bad serial outsourcing patterns while preserving initial ready-node assignment and real parallel inspect groups.

## Tests

Required focused tests:

- Rust unit test that TaskSpace developer context contains the minimal-sufficient-map rule.
- Rust unit tests that reject completing implementation/test nodes without matching successful tool evidence.
- Rust unit test that rejects handing off the current main-held node to a subagent.
- Rust unit tests that reject serial single-track inspect spawn after a completed narrow inspect and while the main agent already holds a running inspect node, while still allowing a two-track inspect group to assign both explorers.
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
- full E2 matrix reports utility cost gaps separately; `e2_utility_clean_readiness` may remain false without invalidating the E2 mechanism pass.

If the final item is false, the implementation still counts as progress, but the conclusion must say clean E2 was not reached and identify the remaining warning source.
