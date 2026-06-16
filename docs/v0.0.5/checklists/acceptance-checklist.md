# v0.0.5 Corrected Acceptance Checklist

This checklist follows `13-design-corrections-and-engineering-contract.md`. It supersedes earlier checklist language when the two conflict.

## Profile And Baseline

- [ ] `taskspace-v005-shadow` profile exists and does not replace model-visible context.
- [ ] `taskspace-v005-active` profile exists and is the only profile eligible for cost PASS.
- [ ] Standard, v004 legacy TaskSpace, and v005 active/shadow runs are distinguishable in reports.
- [ ] Missing usage fields are reported as unavailable, not treated as zero.

## Cost Governance

- [ ] `token-summary.json` exists for every side, pair, sample, and suite.
- [ ] `request-summary.json` exists for every side, pair, sample, and suite.
- [ ] `suite-cost-gate.json` reports PASS / PARTIAL / FAIL.
- [ ] PASS requires direct input+output ratio `<= 2.0x`.
- [ ] PASS requires agent walltime ratio `<= 2.0x`.
- [ ] PARTIAL requires direct input+output ratio `<= 3.0x`.
- [ ] PARTIAL requires agent walltime ratio `<= 3.0x`.
- [ ] PARTIAL requires `model_request_count_ratio <= 2.5x`.
- [ ] FAIL is emitted if direct input+output, walltime, or model request ratio remains `> 5.0x`.
- [ ] Diagnostic metrics include `avg_input_per_request_ratio`, `uncached_input_ratio` when available, `output_token_ratio`, `state_commit_count`, `projection_tokens`, and `large_output_replay_count`.

## Protocol Compaction

- [ ] `state_commit` is enabled as a compatible new action.
- [ ] Legacy fine-grained actions still work.
- [ ] `state_commit` includes `schema_version`, `commit_id`, and `active_node_id`.
- [ ] Replaying the same `commit_id` is idempotent.
- [ ] Invalid sections are rejected without mutating that section.
- [ ] Valid sections in a partial commit can be accepted.
- [ ] Commit events distinguish accepted, partial, and rejected.
- [ ] Gate responses include structured `next_valid_actions`, `blocking_items`, and `missing_evidence`.
- [ ] `taskspace_control` legacy action usage is reported.

## Output Referenceization

- [ ] Raw tool output `>50KB` is not present in the next active-profile prompt.
- [ ] Provider tool-call/tool-output pairing remains valid after referenceization.
- [ ] `OutputReferenceV1` includes `output_ref`, `sha256`, `bytes`, summary, head, tail, and `raw_output_elided`.
- [ ] Full raw output is stored as an audit artifact.
- [ ] Slice-on-demand returns bounded head/tail/line-range/grep slices.
- [ ] `large_output_replay_count = 0` in `taskspace-v005-active`.

## Context Projection

- [ ] Every TaskSpace request generates a `ContextProjectionV1` event.
- [ ] Shadow profile records projection metrics without replacing context.
- [ ] Active profile uses projection as the model-visible TaskSpace surface.
- [ ] Projection token size is measured and compared with the profile budget.
- [ ] Protected items are never omitted while unresolved.
- [ ] Protected-miss count is zero.
- [ ] Rollback to legacy/full TaskSpace context is available.

## Map Self-Management

- [ ] Every map item has `retention_class`.
- [ ] Every map item has deterministic `base_salience`.
- [ ] Protected evidence has `protected_reason`.
- [ ] GC is archive/audit-only, not physical deletion.
- [ ] Compaction events are produced.
- [ ] Stale blocked nodes are absent from final projection unless still protected.
- [ ] Unreviewed active result count is reduced by `>=60%` in focused E3 or partial reason is documented.
- [ ] Semantic replacement metrics are present.

## Routing

- [ ] Every TaskSpace run has `routing-decision.json`.
- [ ] Router starts report-only before active routing.
- [ ] Low-confidence routing chooses `default_compact`.
- [ ] Thin tasks do not spawn subagents by default.
- [ ] `count-call-stack` routes to `verification_first` when active routing is enabled.
- [ ] Verification-first runs record expected-format decision and local checker evidence.
- [ ] Validator failure or ambiguity can escalate from thin/verification-first.

## Quality

- [ ] `engineering_clean = true`.
- [ ] `suite_score_valid = true`.
- [ ] TaskSpace solved is at least `Standard solved - 1`.
- [ ] `analyze-access-logs` reliability is not below Standard.
- [ ] `log-summary` subagent-heavy runs show decision yield or stopped spawn.
- [ ] Release decision states PASS, PARTIAL, or FAIL with evidence paths.
