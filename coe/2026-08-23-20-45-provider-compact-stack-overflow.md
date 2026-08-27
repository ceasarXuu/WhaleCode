# Problem P-001: Provider-aware previous-model compact stack overflow
- Status: fixed
- Created: 2026-08-23 20:45
- Updated: 2026-08-23 21:22
- Objective: explain and eliminate the stack overflow in the previous-model compaction compatibility regression without weakening provider-route correctness.
- Symptoms:
  - `cargo test -p codex-core previous_model` aborts in `pre_sampling_compact_keeps_unknown_previous_model_for_api_key_auth_and_custom_provider` with stack overflow.
- Expected behavior:
  - The compatibility test completes and keeps the unknown previous model on the custom provider path.
- Actual behavior:
  - The test process aborts with signal 6 after reporting a stack overflow.
- Impact:
  - Blocks validation and delivery of v0.0.6 provider-aware compaction changes.
- Reproduction:
  - From `third_party/codex-cli/codex-rs`, run `cargo test -p codex-core previous_model`.
- Environment:
  - Ubuntu 24.04 x86_64, Rust workspace on branch `whalecode-alpha`, 2026-08-23 working tree.
- Known facts:
  - The core/app-server compile checks and non-compaction provider tests pass.
  - The failure is deterministic in the filtered integration-test run.
  - The exact test also overflows alone with one test thread.
  - With a 32 MiB test-thread stack, execution reaches the final request-count assertion instead of overflowing.
- Ruled out:
  - unbounded recursion
  - filtered-test concurrency
  - full `ModelClientState` reconstruction as the sole cause
- Fix criteria:
  - Confirm a causal mechanism, apply a scoped repair, and pass the original focused test plus provider transition regressions.
- Current conclusion: H-004 is confirmed. Provider-aware changes exposed a finite, cumulatively deep async poll chain spanning session initialization, turn admission, previous-model step-context capture, compaction and sampling. Lightweight turn-scoped provider state reduced one contributor, but fresh cancellable Tokio task boundaries were required at the independently deep lifecycle seams.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - exact default-stack reproduction passes after cleanup
  - `cargo test -p codex-core previous_model` passes 5/5
  - provider binding, protocol route and app-server settings regressions pass
- Close reason:
  - repaired and verified without increasing runtime thread stack size

## Hypothesis H-001: Previous-route context reconstruction recurses through provider model resolution
- Status: refuted
- Parent: P-001
- Claim: the new previous-route reconstruction path repeatedly re-enters model resolution for the unknown previous model until the test thread stack is exhausted.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The regression first appeared after `maybe_run_previous_model_inline_compact` began selecting a route-specific runtime and calling `with_provider_model`.
- Falsifiable predictions:
  - If true: the isolated test still overflows; a larger stack either shows repeated model-resolution frames or only delays the overflow.
  - If false: the isolated test passes, or a larger stack makes it complete without evidence of recursive re-entry.
- Diagnostic evidence plan:
  - Prediction or clause under test: isolated deterministic reproduction and stack-size sensitivity.
  - Signal: focused test exit status and overflow marker under default and enlarged `RUST_MIN_STACK`.
  - Capture method: run the exact test twice with controlled test thread settings.
  - Event name or marker:
    - `pre_sampling_compact_keeps_unknown_previous_model_for_api_key_auth_and_custom_provider`
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-002
  - Supports if:
    - overflow reproduces independently and persists or scales with the enlarged stack.
  - Refutes if:
    - isolated default-stack execution passes or failure belongs to test-filter concurrency.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: E-003 shows enlarged stack execution advances to an ordinary assertion instead of continuing to overflow, contradicting unbounded recursive re-entry.
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: run isolated default-stack and enlarged-stack reproductions.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Filtered parallel test execution causes unrelated stack pressure
- Status: refuted
- Parent: P-001
- Claim: the overflow requires the five-test filtered integration run and does not occur when the named test runs alone.
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The failure was observed in a filter that selected five tests, so concurrency is a meaningful alternative.
- Falsifiable predictions:
  - If true: the exact named test passes alone and fails only in the multi-test filter.
  - If false: the exact named test overflows alone.
- Diagnostic evidence plan:
  - Prediction or clause under test: concurrency dependence.
  - Signal: isolated test exit status.
  - Capture method: run the exact integration-test name with `--exact --test-threads=1`.
  - Event name or marker:
    - test name
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-001
  - Supports if:
    - isolated execution passes.
  - Refutes if:
    - isolated execution overflows.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: E-002 reproduces the overflow with the exact test alone and one test thread.
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: run the isolated test.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: Turn client provider override reconstructs oversized session state on the stack
- Status: refuted
- Parent: P-001
- Claim: `new_session_for_provider` treats separately allocated but semantically identical providers as a switch, then constructs a complete `ModelClientState` literal on the Tokio worker stack; this finite construction exhausts the default test-thread stack.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Session initialization constructs the session provider and `ModelClient::new` constructs another provider from cloned metadata, so `Arc::ptr_eq` is not a valid same-provider test.
- Falsifiable predictions:
  - If true: the normal turn path enters the reconstruction branch even without a provider switch, default stack overflows, and enlarged stack advances.
  - If false: both session surfaces share the same provider `Arc`, or enlarged stack still overflows indefinitely.
- Diagnostic evidence plan:
  - Prediction or clause under test: provider identity mismatch plus finite stack-size sensitivity.
  - Signal: constructor code path, pointer-identity guard, exact-test results at default and 32 MiB stacks.
  - Capture method: inspect the provider construction and turn session call sites; compare controlled reproductions.
  - Event name or marker:
    - `new_session_for_provider`
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-001 and H-002
  - Supports if:
    - code constructs independent providers and the larger stack eliminates the overflow.
  - Refutes if:
    - provider instances are shared or stack size has no effect.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-003
  - E-004
- Conclusion: the reconstruction was a real unnecessary stack contributor and was replaced by a lightweight turn-scoped override, but the original test still overflowed afterward. It was not the sole causal mechanism.
- Repair design readiness: superseded by H-004
- Next step: none
- Blocker:
  - none
- Close reason:
  - disproven as sole root cause by post-repair reproduction

## Hypothesis H-004: Multiple independently deep async lifecycle chains share one Tokio worker poll stack
- Status: confirmed
- Parent: P-001
- Claim: the regression is caused by finite cumulative poll-stack depth across session initialization, turn admission, previous-model compaction context capture and sampling; isolating those chains on fresh cancellable Tokio tasks removes the overflow without changing the configured stack size.
- Layer: root-cause
- Factor relation: cumulative
- Depends on:
  - H-003
- Rationale:
  - GDB repeatedly moved the overflow boundary deeper as individual futures were boxed or isolated, and never showed recursive re-entry.
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-005
  - E-006
- Conclusion: confirmed by default-stack GDB traces and the passing original reproduction after task-boundary isolation.
- Repair design readiness: applied
- Next step: retain the focused regression and monitor future growth of these async seams.
- Blocker:
  - none
- Close reason:
  - repaired and verified

## Evidence E-001: Filtered compaction regression aborts
- Related hypotheses:
  - H-001
  - H-002
- Direction: neutral
- Type: reproduction
- Source: `cargo test -p codex-core previous_model`
- Prediction or plan link:
  - Initial symptom reproduction before hypothesis separation.
- Matched signal:
  - stack overflow abort in the named compatibility test
- Correlation keys:
  - `pre_sampling_compact_keeps_unknown_previous_model_for_api_key_auth_and_custom_provider`
- Raw content:
  ```text
  thread 'suite::compact::pre_sampling_compact_keeps_unknown_previous_model_for_api_key_auth_and_custom_provider' has overflowed its stack
  fatal runtime error: stack overflow, aborting
  ```
- Interpretation: the regression is real in the five-test filtered run, but this evidence does not yet distinguish recursion from concurrency.
- Time: 2026-08-23 20:45

## Evidence E-002: Exact single-thread test still overflows
- Related hypotheses:
  - H-002
  - H-003
- Direction: refutes
- Type: reproduction
- Source: exact `cargo test -p codex-core --test all ... --exact --test-threads=1`
- Prediction or plan link:
  - H-002 concurrency-dependence prediction.
- Matched signal:
  - stack overflow with only the named test running
- Correlation keys:
  - test name
- Raw content:
  ```text
  running 1 test
  ... has overflowed its stack
  fatal runtime error: stack overflow, aborting
  ```
- Interpretation: test-filter concurrency is not required; H-002 is refuted.
- Time: 2026-08-23 20:47

## Evidence E-003: Enlarged stack advances to request-count assertion
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: experiment
- Source: `RUST_MIN_STACK=33554432 cargo test ... --exact --test-threads=1`
- Prediction or plan link:
  - H-001 recursion and H-003 finite stack-pressure predictions.
- Matched signal:
  - no stack overflow; test reaches line 2852 and reports request count `1` versus `3`
- Correlation keys:
  - test name
- Raw content:
  ```text
  assertion failed: `(left == right)`
  <1
  >3
  ```
- Interpretation: the stack demand is finite, refuting unbounded recursion and supporting oversized stack construction.
- Time: 2026-08-23 20:48

## Evidence E-004: Pointer identity guard cannot recognize the normal provider
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `core/src/client.rs:721`, `core/src/session/session.rs:1449`, `core/src/session/turn.rs:166`
- Prediction or plan link:
  - H-003 provider identity mismatch clause.
- Matched signal:
  - `ModelClient::new` creates its own provider from cloned metadata; the turn passes `TurnContext.provider`; `new_session_for_provider` compares only `Arc::ptr_eq` and otherwise builds a full `ModelClientState` literal.
- Correlation keys:
  - `new_session_for_provider`
- Raw content:
  ```text
  if Arc::ptr_eq(&provider, &self.state.provider) { ... }
  model_client: ModelClient::new(... session_configuration.provider.info().clone(), ...)
  .new_session_for_provider(Arc::clone(&turn_context.provider))
  ```
- Interpretation: even a non-switch turn can take the heavyweight override branch; this explains both the regression trigger and its stack sensitivity.
- Time: 2026-08-23 20:49

## Evidence E-005: GDB shows a finite deep poll chain rather than recursion
- Related hypotheses:
  - H-003
  - H-004
- Direction: supports
- Type: debugger trace
- Source: exact default-stack test under GDB
- Prediction or plan link:
  - H-004 cumulative poll-stack claim.
- Matched signal:
  - successive top frames occurred in environment resolution, session-store initialization, turn input, MCP step-context capture, pre-sampling compact and sampling request paths.
- Correlation keys:
  - exact test name
- Raw content:
  ```text
  resolve_selected_capability_roots
  capture_step_context_with_required_mcp_servers
  maybe_run_previous_model_inline_compact_with_settings
  run_pre_sampling_compact
  run_turn
  ```
- Interpretation: frames form one finite async call chain with no repeating recursion cycle; task-stack isolation is the scoped repair.
- Time: 2026-08-23 21:08

## Evidence E-006: Original default-stack regression and related suites pass
- Related hypotheses:
  - H-004
- Direction: supports
- Type: validation
- Source: focused local mock tests
- Prediction or plan link:
  - P-001 fix criteria.
- Matched signal:
  - exact reproduction passes with 3 mock requests; the five-test `previous_model` filter passes 5/5.
- Correlation keys:
  - exact test name
  - `previous_model`
- Raw content:
  ```text
  test ...pre_sampling_compact_keeps_unknown_previous_model_for_api_key_auth_and_custom_provider ... ok
  test result: ok. 5 passed; 0 failed
  ```
- Interpretation: the overflow is eliminated on the normal default stack, and previous-model fallback behavior remains intact.
- Time: 2026-08-23 21:22
