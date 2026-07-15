# Problem P-001: R6 terminal commit may cross non-atomic rollout append boundaries
- Status: open
- Created: 2026-07-16 06:43
- Updated: 2026-07-16 06:48
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
- Current conclusion: The crash-consistency defect is confirmed: a valid persisted prefix can contain terminal domain evidence while canonical replay silently returns the older OPEN Map.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

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
- Next step: Replace the multi-append terminal boundary with one durable terminal transaction envelope and make replay consume it as a checkpoint.
- Blocker:
  - none
- Close reason:
  - not closed

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
