# Phase C. Exact provider payload scan proof

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## C.1 Goal

Release proof must be tied to the exact provider-visible payload. It must not be inferred only from `projection-events.jsonl` or from booleans synthesized by `cost-instrumentation.ps1`.

## C.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/client.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## C.3 New client-side structs

In `client.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ExactPayloadScanEventV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) scan_event_id: String,
    pub(crate) request_id: String,
    pub(crate) provider_payload_sha256: String,
    pub(crate) scanner_version: String,
    pub(crate) matcher_version: String,
    pub(crate) checked_byte_ranges: Vec<(usize, usize)>,
    pub(crate) negative_checks_performed: Vec<String>,
    pub(crate) active_projection_present: bool,
    pub(crate) legacy_taskspace_history_present: bool,
    pub(crate) raw_taskspace_control_history_tokens: usize,
    pub(crate) completed_stale_node_history_tokens: usize,
    pub(crate) rejected_subagent_body_tokens: usize,
    pub(crate) large_raw_output_tokens: usize,
    pub(crate) protected_items_present: bool,
    pub(crate) passed: bool,
    pub(crate) failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderPayloadEvidenceV1 {
    pub(crate) sha256: String,
    pub(crate) bytes: usize,
    pub(crate) artifact_path: Option<String>,
    pub(crate) exact_scan: ExactPayloadScanEventV1,
}
```

Replace `provider_payload_digest` with:

```rust
fn provider_payload_evidence<T: serde::Serialize>(
    request_id: &str,
    payload: &T,
    capture_policy: ProviderPayloadCapturePolicy,
) -> Option<ProviderPayloadEvidenceV1>
```

## C.4 Scanner implementation

Pseudo:

```rust
fn scan_provider_payload_text(request_id: &str, sha256: &str, text: &str) -> ExactPayloadScanEventV1 {
    let active_projection_present = text.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER);
    let legacy_taskspace_history_present = contains_any(text, &[
        TASKSPACE_SHADOW_PROJECTION_MARKER,
        "TaskSpace Bootstrap",
        "TaskSpace ContextProjectionV1 shadow update",
        "TaskSpace mode is now active",
        "taskspace_control("
    ]);
    let raw_taskspace_control_history_tokens = count_token_estimate_for_blocks(text, "taskspace_control");
    let completed_stale_node_history_tokens = count_token_estimate_for_blocks(text, "completed stale node");
    let rejected_subagent_body_tokens = count_token_estimate_for_blocks(text, "rejected subagent");
    let large_raw_output_tokens = estimate_large_raw_output_tokens(text);
    let protected_items_present = active_projection_block_contains_protected_items(text);

    let mut failure_reasons = Vec::new();
    if !active_projection_present { failure_reasons.push("active_projection_missing".into()); }
    if legacy_taskspace_history_present { failure_reasons.push("legacy_taskspace_history_present".into()); }
    if large_raw_output_tokens > 0 { failure_reasons.push("large_raw_output_present".into()); }
    if !protected_items_present { failure_reasons.push("protected_items_missing".into()); }

    ExactPayloadScanEventV1 {
        schema_version: "taskspace-exact-payload-scan-event-v1",
        scan_event_id: format!("scan:{request_id}:{sha256}"),
        request_id: request_id.to_string(),
        provider_payload_sha256: sha256.to_string(),
        scanner_version: "v005-exact-scan-2".to_string(),
        matcher_version: "v005-marker-and-structural-negative-checks-2".to_string(),
        checked_byte_ranges: vec![(0, text.len())],
        negative_checks_performed: vec![
            "legacy_taskspace_history".into(),
            "raw_taskspace_control_history".into(),
            "completed_stale_node_history".into(),
            "rejected_subagent_body".into(),
            "large_raw_output".into(),
        ],
        active_projection_present,
        legacy_taskspace_history_present,
        raw_taskspace_control_history_tokens,
        completed_stale_node_history_tokens,
        rejected_subagent_body_tokens,
        large_raw_output_tokens,
        protected_items_present,
        passed: failure_reasons.is_empty(),
        failure_reasons,
    }
}
```

## C.5 Event propagation

`ProviderRequestBudgetEvent` must add:

```rust
pub(crate) exact_payload_scan_event_id: Option<String>,
pub(crate) provider_payload_artifact: Option<String>,
pub(crate) raw_taskspace_control_history_tokens: Option<usize>,
pub(crate) completed_stale_node_history_tokens: Option<usize>,
pub(crate) rejected_subagent_body_tokens: Option<usize>,
```

`record_provider_payload` must push both:

```text
provider_request_budget status=payload_captured
exact_payload_scan event with same request_id + provider_payload_sha256
```

The scan event must be created before redaction/hash-only fallback. If payload artifact capture is disabled for privacy, scan event is still mandatory.

## C.6 Artifact generation rule

Update `New-TaskspaceActiveReplacementArtifacts`:

Current unacceptable pattern:

```powershell
# Do not synthesize scan events from budget event booleans alone.
$scanId = "scan-$($event.trace_event_id)"
$passed = [bool]$event.exact_payload_scan_passed -and ...
```

Required pattern:

```powershell
$exactPayloadScanEvents = Get-TaskspaceTraceEvents $ObservabilityJsonPath @("exact_payload_scan")
$providerRequestEvents = Get-TaskspaceTraceEvents $ObservabilityJsonPath @("provider_request_budget")

foreach ($scan in $exactPayloadScanEvents) {
    $matchingProvider = $providerRequestEvents | Where-Object {
        $_.request_id -eq $scan.request_id -and
        $_.provider_payload_sha256 -eq $scan.provider_payload_sha256
    }
    if (-not $matchingProvider) { mark failure }
}
```

## C.7 Release gate rule

`write-release-decision.ps1` must fail when:

```text
active-context-replacement-report.json is present but exact-payload-scan-events.jsonl is empty
scan_event_id does not join provider request event by request_id and payload hash
scan event was synthesized by cost instrumentation instead of producer=provider_lifecycle or producer=provider_payload_scanner
provider_payload_sha256 is empty
legacy_taskspace_history_present=true
large_raw_output_tokens>0
protected_items_present=false
```

## C.8 Tests

Rust tests in `client_tests.rs`:

```rust
#[test]
fn exact_payload_scan_event_id_matches_request_and_payload_hash() {}

#[test]
fn exact_payload_scan_fails_when_shadow_projection_present() {}

#[test]
fn exact_payload_scan_fails_when_large_raw_output_present() {}

#[test]
fn exact_payload_scan_passes_active_projection_with_protected_items() {}
```

PowerShell fixtures:

```text
release-decision fails when exact scan is synthesized without provider event
release-decision fails when scan hash mismatches provider request hash
release-decision passes when scan joins provider request by request_id/hash
```

## C.9 Acceptance

```text
active_context_replacement_gate_pass = true
exact_payload_scan_gate_pass = true
exact_payload_scan_matching_provider_event_count > 0
legacy_taskspace_history_present = false
large_raw_output_tokens = 0
protected_items_present = true
```
