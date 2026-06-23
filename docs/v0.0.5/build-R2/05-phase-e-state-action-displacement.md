# Phase E. Legacy state action displacement denominator

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## E.1 Goal

`state_commit_displacement` must measure real legacy actions attempted / displaced / allowed. It must not use `state_commit` section count as the denominator.

## E.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## E.3 Runtime tracking structs

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyStateActionAttemptV1 {
    pub(crate) action: String,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) displaced: bool,
    pub(crate) allowed: bool,
    pub(crate) reason: String,
    pub(crate) created_at_ms: i64,
}
```

Add field:

```rust
legacy_state_action_attempts: Vec<LegacyStateActionAttemptV1>,
```

## E.4 Handler refactor

Current handler calls a pure reject function before executing actions. Replace:

```rust
reject_legacy_state_action_for_active_profile(&args)?;
```

with:

```rust
if let Some(action) = legacy_state_action_name(&args) {
    session
        .record_action_map_legacy_state_action_attempt(
            &turn,
            action,
            /*displaced*/ true,
            /*allowed*/ false,
            "active_profile_requires_state_commit",
        )
        .await;
    return Err(FunctionCallError::RespondToModel(format!(
        "TaskSpace active profile blocks legacy state action `{action}`. Use taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1) to batch state changes; start_task initial_* fields remain allowed for new-task setup."
    )));
}
```

Add:

```rust
fn legacy_state_action_name(args: &TaskSpaceControlArgs) -> Option<&'static str> {
    match args {
        TaskSpaceControlArgs::RecordOutputContract { .. } => Some("record_output_contract"),
        TaskSpaceControlArgs::RecordFactSource { .. } => Some("record_fact_source"),
        TaskSpaceControlArgs::RecordFact { .. } => Some("record_fact"),
        TaskSpaceControlArgs::RecordSuccessCriteria { .. } => Some("record_success_criteria"),
        TaskSpaceControlArgs::RecordOpenQuestion { .. } => Some("record_open_question"),
        TaskSpaceControlArgs::CloseOpenQuestion { .. } => Some("close_open_question"),
        TaskSpaceControlArgs::RecordDecision { .. } => Some("record_decision"),
        TaskSpaceControlArgs::RecordNextBestAction { .. } => Some("record_next_best_action"),
        TaskSpaceControlArgs::MarkResultValidity { .. } => Some("mark_result_validity"),
        TaskSpaceControlArgs::AdoptResult { .. } => Some("adopt_result"),
        _ => None,
    }
}
```

## E.5 Session wrapper

Add to `Session` in `core/src/session/mod.rs`:

```rust
pub(crate) async fn record_action_map_legacy_state_action_attempt(
    &self,
    turn_context: &TurnContext,
    action: &str,
    displaced: bool,
    allowed: bool,
    reason: &str,
) {
    let events = {
        let mut state = self.state.lock().await;
        state.action_map_runtime.record_legacy_state_action_attempt(
            action,
            displaced,
            allowed,
            reason,
        )
    };
    self.emit_map_runtime_events(turn_context, events).await;
}
```

## E.6 Runtime method

```rust
pub(crate) fn record_legacy_state_action_attempt(
    &mut self,
    action: &str,
    displaced: bool,
    allowed: bool,
    reason: &str,
) -> Vec<MapRuntimeEvent> {
    if self.mode != MapRuntimeMode::Experiment { return Vec::new(); }
    let attempt = LegacyStateActionAttemptV1 { ... };
    self.budget_counters.legacy_state_action_attempt_count += 1;
    if displaced { self.budget_counters.legacy_state_action_displaced_count += 1; }
    if allowed { self.budget_counters.legacy_state_action_allowed_count += 1; }
    self.legacy_state_action_attempts.push(attempt);
    vec![self.record_runtime_budget_trace_event(
        "legacy_state_action_attempt",
        ...,
        vec![
            "schema:taskspace-legacy-state-action-attempt-v1".to_string(),
            "producer:runtime".to_string(),
            format!("action:{action}"),
            format!("displaced:{displaced}"),
            format!("allowed:{allowed}"),
            format!("reason:{reason}"),
        ],
    )]
}
```

## E.7 State commit displacement event

In `state_commit_for_main`, replace:

```rust
let legacy_state_action_attempt_count = outcome.accepted_sections.len() + outcome.rejected_sections.len();
let legacy_state_action_displaced_count = outcome.accepted_sections.len();
```

with:

```rust
let legacy_state_action_attempt_count = self.budget_counters.legacy_state_action_attempt_count;
let legacy_state_action_displaced_count = self.budget_counters.legacy_state_action_displaced_count;
let legacy_state_action_count = self.budget_counters.legacy_state_action_allowed_count;
```

Also emit `state_commit_section_count` separately so compression can be measured without corrupting displacement denominator.

## E.8 Script changes

`New-TaskspaceStateCommitDisplacementSummary` must read both:

```text
legacy_state_action_attempt events
state_commit_displacement events
```

Summary rules:

```powershell
$attemptCount = count legacy_state_action_attempt where producer=runtime
$displacedCount = count attempts where displaced=true
$allowedCount = count attempts where allowed=true
$stateCommitCount = count state_commit_displacement where status in accepted,partial
$status = if ($stateCommitCount -gt 0 -and $attemptCount -gt 0 -and $displacedCount -ge $attemptCount -and $allowedCount -le $legacyBudget) { "pass" } else { "fail" }
```

## E.9 Tests

Rust:

```rust
#[test]
fn legacy_state_action_rejection_records_attempt_and_displacement() {}

#[test]
fn state_commit_displacement_denominator_uses_legacy_attempts_not_sections() {}

#[test]
fn state_commit_sections_do_not_increment_legacy_attempt_count() {}
```

PowerShell:

```text
state_commit_displacement fails if no legacy_state_action_attempt event exists
state_commit_displacement fails if displacement denominator is state_commit section count only
state_commit_displacement passes when legacy attempt is blocked and state_commit accepted
```

## E.10 Acceptance

```text
legacy_state_action_attempt_count > 0 on displacement fixture
legacy_state_action_displaced_count >= legacy_state_action_attempt_count
legacy_state_action_count <= route budget
state_commit_count > 0
state_commit_section_count is reported separately
```
