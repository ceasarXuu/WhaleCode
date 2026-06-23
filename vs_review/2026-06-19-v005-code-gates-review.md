# Subagent VS Review: v0.0.5 Code Gates

- Created: 2026-06-19T13:05:00+08:00
- Updated: 2026-06-19T13:12:00+08:00
- Task: 对 v0.0.5 继续开发代码落地执行对抗性审查，确认 provider request budget hook、runtime trace bridge、E3 start gate 和 release gate 是否足以继续进入后续验证。
- Report path: `vs_review/2026-06-19-v005-code-gates-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: Implementation Review

### Review Input

#### Objective
检查 v0.0.5 未完成项的当前代码落地是否消除了方案审查中的核心 blocker：provider budget hook 不再 orphan，正式 E3 start gate 不能被弱 marker 绕过，release decision 不能再用 report-only projection 或缺失 v0.0.5 artifacts 得出 `release_pass`。

#### Review Target
代码实现、脚本门禁、测试覆盖和剩余发布风险。

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/mod.rs`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`

#### Change Introduction
Implemented a TaskSpace provider request budget hook that is passed from `try_run_sampling_request` to `ModelClientSession::stream_with_provider_request_budget`, checked immediately before HTTP/WebSocket provider `stream_request`, drained back into ActionMap runtime trace events, and covered by focused Rust tests. Hardened E3 start gate markers into structured JSON identity-bound artifacts, made suite runner honor `gate_decision.full_e3_allowed=false`, and added v0.0.5 required release artifacts/gates.

#### Risk Focus
- Provider budget hook still not truly before all real provider dispatches, or it miscounts retries/fallback/warmup.
- Runtime trace bridge records misleading success/failure, loses request identity, or fails when no active TaskSpace map exists.
- Start gate still allows formal E3 when marker contents are spoofed/stale/mismatched.
- Suite runner still schedules samples when `full_e3_allowed=false`.
- Release decision pass fixtures are self-deceptive and do not prove exact active replacement or runtime budget response.
- Tests only cover helper internals and miss public runner/release behavior.

#### Verification Status
- `cargo test -p codex-core provider_request_budget -- --nocapture` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-e3-start-gate-selftest-3` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-release-decision-selftest-2` passed.
- No real E3 or real agent benchmark has been run after these changes.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Lead with blocking findings.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Multi-module Rust state flow and PowerShell runner behavior need correctness review. | dispatch, runtime state, error paths |
| test-validity-adversary | The historical failure mode was invalid E3/release evidence. | self-deceptive tests, gate bypasses |
| observability-adversary | Runtime budget control depends on request/budget events being diagnostic and joinable. | event schema, trace evidence, release artifacts |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede3c-b66c-7b12-976a-8ae8ca00fd19` / Volta | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede3d-44dc-7811-98f8-dd6c1a4a20fc` / Faraday | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| observability-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede3d-8f8e-7c51-be0d-396f0aa81798` / Maxwell | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Volta

##### Summary
Found blocking issues in provider budget hook semantics, runtime trace bridge, E3 marker validation and release gate.

##### Blocking Findings
- WebSocket connection/auth path happened before budget check.
- Budget status `completed` was emitted when stream opened, before response completion.
- WebSocket prewarm uses disabled budget.
- Runtime bridge can attribute events to a snapshot if TaskSpace state changes later.
- Release gate accepted any exact scan event rather than correlating scan id/request/hash to active replacement report.
- Marker freshness was not validated.
- `v005-non-agent-gates.json` release gate only checked top-level status.

##### Required Fixes
- Move WebSocket budget check before provider connection/request side effects.
- Rename or delay terminal status so stream-open does not mean response-complete.
- Correlate active replacement report to exact scan by scan id, request id and payload hash.
- Parse and enforce marker freshness.
- Validate structured non-agent gate set and evidence paths.

#### test-validity-adversary / Faraday

##### Summary
Gates are stronger than existence checks but still allowed forged/stale shape-correct JSON artifacts.

##### Blocking Findings
- Marker staleness/spoofing still accepted because timestamp freshness, producer, current HEAD and sample-set binding were not enforced.
- `v005_non_agent_gates` accepted pass strings without evidence paths.
- Active replacement proof was not correlated to exact scan event.
- Output-ref proof remains count/shape based.
- Release pass fixture is still synthetic and does not prove real runner artifact consistency.
- Runner honors `full_e3_allowed=false`, but trusts externally supplied markers.

##### Required Fixes
- Add marker freshness and provenance.
- Require structured non-agent gate objects with evidence paths.
- Correlate active replacement report and exact scan event.
- Add tampered shape-valid fixtures that must fail.

#### observability-adversary / Maxwell

##### Summary
Implementation now has a real pre-dispatch counter/blocker, but observability is still budget trace rather than a full provider request lifecycle event.

##### Blocking Findings
- Provider request ids are synthetic and not yet joinable to API telemetry / rollout trace / exact payload scan.
- Budget events were marked completed before response completion and token usage.
- Provider budget trace lacks provider/model, token, latency, payload and terminal lifecycle fields.
- Runtime does not yet produce provider payload proof.
- Release decision checked presence/counts for provider artifacts rather than semantic sufficiency.

##### Required Fixes
- Add real `TaskSpaceProviderRequestEventV1` lifecycle events.
- Use one stable request id across runtime/budget/API/payload scan artifacts.
- Distinguish `dispatch_started`, `stream_opened`, `response_completed`, `failed`, `blocked`.
- Strengthen release provider artifact checks beyond row presence.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Volta | WebSocket budget check happened after connection side effects | blocking | accept | `before_dispatch` was below websocket connection acquisition. | Moved WebSocket budget check before `websocket_connection(...)`. | Covered by Rust focused tests for budget behavior; no WebSocket-specific no-connect mock yet. |
| Volta / Maxwell | `completed` meant stream opened, not response completed | blocking | accept | Stream is consumed later in `turn.rs`. | Renamed status to `stream_opened` to avoid false terminal semantics. | Full `response_completed` lifecycle event remains future implementation before release-pass proof. |
| Volta / Faraday | Exact scan proof not correlated to active replacement report | blocking | accept | Release gate accepted any passed scan row. | `write-release-decision.ps1` now requires matching `exact_payload_scan_event_id`, `request_id`, and `provider_payload_sha256`; added mismatch fixture. | Real artifact generator still needs to produce these fields. |
| Volta / Faraday | Marker freshness/provenance weak | blocking | accept | Marker only checked non-empty `generated_at`. | Start gate now parses `generated_at`, rejects >24h stale markers, requires `producer`, checks code-complete `git_commit` against current HEAD; added stale fixture. | Approval sample-set binding is still partial and should be strengthened before formal E3. |
| Volta / Faraday | `v005_non_agent_gates` accepted pass strings / top-level status | blocking | accept | Both start gate and release gate trusted weak structure. | Start gate and release decision now require structured gates with `status=pass` and `evidence_path`; tests updated. | Evidence path existence/content validation remains a later hardening item. |
| Maxwell | Provider lifecycle observability still incomplete | blocking | accept | Current runtime events are budget trace tags, not full lifecycle events with tokens/payload. | Partially mitigated by not claiming completion; release gate now rejects weak payload proof. | Must implement `TaskSpaceProviderRequestEventV1` lifecycle before v0.0.5 can honestly close. |
| Faraday | Output-ref proof count/shape based | major | defer | This predates current hook/gate task and is not the dominant E3-start blocker. | No change in this pass. | Track as release hardening before final closeout. |
| Faraday | Synthetic pass fixture is self-deceptive | major | accept | It can hide missing semantic joins. | Added tampered fixtures for missing provider event, hash-only replacement, mismatched scan, weak non-agent gates. | A generated realistic fixture is still needed before closeout. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: partially
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Required after the remaining lifecycle-observability work is implemented.
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: no for formal E3; yes for further implementation

## Final Conclusion

The current code is stronger than before and fixes several accepted blockers, but review remains blocked because provider request lifecycle observability is not yet complete. Do not run formal E3 yet.
