# Subagent VS Review: TaskSpace Cognitive Preflight Tests

- Created: 2026-06-04T04:30:00+08:00
- Updated: 2026-06-04T04:50:00+08:00
- Task: 对 TaskSpace 工程落地方案做最小化预实验，只新增测试代码，不写正式 runtime 功能代码。
- Report path: `vs_review/2026-06-04-taskspace-cognitive-preflight-tests-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: Preflight Test Review

### Review Input

#### Objective

Review whether the new preflight tests are a useful and safe final check before implementing the TaskSpace cognitive-state MVP. The user explicitly asked for a minimal pre-experiment, not formal production code.

#### Review Target

Test-only changes in the existing ActionMap runtime unit test module, plus follow-up production-path schema/tool preflight tests added after review.

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`

#### Change Introduction

The initial change added test-only preflight types and `cognitive_preflight_*` tests under `#[cfg(test)] mod tests` in `runtime.rs`.

The follow-up change split the checks into two categories:

- `cognitive_preflight_contract_sketch_audit_*`: self-contained contract-sketch tests that audit the planned MVP hard gates without claiming production integration coverage.
- real production-path preflight tests in `protocol.rs`, `taskspace_tool.rs`, and `runtime.rs` that inspect existing snapshot/result/tool affordances.

The tests do not change production structs, runtime behavior, tool schema, prompts, snapshot protocol, or viewer code.

#### Verification Status

Commands run:

```powershell
rustup run stable cargo test -p codex-core cognitive_preflight --lib --locked
```

Result: 9 passed, 0 failed.

```powershell
rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked
```

Result: 104 passed, 0 failed.

```powershell
rustup run stable cargo test -p codex-protocol action_map_snapshot_result_serializes_audit_join_keys_and_tool_success --lib --locked
```

Result: 1 passed, 0 failed.

```powershell
rustup run stable cargo test -p codex-tools taskspace_control_preflight --lib --locked
```

Result: 2 passed, 0 failed.

```powershell
git diff --check
```

Result: passed. Git reported only CRLF conversion warnings for edited Rust files.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | The change is entirely tests; it must not become self-deceptive pseudo-validation. | validation quality, fixture realism, coverage |
| architecture-boundary-adversary | The user required no formal runtime implementation; review must ensure only test code changed and scope is clean. | production boundary, overfitting, future implementation safety |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f2d-9fec-7dc1-85f2-0ba62b7d4ef1` / Arendt | spawn_agent tool result | no | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| architecture-boundary-adversary | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f2d-eaf1-7c30-a167-28b638fedf50` / Faraday | spawn_agent tool result | no | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### test-validity-adversary

Status: not closed before fixes.

Blocking findings:

- Most initial tests validated a self-contained test helper rather than production paths, so they could create false confidence.
- Phase-0 schema/tool feasibility was not tested against the real `taskspace_control` tool or snapshot/result protocol.
- Snapshot/restore compatibility for future cognitive state was not proven.

Required fixes:

- Rename or separate self-contained tests as contract-sketch/audit tests, so they do not claim implementation coverage.
- Add a production-path preflight test that serializes real `ActionMapSnapshotResult` and asserts stable audit join keys and camelCase JSON names.
- Add a tool-schema preflight that confirms the current `taskspace_control` schema does not yet expose the planned cognitive MVP actions/fields, making the implementation gap explicit.

#### architecture-boundary-adversary

Status: closed for production-boundary scope.

Findings:

- No production runtime behavior was changed; the initial runtime changes were under `#[cfg(test)] mod tests`.
- Non-blocking risk: contract-sketch tests could be mistaken for a production template if not clearly named.
- Missing useful test: explicit rejection when a required output contract marker is absent.

### Main Agent Response

| Finding | Decision | Action |
|---|---|---|
| Self-contained helper tests can create false confidence. | Accepted. | Renamed the helper-backed tests to `cognitive_preflight_contract_sketch_audit_*` and kept them explicitly scoped as contract-sketch audits. |
| Real result/snapshot affordances were not tested. | Accepted. | Added `action_map_snapshot_result_serializes_audit_join_keys_and_tool_success` in `protocol.rs` using the real `ActionMapSnapshotResult`. |
| Real `taskspace_control` schema gap was not tested. | Accepted. | Added `taskspace_control_preflight_*` tests in `taskspace_tool.rs` to assert current actions/fields and absence of planned MVP cognitive protocol. |
| Missing output contract negative case. | Accepted. | Added `cognitive_preflight_contract_sketch_audit_rejects_missing_output_contract`. |
| Future cognitive snapshot restore is untested. | Partially accepted, scope-limited. | This cannot be fully implemented without adding formal cognitive-state production schema, which the user explicitly deferred. The preflight now verifies current join keys and current schema gap; formal restore/backward-compat tests remain required in the implementation phase. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, future cognitive snapshot restore compatibility belongs to formal implementation because the fields do not exist yet
- Allowed to proceed: yes

## Round 2: Blocking Closure Review

### Closure Reviewer

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Read-only |
|---|---|---|---|
| blocking-closure-reviewer | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f3b-c2c7-7d10-bad7-32c003f5dc18` / Darwin | no | yes |

### Closure Output

Status: CLOSED.

Closure basis:

- The contract-sketch tests are now clearly named `cognitive_preflight_contract_sketch_audit_*`, so they no longer present themselves as production integration coverage.
- The missing output-contract negative case exists.
- Runtime snapshot result join-key coverage exists and uses actual `ActionMapRuntimeState` snapshot output.
- Protocol serialization coverage exists for `assignmentId`, `mapId`, `nodeId`, `toolSuccess`, and rejects snake_case JSON keys.
- Real `taskspace_control` schema preflight exists and asserts that only current actions are exposed while cognitive MVP actions/fields remain absent.

Boundary judgment:

- The change does not violate the "只写测试，不写正式代码" constraint.
- No production structs, runtime behavior, tool schema, prompts, snapshot protocol, or viewer/runtime code were changed.

Remaining formal implementation test debt:

- Positive production tests for the real cognitive-state schema after it exists.
- Snapshot restore/backward-compat tests for cognitive fields after implementation.
- Runtime transition tests for output contracts, fact provenance, result validity, sentinel clearing, and final-artifact dependency gating.
- `taskspace_control` positive/negative tests for actual cognitive MVP actions/fields once added.
- Replay/event/viewer-facing tests proving cognitive state is observable and recoverable.
