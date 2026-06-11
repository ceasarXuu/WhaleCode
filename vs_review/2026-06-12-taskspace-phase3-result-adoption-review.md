# TaskSpace Phase 3 Result Adoption Review

Created: 2026-06-12T03:56:36+08:00
Status: accepted_after_changes

## Scope

Phase 3 implements TaskSpace result adoption and final synthesis gates:

- Track whether node results are adopted by facts, hypotheses, decisions, criteria, or nodes.
- Add `taskspace_control(action="adopt_result")`.
- Auto-adopt result references from facts and decisions.
- Require decisions to cite dependencies and reject unreviewed/invalid result dependencies.
- Block final response on unresolved blocking questions and unsafe decision-result dependencies.
- Expose result adoption and canonical node kind through snapshots, generated schemas, and the TaskSpace viewer.

## Independent Review Round 1

Reviewer: subagent `019eb817-f112-70a1-8bb7-a8ede1d1e228`

Decision: request changes

Finding:

- Medium: restored legacy snapshots could report wrong adoption state. An older snapshot with an accepted result and no `adoption` field restored as `none`, not `accepted_unused`; questioned/invalid results could be underreported as well.

Test gaps noted:

- Missing regression for restoring pre-adoption snapshots.
- Missing final-gate regression for decisions whose referenced result validity changes after recording.
- Residual multi-map decision lookup risk if a future task keeps several map paths under one ledger.

## Fix Applied

- `evidence_package_from_snapshot` now parses result validity first.
- `result_adoption_from_snapshot` preserves adoption refs from the snapshot, then derives adoption state from `validity + refs` via `refresh_state`.
- Added `restore_snapshot_rehydrates_legacy_result_adoption_state`.
- Corrected an existing snapshot round-trip fixture so a manually built accepted result has `AcceptedUnused` adoption state.

## Independent Review Round 2

Reviewer: subagent `019eb841-d5d2-7382-ade2-7411a0396c06`

Decision: accept

Findings:

- No blocking, major, or minor findings.

Residual risk:

- A future restore-level test for `accepted + refs => accepted_adopted` would further harden the migration path, although the current implementation already preserves refs and derives the correct state.

## Validation

Passed:

- `cargo fmt --all`
- `cargo test -p codex-core restore_snapshot_rehydrates_legacy_result_adoption_state --locked --jobs 2`
- `cargo test -p codex-core --lib action_map --locked --jobs 2`
- `cargo test -p codex-tools taskspace_control --locked --jobs 2`
- `cargo test -p codex-protocol action_map_snapshot --locked --jobs 2`
- `cargo run -p codex-app-server-protocol --bin write_schema_fixtures --locked --`
- `cargo test -p codex-app-server-protocol schema_fixtures --locked --jobs 2`
- `cargo test -p codex-tui action_map_viewer --locked --jobs 1`
- `cargo test -p codex-core taskspace_control --locked --jobs 2`
- `cargo check -p codex-core --locked`
- `.\scripts\install-whale-local.ps1`
- `whale --version`
- `whale debug models --bundled`

Notes:

- `cargo fmt --all` emitted the existing nightly-only `imports_granularity = Item` warning but exited successfully.
- Some Windows cargo runs needed longer timeouts due slow compilation and file-lock waits; final reruns passed.
