# Problem P-001: TaskSpace action-contract control ABI mismatch
- Status: fixed
- Created: 2026-06-25 15:30
- Updated: 2026-06-25 15:57
- Objective: Fix cache-optimized TaskSpace action-contract control actions so lifecycle commands compile into valid native `taskspace_control` arguments before execution.
- Symptoms:
  - Phase B rerun45 TaskSpace side produced `taskspace_control_count=0` and did not patch `src/tax_calc.py`.
  - stderr contained two `failed to parse function arguments: missing field 'action'` failures.
- Expected behavior:
  - A `taskspace-action-v1` `taskspace_control` action must become a valid native `taskspace_control` tool payload with a top-level `action` discriminator.
- Actual behavior:
  - Action-contract `args.action_name=start_task` and `args.command=finish_node` reached the native handler without a canonical top-level `action`.
- Impact:
  - DeepSeek cache-optimized TaskSpace transport can keep high provider cache hit rate while still failing the lifecycle state machine.
- Reproduction:
  - Parse rerun45 assistant JSON objects with `args.action_name=start_task` or `args.command=finish_node`, then convert them through action-contract transport into `taskspace_control`.
- Environment:
  - Windows, branch `whalecode-alpha`, TaskSpace `cache_optimized_action_contract` transport after Phase B request-phase attribution.
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
- Ruled out:
  - DeepSeek provider cache miss as primary cause for this symptom, based on rerun45 cache summary in E-002.
- Fix criteria:
  - Unit tests cover rerun45 `action_name` and `command` aliases.
  - Missing control action is rejected before the native handler with a stable error code.
  - Focused action-contract tests pass.
  - A live or smoke TaskSpace run no longer shows this failure mode.
- Current conclusion: H-001 is fixed by canonical action-contract control argument compilation, handler-level alias normalization, and action-contract-aware control usage metrics.
- Related hypotheses:
  - H-001
- Resolution basis:
  - H-001, E-003, E-004, E-005
- Close reason:
  - focused tests, build, and B-tier smoke validation passed.

## Hypothesis H-001: Control action aliases bypass the action-contract compiler boundary
- Status: confirmed
- Parent: P-001
- Claim: The action-contract transport lacks a unified canonicalization step for `taskspace_control` lifecycle action aliases, so model-valid intent can be serialized into native tool payloads without the required serde tag.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The native `TaskSpaceControlArgs` enum is serde-tagged by `action`, while the action-contract path directly forwards or only partially normalizes `args`.
- Falsifiable predictions:
  - If true: code will show `taskspace_control` payload creation from action-contract `args` without supporting `action_name` and `command`, and rerun45 outputs will use exactly those fields.
  - If false: the action-contract path already normalizes these aliases before handler invocation, or rerun45 failures come from a different parse layer.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare rerun45 JSON with the action-contract conversion code and native handler discriminator.
  - Signal: source code and archived run artifacts.
  - Capture method: inspect `turn.rs`, `taskspace_control.rs`, and rerun45 artifacts.
  - Event name or marker:
    - `missing field 'action'`
  - Correlation keys:
    - `target/phase-b-complete-B-rerun45/single-file-fast-fix/20260624-203208-963`
  - Differentiates from:
    - provider cache instability
  - Supports if:
    - aliases appear in rerun45 and are not canonicalized by the action-contract tool-call compiler.
  - Refutes if:
    - aliases are canonicalized before native handler invocation.
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain stable compiler error codes as permanent observability.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
- Conclusion: confirmed
- Repair design readiness: satisfied
- Next step: monitor subsequent B/C reruns for semantic quality and remaining TaskSpace utility issues.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Native handler requires top-level action discriminator
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- Prediction or plan link:
  - H-001 predicts native `taskspace_control` serde requires a top-level `action`.
- Matched signal:
  - `#[serde(tag = "action", rename_all = "snake_case")] enum TaskSpaceControlArgs`
- Correlation keys:
  - none
- Raw content:
  ```text
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TaskSpaceControlArgs
  ```
- Interpretation: Any native payload missing top-level `action` fails before lifecycle logic can run.
- Time: 2026-06-25 15:30

## Evidence E-002: Rerun45 used non-canonical aliases while cache remained healthy
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `target/phase-b-complete-B-rerun45/single-file-fast-fix/20260624-203208-963`
- Prediction or plan link:
  - H-001 predicts rerun45 outputs used aliases not normalized by the action-contract compiler.
- Matched signal:
  - Assistant JSON contained `args.action_name=start_task` and `args.command=finish_node`; stderr reported missing `action`.
- Correlation keys:
  - `20260624-203208-963`
- Raw content:
  ```text
taskspace_control_count=0
state_commit_count=0
stderr: failed to parse function arguments: missing field 'action'
provider cache request_2_plus_hit_rate=0.991148
native_tools_schema_hot_path_count=0
tool_free_action_contract_count=7
  ```
- Interpretation: The immediate failure is an action-contract ABI mismatch, not loss of DeepSeek cache transport.
- Time: 2026-06-25 15:30

## Evidence E-003: Focused ABI and handler regression tests pass
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-core taskspace_action_contract --lib`; `cargo test -p codex-core taskspace_control --lib`
- Prediction or plan link:
  - H-001 fix criteria require rerun45 alias shapes to compile before native handler invocation and handler aliases to remain parseable.
- Matched signal:
  - New tests cover `action_name=start_task`, `command=finish_node`, missing discriminator rejection, alias conflict rejection, and native handler alias parsing.
- Correlation keys:
  - none
- Raw content:
  ```text
taskspace_action_contract: 26 passed; 0 failed
taskspace_control: 31 passed; 0 failed
  ```
- Interpretation: The deterministic rerun45 ABI failure is now covered at both action-contract compiler and native handler boundaries.
- Time: 2026-06-25 15:51

## Evidence E-004: Built whale and completed B-tier smoke without missing action failure
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `target/action-contract-repair-B-rerun46/single-file-fast-fix/20260625-154751-205`
- Prediction or plan link:
  - H-001 fix criteria require a smoke TaskSpace run to avoid the original `missing field 'action'` failure while preserving cache-optimized transport.
- Matched signal:
  - TaskSpace side solved the benchmark, validation passed, and stderr did not contain `missing field 'action'`.
- Correlation keys:
  - `20260625-154751-205`
- Raw content:
  ```text
cargo build -p codex-cli --bin whale: Finished dev profile
right/taskspace business_success=True
right/taskspace public_validation_exit_code=0
right/taskspace hidden_oracle_exit_code=0
right/taskspace changed_paths include src/tax_calc.py
validation.stdout.log: 3 passed in 0.03s
provider cache request_2_plus_hit_rate=0.988546
native_tools_schema_hot_path_count=0
tool_free_action_contract_count=8
  ```
- Interpretation: The repair did not regress canonical action-contract execution or DeepSeek cache shape; this B-tier sample no longer exhibits the original failure signature.
- Time: 2026-06-25 15:50

## Evidence E-005: Control usage metrics now count action-contract lifecycle actions
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`; `scripts/taskspace-benchmark/test-harness.ps1`
- Prediction or plan link:
  - H-001 observability criteria require `taskspace_control_count=0` to stop hiding action-contract lifecycle transitions.
- Matched signal:
  - Recomputed B-tier right-side usage reports two action-contract lifecycle controls and no native controls.
- Correlation keys:
  - `20260625-154751-205`
- Raw content:
  ```text
test-cost-instrumentation.ps1: cost instrumentation selftest passed
test-harness.ps1: TaskSpace benchmark harness self-test: PASS
recomputed right/taskspace-control-usage.json:
taskspace_control_count=2
native_taskspace_control_count=0
action_contract_taskspace_control_count=2
action_counts.start_task=1
action_counts.finish_node=1
  ```
- Interpretation: Future runs can distinguish cache-optimized action-contract lifecycle transitions from native tool calls instead of reporting a misleading zero.
- Time: 2026-06-25 15:57
