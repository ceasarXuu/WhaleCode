# Problem P-001: W9 TaskSpace mode command is acknowledged without observable activation
- Status: open
- Created: 2026-08-01 23:55
- Updated: 2026-08-02 00:31
- Objective: Identify the first boundary where an explicit TUI TaskSpace activation stops propagating from the embedded app-server command to the thread TaskSpace snapshot.
- Symptoms:
  - `action_map_commands_are_routed_through_app_server_in_tui` is the only failing TUI test.
  - `ThreadMapRuntimeModeSet(Experiment)` returns handled/success, but `thread/taskspace/read` remains `Standard` for 500 ms.
- Expected behavior:
  - Standard remains the default; an explicit `/taskspace` command switches that thread to the Whale TaskSpace runtime represented internally by `Experiment`.
- Actual behavior:
  - The explicit set request is acknowledged, but the read-side snapshot remains in Standard mode.
- Impact:
  - The TUI full gate remains non-green, and explicit TaskSpace activation through the embedded app-server may be ineffective or unobservable.
- Reproduction:
  - Run `cargo nextest run -p codex-tui -E 'test(action_map_commands_are_routed_through_app_server_in_tui)'` and observe `Standard != Experiment` at `tui/src/app/tests.rs:4971`.
- Environment:
  - Linux, branch `whalecode-codex`, HEAD `9f10895af5`, embedded app-server TUI test path.
- Known facts:
  - `MapRuntimeMode::Standard` is the default and means natural context without TaskSpace.
  - `MapRuntimeMode::Experiment` is Whale Rooted DAG TaskSpace, not an upstream Codex feature.
  - The set RPC returns success before the failing read assertion.
- Ruled out:
  - The failure being caused by Standard as the intended default; the test explicitly requests TaskSpace after startup.
- Fix criteria:
  - Diagnostic evidence identifies the first boundary that loses or hides the requested mode and separates routing, completion-order, and state-owner hypotheses.
  - After an authorized repair, the focused W9 test and TaskSpace state/store regressions pass without changing Standard as the default.
- Current conclusion: W9 starts an embedded thread without the projection policy required by TaskSpace activation. The app-server acknowledges Core queue submission, then Core rejects `Experiment` asynchronously and preserves the correct `Standard` default.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - Static call-chain evidence proves the RPC response follows queue submission rather than semantic mode application.
  - A single-variable controlled run changed only the temporary test Codex home to `taskspace_projection_policy = "map-always"`; W9 changed from deterministic failure to pass.
- Disposition: deferred by user decision; do not repair or suppress the test in the current upstream-sync batch.
- Deferred owner: future TaskSpace-focused branch/workstream; this record does not authorize creating that branch now.
- Re-entry criteria:
  - TaskSpace-focused work is explicitly started.
  - The local W9 fixture is given an explicit supported projection policy without changing `Standard` as the product default.
  - A complementary rejection/observability regression covers the missing-policy path.
  - Focused W9, TaskSpace state/store regressions, and the full TUI gate are rerun.
- Close reason:
  - not closed

## Hypothesis H-001: App-server acknowledges the set RPC before Core applies the mode
- Status: confirmed
- Parent: P-001
- Claim: `thread/mapRuntimeMode/set` returns after enqueueing `Op::SetMapRuntimeMode`, while the Core task that applies the op is delayed, rejected, or never observed by the test polling window.
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The RPC response and TUI handled result prove submission success, not necessarily state mutation completion.
- Falsifiable predictions:
  - If true: the set handler sends its response before a correlated Core `ModeChanged` event or store mutation, and extending/awaiting the application signal changes the observation.
  - If false: the Core state is already Experiment before the RPC response is sent.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare the order of set request, Core op processing, mode-changed event, RPC response, and read request for one thread ID.
  - Signal: code-level await boundaries plus a diagnostic event trace or focused probe.
  - Capture method: inspect handlers, then run a diagnostic-only focused test trace if source inspection is insufficient.
  - Event name or marker:
    - set_map_runtime_mode
    - MapRuntimeEvent::ModeChanged
  - Correlation keys:
    - thread_id
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - response precedes mode application, or no application event occurs after a successful enqueue.
  - Refutes if:
    - application is synchronously confirmed before response.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - remove after diagnosis or promote stable thread-correlated state-transition logging after repair
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: `submit_core_op` awaits only `Codex::submit_with_trace`, which sends a `Submission` into `tx_sub`; the empty RPC response therefore confirms enqueueing, not completion or acceptance of the requested state transition.
- Repair design readiness: ready as an API-observability concern, but not required to repair the stale W9 fixture
- Next step: decide separately whether the RPC contract should expose semantic rejection instead of relying on the event stream.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Set and read use different TaskSpace state owners
- Status: refuted
- Parent: P-001
- Claim: Core updates the live session `ActionMapRuntime`, but `thread/taskspace/read` resolves a separate store, hydrated snapshot, or thread instance that remains Standard.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - TaskSpace has both live session state and persistent store/read projection paths, creating a plausible split-brain boundary.
- Falsifiable predictions:
  - If true: the set path and read path resolve different objects or owner identities for the same external thread ID; live state becomes Experiment while read output stays Standard.
  - If false: both paths read and mutate the same authoritative state object.
- Diagnostic evidence plan:
  - Prediction or clause under test: trace the state owner and conversion used by set and `thread/taskspace/read`.
  - Signal: object/source path, conversation/thread IDs, and mode at each boundary.
  - Capture method: code-path inspection followed by a focused in-process probe of live state versus read snapshot.
  - Event name or marker:
    - thread/mapRuntimeMode/set
    - thread/taskspace/read
  - Correlation keys:
    - thread_id
    - conversation_id
  - Differentiates from:
    - H-001
    - H-003
  - Supports if:
    - correlated set/read resolve different session/store authorities.
  - Refutes if:
    - both operations synchronously share the same state authority and identity.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - remove after diagnosis or retain a stable owner/source field if operationally useful
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-004
- Conclusion: both handlers resolve the same `CodexThread` from the same `ThreadManager`, and the explicit-policy controlled run is observable through the unchanged `thread/taskspace/read` path.
- Repair design readiness: not applicable
- Next step: none.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: TUI routes the command to a different thread/session than the read
- Status: refuted
- Parent: P-001
- Claim: the TUI registry accepts the command as handled but uses a stale or different active-thread mapping, so the set RPC and subsequent read do not target the same Core thread.
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The test registers a primary session and separately holds the returned thread ID; routing indirection could diverge.
- Falsifiable predictions:
  - If true: the thread ID sent by `try_submit_active_thread_op_via_app_server` differs from the ID read by `thread_taskspace_read`, or the adapter remaps it to another session.
  - If false: both RPCs carry the identical thread ID and resolve the same thread object.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare command view, explicit thread ID, RPC params, and server thread lookup.
  - Signal: thread IDs at TUI routing and app-server request boundaries.
  - Capture method: static call-chain inspection and, if necessary, a diagnostic assertion in the focused test.
  - Event name or marker:
    - thread/mapRuntimeMode/set
    - thread/taskspace/read
  - Correlation keys:
    - thread_id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - IDs or thread lookup results differ.
  - Refutes if:
    - IDs and resolved thread handles are identical.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - remove after diagnosis
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-004
- Conclusion: the test passes one `thread_id` to both the set and read calls; both server handlers parse that same ID and call the same thread manager lookup. The controlled policy change fixes the failure without touching routing.
- Repair design readiness: not applicable
- Next step: none.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Focused W9 assertion reproduces a successful acknowledgement with stale mode
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction
- Source: `third_party/codex-cli/codex-rs/tui/src/app/tests.rs`, full TUI baseline run
- Prediction or plan link:
  - P-001 reproduction and all three initial hypotheses
- Matched signal:
  - handled is true; read snapshot remains Standard; expected Experiment
- Correlation keys:
  - embedded test thread_id
- Raw content:
  ```text
  assertion failed: `(left == right)`
  <Standard
  >Experiment
  ```
- Interpretation: The failure is after the TUI reports the set command handled and before the read side exposes TaskSpace mode; it does not yet identify which boundary is responsible.
- Time: 2026-08-01 23:55

## Hypothesis H-004: W9 fixture violates the explicit TaskSpace projection-policy precondition
- Status: confirmed
- Parent: P-001
- Claim: W9 starts its embedded thread with `taskspace_projection_policy = None`, so Core intentionally rejects `Experiment`; the test still expects the older unconditional activation contract.
- Layer: root-cause
- Factor relation: interaction
- Depends on:
  - H-001
- Rationale:
  - `set_map_runtime_mode` has an explicit early-return guard for `Experiment` when no projection policy is configured, while the RPC only awaits submission to the Core queue.
- Falsifiable predictions:
  - If true: the W9 fixture resolves to no projection policy, and adding exactly one explicit supported policy makes the unchanged routing/read assertions pass.
  - If false: W9 already has a supported policy, or the mode remains Standard after adding one.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare the focused test with its original config and with only `taskspace_projection_policy = Some(MapAlways)` added before app-server startup.
  - Signal: focused W9 result and observed snapshot mode.
  - Capture method: run original focused reproduction, then a diagnostic-only one-line fixture mutation, rerun, and restore the source.
  - Event name or marker:
    - taskspace.projection_policy_missing
    - MapRuntimeEvent::ModeChanged
  - Correlation keys:
    - test name
    - thread_id
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - original config fails Standard/Experiment and the explicit-policy variant passes.
  - Refutes if:
    - the explicit-policy variant has no effect.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - restore the test source immediately after the controlled run
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: the original default-only fixture lacks a policy and fails; writing exactly `taskspace_projection_policy = "map-always"` into its temporary Codex home makes the unchanged set/read behavior pass.
- Repair design readiness: ready
- Next step: none in the current batch; resume only in the future TaskSpace-focused branch/workstream under the re-entry criteria in P-001.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-002: RPC success is queue acceptance while Core may reject the transition later
- Related hypotheses:
  - H-001
  - H-004
- Direction: supports
- Type: code-path
- Source: `app-server/src/codex_message_processor.rs`, `core/src/session/mod.rs`, `core/src/session/handlers.rs`
- Prediction or plan link:
  - H-001 await-semantics prediction
- Matched signal:
  - `thread_map_runtime_mode_set` awaits `submit_core_op`, which awaits `submit_with_trace`; that method sends the submission to `tx_sub` and returns its ID. `set_map_runtime_mode` runs later and returns early when the projection policy is absent.
- Correlation keys:
  - submission id
  - thread id
- Raw content:
  ```text
  submit_with_trace -> submit_with_id -> tx_sub.send(sub)
  set_map_runtime_mode(Experiment) + policy None -> taskspace.projection_policy_missing -> ErrorEvent -> return
  ```
- Interpretation: `ThreadMapRuntimeModeSetResponse {}` means the command entered Core's queue; it does not prove that Core accepted or applied the mode.
- Time: 2026-08-02 00:10

## Evidence E-003: Set and read resolve the same thread identity and authority
- Related hypotheses:
  - H-002
  - H-003
- Direction: refutes
- Type: code-path
- Source: `tui/src/app/tests.rs`, `tui/src/app/thread_routing.rs`, `app-server/src/codex_message_processor.rs`, `core/src/codex_thread.rs`
- Prediction or plan link:
  - H-002 and H-003 identity/owner predictions
- Matched signal:
  - One local `thread_id` is passed to both RPCs. Both server handlers call `load_thread`, which parses that ID and retrieves one `Arc<CodexThread>` from `ThreadManager`; the read calls that thread's canonical session snapshot.
- Correlation keys:
  - thread id
- Raw content:
  ```text
  set: load_thread(thread_id) -> Op::SetMapRuntimeMode
  read: load_thread(thread_id) -> CodexThread::action_map_snapshot -> Session::canonical_action_map_snapshot
  ```
- Interpretation: there is no distinct thread or store authority in the failing path.
- Time: 2026-08-02 00:12

## Evidence E-004: Explicit projection policy flips the unchanged focused W9 from fail to pass
- Related hypotheses:
  - H-002
  - H-003
  - H-004
- Direction: supports
- Type: controlled experiment
- Source: focused `codex-tui` nextest runs with a diagnostic-only temporary config write
- Prediction or plan link:
  - H-004 single-variable comparison
- Matched signal:
  - Original fixture: `Standard != Experiment`, failed in 0.722 s.
  - Same test with only `taskspace_projection_policy = "map-always"` written to its temporary Codex home: passed in 0.251 s.
- Correlation keys:
  - `app::tests::action_map_commands_are_routed_through_app_server_in_tui`
- Raw content:
  ```text
  original: FAIL, <Standard >Experiment
  explicit policy: PASS, 1 passed, 1892 skipped
  ```
- Interpretation: the projection-policy precondition is both necessary in the original path and sufficient to make mode mutation visible through the existing read path. Diagnostic source changes were restored immediately after the run.
- Time: 2026-08-02 00:17
