# Problem P-001: R6 terminal commit may cross non-atomic rollout append boundaries
- Status: fixed
- Created: 2026-07-16 06:43
- Updated: 2026-07-16 08:32
- Objective: Ensure crash, resume and offline replay can never observe contradictory terminal event and ActionMap snapshot state.
- Symptoms:
  - Code inspection shows one committed terminal transaction is persisted as multiple rollout appends.
- Expected behavior:
  - A surviving rollout contains either the complete pre-terminal state or the complete terminal commit evidence and replay state.
- Actual behavior:
  - `GraphRevisionCommitted`, terminal trace and snapshot delta are sent through separate `send_event_raw` calls.
- Impact:
  - A process crash can potentially leave persisted `TerminalCommitted` evidence while canonical replay restores Root OPEN / Finish READY from the prior checkpoint.
- Reproduction:
  - Build a rollout with the pre-terminal checkpoint and terminal `GraphRevisionCommitted`, then truncate before the corresponding snapshot delta and run canonical replay.
- Environment:
  - Linux, branch `whalecode-alpha`, E5 candidate `106440d65`.
- Known facts:
  - In-memory `finish_end_for_main` uses clone-validate-commit and closes Root/Finish together.
  - Each `send_event_raw` invokes `persist_rollout_items` independently.
  - Canonical replay currently reduces SnapshotUpdated/SnapshotDelta and ignores GraphRevisionCommitted.
- Ruled out:
  - In-memory Root and Finish are written in separate domain transactions.
- Fix criteria:
  - Deterministic crash-window fixtures prove no surviving rollout can contain a committed terminal event with a pre-terminal replay state.
  - Persistence failure produces no terminal carrier or successful final answer.
  - Resume and offline replay return the same typed verdict and final hash for every crash boundary.
- Current conclusion: The crash-consistency defect was confirmed and repaired. Terminal graph revision, terminal trace and canonical closed snapshot now share one durable envelope; replay rejects every legacy split terminal prefix instead of returning the older OPEN Map.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001, E-004, E-005, E-006 and E-007
- Close reason:
  - the original truncated-prefix reproduction now returns `incomplete_transaction`; complete, corrupt, resumed, forked and live tool-loop paths all satisfy the terminal transaction contract

## Hypothesis H-001: Separate rollout appends expose a terminal commit crash window
- Status: confirmed
- Parent: P-001
- Claim: `emit_action_map_events_for_turn` persists terminal domain events before the matching snapshot delta, so truncation between appends leaves a terminal commit that canonical replay silently omits.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The terminal Runtime mutation and delta are coherent in memory, but durable evidence is emitted by separate awaited append calls.
- Falsifiable predictions:
  - If true: a checkpoint + terminal graph event without its delta replays successfully to the older OPEN map instead of rejecting or restoring the terminal state.
  - If false: append/replay boundaries make the graph event and delta indivisible, or canonical replay detects the incomplete commit.
- Diagnostic evidence plan:
  - Prediction or clause under test: Truncate a deterministic terminal rollout immediately after `GraphRevisionCommitted` and compare replay state with the terminal event revision.
  - Signal: replay verdict, replay map revision/complete state, graph event revision and terminal event payload.
  - Capture method: Add a focused crash-window fixture around the canonical replay entrypoint.
  - Event name or marker:
    - `graph_revision_committed`
    - `map_runtime_snapshot_delta`
  - Correlation keys:
    - terminal map id and revision
  - Differentiates from:
    - H-002
  - Supports if:
    - Replay succeeds at the pre-terminal revision while the surviving graph event records `TerminalCommitted` at the next revision.
  - Refutes if:
    - Replay rejects the incomplete boundary or restores the terminal revision.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - Convert the fixture into a permanent regression test after repair.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready; user authorized continued Phase E implementation
- Next step: closed by the durable terminal transaction envelope and canonical replay validation.
- Blocker:
  - none
- Close reason:
  - fixed and validated by E-004 through E-007

## Hypothesis H-002: ThreadStore batches adjacent append calls into one durable transaction
- Status: refuted
- Parent: P-001
- Claim: Although Session calls persistence multiple times, the underlying live thread keeps adjacent Map events and the delta indivisible across process crash.
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - A lower storage layer could theoretically defer and batch writes.
- Falsifiable predictions:
  - If true: separate `append_items` calls share one transaction or flush boundary and no persisted prefix can end between graph event and delta.
  - If false: each call commits independently and a prefix ending at the graph event is valid input to rollout loading.
- Diagnostic evidence plan:
  - Prediction or clause under test: Trace `live_thread.append_items` to its storage transaction and execute the truncated-prefix replay fixture.
  - Signal: transaction scope and loader acceptance of the prefix.
  - Capture method: Code-path inspection plus deterministic fixture.
  - Event name or marker:
    - none
  - Correlation keys:
    - none
  - Differentiates from:
    - H-001
  - Supports if:
    - Storage proves a cross-call atomic transaction.
  - Refutes if:
    - Each call owns its append transaction and the truncated prefix is loadable.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: closed as alternative.
- Blocker:
  - none
- Close reason:
  - each AddItems command is flushed independently and JSONL items are written sequentially

## Evidence E-001: Session persists lifecycle events and snapshot delta through separate calls
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: code-location
- Source: `core/src/session/mod.rs`, `core/src/taskspace_replay.rs`
- Prediction or plan link:
  - H-001/H-002 persistence and replay path predictions.
- Matched signal:
  - `emit_action_map_events_for_turn` awaits `send_event` for every lifecycle event, then calls `emit_action_map_delta`; `send_event_raw` persists one event; replay selects only checkpoint/delta/mode events.
- Correlation keys:
  - none
- Raw content:
  ```text
  for event in events { self.send_event(...).await; }
  self.emit_action_map_delta().await;
  send_event_raw -> persist_rollout_items([one EventMsg])
  ReplayItem = Mode | Checkpoint | Delta
  ```
- Interpretation: The code has a concrete multi-append boundary and no graph-event reconciliation in replay, supporting H-001 while making H-002 dependent on lower-store transaction behavior.
- Time: 2026-07-16 06:43

## Evidence E-002: Truncated terminal rollout silently replays the pre-terminal snapshot
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `cargo test -p codex-core replay_currently_ignores_terminal_graph_commit_without_delta --lib -- --nocapture`
- Prediction or plan link:
  - H-001 truncated-prefix replay prediction.
- Matched signal:
  - The fixture contains a revision 4 `TerminalCommitted` graph event after a revision 3 checkpoint; replay succeeds and returns the revision 3 OPEN snapshot.
- Correlation keys:
  - rollout id `terminal-crash-window`
- Raw content:
  ```text
  test taskspace_replay_tests::replay_currently_ignores_terminal_graph_commit_without_delta ... ok
  pre_terminal.complete=false
  terminal.complete=true
  replayed.snapshot == pre_terminal
  ```
- Interpretation: The original symptom is deterministic and separates H-001 from a hypothetical atomic lower storage layer.
- Time: 2026-07-16 06:48

## Evidence E-003: Rollout writer has no transaction across AddItems calls
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: code-location
- Source: `thread-store/src/local/live_writer.rs`, `rollout/src/recorder.rs`
- Prediction or plan link:
  - H-002 cross-call transaction prediction.
- Matched signal:
  - `append_items` queues one `RolloutCmd::AddItems`; the writer processes each command, writes pending JSONL items sequentially and flushes immediately.
- Correlation keys:
  - none
- Raw content:
  ```text
  RolloutCmd::AddItems(items) -> state.add_items(items) -> state.flush_if_materialized().await
  write_pending_items_once loops over each JSONL item
  ```
- Interpretation: No lower layer makes the terminal graph event and later delta indivisible; H-002 is refuted.
- Time: 2026-07-16 06:48

## Hypothesis H-003: An unboxed terminal envelope inflates every protocol event stack frame
- Status: confirmed
- Parent: P-001
- Claim: Adding the large terminal snapshot envelope as an unboxed `MapRuntimeEvent` variant increases the size of every enclosing `EventMsg`/`RolloutItem` stack value enough to overflow Tokio worker stacks in unrelated terminal paths.
- Layer: repair-regression
- Factor relation: single
- Depends on:
  - H-001 repair
- Rationale:
  - Rust enums occupy the size of their largest variant; the terminal envelope contains a full ActionMap snapshot.
- Falsifiable predictions:
  - If true: both committed-terminal and plain-final integration tests overflow after the unboxed variant is introduced, and both recover when only that variant is boxed without changing wire JSON.
  - If false: the overflow remains after boxing or only occurs when the terminal variant is constructed.
- Diagnostic evidence plan:
  - Prediction or clause under test: Execute both integration paths before and after boxing only `TerminalCommitted`.
  - Signal: Tokio worker stack overflow and protocol round-trip equality.
  - Capture method: Focused integration tests plus protocol serialization test.
  - Event name or marker:
    - none
  - Correlation keys:
    - test name
  - Differentiates from:
    - recursion in terminal commit or replay
  - Supports if:
    - plain-final also overflows while unboxed and passes after boxing with unchanged JSON.
  - Refutes if:
    - boxing does not change the result.
  - Instrumentation status: permanent regression
  - Instrumentation lifecycle:
    - Retain the protocol round-trip and both integration paths.
- Evidence gate: satisfied
- Related evidence:
  - E-008
- Conclusion: confirmed
- Repair design readiness: complete
- Next step: keep large terminal payload boxed at the protocol boundary.
- Blocker:
  - none
- Close reason:
  - fixed by boxing `MapRuntimeEvent::TerminalCommitted`

## Evidence E-004: The original split terminal prefix is rejected
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-core taskspace_replay_tests --lib`
- Prediction or plan link:
  - P-001 original reproduction and H-001 truncated-prefix prediction.
- Matched signal:
  - `replay_rejects_terminal_graph_commit_without_transaction_envelope` returns `incomplete_transaction`; replay cannot install the older OPEN checkpoint as a successful result.
- Correlation keys:
  - rollout id `terminal-crash-window`
- Raw content:
  ```text
  test taskspace_replay_tests::replay_rejects_terminal_graph_commit_without_transaction_envelope ... ok
  ```
- Interpretation: The exact original symptom no longer occurs.
- Time: 2026-07-16 08:18

## Evidence E-005: Complete and corrupt terminal envelopes have deterministic verdicts
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-core taskspace_replay_tests --lib`
- Prediction or plan link:
  - P-001 crash/corruption fix criteria.
- Matched signal:
  - One valid envelope installs the CLOSED snapshot as one checkpoint; bad snapshot hash returns `result_hash`; bad revision or terminal trace returns `incomplete_transaction`; no partial restore is returned.
- Correlation keys:
  - checkpoint id prefix `map-terminal-`
- Raw content:
  ```text
  terminal_transaction_envelope_replays_as_one_checkpoint ... ok
  terminal_transaction_corruption_is_fatal_without_partial_restore ... ok
  ```
- Interpretation: Canonical replay treats the envelope as an indivisible validated transaction.
- Time: 2026-07-16 08:18

## Evidence E-006: Resume and fork restore the same committed terminal checkpoint
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-core terminal_transaction --lib`
- Prediction or plan link:
  - P-001 resume/fork fix criteria.
- Matched signal:
  - Resume restores the CLOSED terminal revision and checkpoint; fork restores the same state and only rebinds owner identity.
- Correlation keys:
  - terminal map id and revision
- Raw content:
  ```text
  resumed_history_restores_terminal_transaction_checkpoint ... ok
  forked_history_restores_terminal_transaction_and_rebinds_owner ... ok
  ```
- Interpretation: Production reconstruction and canonical replay agree on the terminal transaction.
- Time: 2026-07-16 08:21

## Evidence E-007: The real TaskSpace tool loop writes one terminal envelope and one final
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-core taskspace_terminal_contract --test all -- --nocapture`
- Prediction or plan link:
  - P-001 persistence and final-carrier criteria.
- Matched signal:
  - Successful finish persists one `TerminalCommitted` envelope, no split `finish_end` graph event, and releases one carrier-backed final; plain provider final releases no final and performs no retry.
- Correlation keys:
  - map id, revision and checkpoint id
- Raw content:
  ```text
  committed_finish_carrier_is_the_only_taskspace_final ... ok
  plain_provider_final_is_nonterminal_and_does_not_retry ... ok
  ```
- Interpretation: The production handler, session persistence and completion gate use the same terminal boundary.
- Time: 2026-07-16 08:17

## Evidence E-008: Boxing removes the repair-induced worker stack overflow
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: protocol and `taskspace_terminal_contract` focused tests
- Prediction or plan link:
  - H-003 enum-size prediction.
- Matched signal:
  - Both integration paths overflowed with the unboxed envelope; both passed after boxing the variant. `terminal_committed_round_trips_one_durable_envelope` proves the JSON contract remains stable.
- Correlation keys:
  - test name
- Raw content:
  ```text
  codex-protocol: 197 passed
  taskspace_terminal_contract: 2 passed
  ```
- Interpretation: The failure was caused by enum layout, not terminal recursion, and the indirection repair preserves wire semantics.
- Time: 2026-07-16 08:17
