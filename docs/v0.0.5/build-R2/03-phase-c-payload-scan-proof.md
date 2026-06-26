# Phase C. Exact provider payload scan proof

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## C.1 Goal

Release proof must be tied to the exact provider-visible payload. It must not be inferred only from `projection-events.jsonl` or from booleans synthesized by `cost-instrumentation.ps1`.

## C.1.1 Current status after Phase A/B follow-up

Status: implemented locally on 2026-06-26; real post-ABI B-tier run evidence still pending.

Current code records provider payload hash/shape fields and scan booleans on
provider request budget events. It now also emits a producer-owned
`exact_payload_scan` runtime trace event with producer `provider_payload_scanner`.

`New-TaskspaceActiveReplacementArtifacts` no longer derives scan rows from
provider budget event booleans. It consumes `exact_payload_scan` trace rows and
joins them to provider request events by `request_id` plus
`provider_payload_sha256`.

The regression gate now rejects:

```text
hash-only active replacement reports
scan hash mismatches
missing provider request payload joins
synthetic scan producers such as cost_instrumentation_synthesized
missing protected_items_present proof
```

Local validation:

```text
cargo test -p codex-core provider_payload --lib
  passed
cargo test -p codex-core provider_request_budget_events_record_replayable_trace --lib
  passed
cargo test -p codex-core taskspace --lib
  91 passed
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
  passed
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
  passed
```

## C.1.2 真实收益证明记录（2026-06-26）

验证命令使用当前 Phase C 代码构建出的真实 `whale.exe`，而不是旧安装包：

```text
WhaleBin = D:\BuildCache\whalecode\cargo-target\debug\whale.exe
Git HEAD = 0125c95e8
RunRoot = target\phase-c-real-benefit-proof\single-file-fast-fix-20260626-202548
Scenario = single-file-fast-fix
Repeats = 1
Model = deepseek-v4-flash
```

这次 smoke 不能证明 TaskSpace 已经获得正向业务收益。结果是
`standard` 解题成功，`taskspace` 解题失败：

```text
reported_evidence_level = E1
valid_pair = True
included_in_utility_aggregate = False
utility_direction = standard_better
failure_taxonomy = agent_patch_wrong
outcome_standard = solved
outcome_taskspace = wrong
taskspace_wall_time_ratio = 1.74
```

因此 Phase C 的真实收益不是“已提升成功率”，而是更基础的工程收益：
它在真实 provider 请求上阻止了 false green，并把失败定位到
provider-visible payload 层。旧的 budget/projection 派生证明只能说明
本地状态或摘要看起来正确，无法证明真正发给 provider 的 payload 已经完成
active context replacement。

真实 artifact 证据：

```text
artifact_dir =
  target\phase-c-real-benefit-proof\single-file-fast-fix-20260626-202548\
  single-file-fast-fix\20260626-202549-940\pair-001\right\artifacts

exact_payload_scan_count = 30
exact_payload_scan_pass_count = 0
exact_payload_scan_producer = provider_payload_scanner
exact_payload_scan_matching_provider_count = 30
failure_reasons =
  legacy_taskspace_history_present,
  raw_taskspace_control_history_present,
  protected_items_missing
```

`active-context-replacement-report.json` 给出的最小现场：

```text
provider_payload_available = true
exact_payload_scan_passed = false
exact_payload_scan_matching_provider_event = true
replacement_confirmed = false
legacy_taskspace_history_present = true
raw_taskspace_control_history_tokens = 917
completed_stale_node_history_tokens = 0
rejected_subagent_body_tokens = 0
large_raw_output_tokens = 0
protected_items_present = false
```

同时，这次失败不能归因为 Phase B 已修过的 action contract ABI 问题或
DeepSeek cache 命中不稳定：

```text
provider_request_count = 10
request_2_plus_hit_rate = 0.991999
trace_coverage = 1
cache_usage_missing_count = 0
native_tools_schema_hot_path_count = 0
tool_free_action_contract_count = 10

request_phase_attribution_coverage = 100
unknown_request_phase_ratio = 0
taskspace_control_count = 2
action_contract_taskspace_control_count = 2
native_taskspace_control_count = 0
```

结论：

```text
真实业务收益：未证明；本样本 TaskSpace 解题失败。
真实工程收益：已证明；Phase C 在真实 provider-visible payload 上发现污染，
  并阻止 active context replacement 被错误标记为通过。
下一步：修复 active context replacement 输入构造，使 provider payload 不再包含
  legacy TaskSpace history / raw taskspace_control history，并生成
  protected_items_present=true 的可验证证明。
```

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

2026-06-26 status: this unacceptable pattern has been replaced. The helper now
parses `exact_payload_scan` trace events and treats budget-only scan booleans as
insufficient evidence.

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
