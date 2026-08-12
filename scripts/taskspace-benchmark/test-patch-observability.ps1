param([string]$RunRoot = "")

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/patch-observability.ps1")
if (-not $RunRoot) {
    $RunRoot = Join-Path $repoRoot ("target/patch-observability-selftest/" + [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
}
$artifactDir = Join-Path $RunRoot "artifacts"
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
$rolloutPath = Join-Path $artifactDir "rollout.jsonl"

function New-Call([string]$Name, [string]$CallId, $Arguments) {
    [pscustomobject]@{
        type = "response_item"
        payload = [pscustomobject]@{
            type = "function_call"; name = $Name; call_id = $CallId
            arguments = ($Arguments | ConvertTo-Json -Compress -Depth 20)
        }
    }
}
function New-Output([string]$CallId, [string]$Text) {
    [pscustomobject]@{
        type = "response_item"
        payload = [pscustomobject]@{ type = "function_call_output"; call_id = $CallId; output = $Text }
    }
}

$multiFilePatch = "*** Begin Patch`n*** Update File: src/a.py`n@@`n-old`n+new`n*** Update File: src/b.py`n@@`n-old`n+new`n*** End Patch"
$singlePatch = "*** Begin Patch`n*** Update File: src/c.py`n@@`n-old`n+new`n*** End Patch"
$bootstrap = @{
    action = "initialize_then_actions"
    initial_nodes = @(@{ node_id = "edit"; kind = "implement_solution"; goal = "edit" })
    current_node_id = "edit"
    continuation = "next_apply_patch"
}
$preflight = '{"error":{"code":"request_multiple_apply_patch_calls_not_allowed"},"request":{"executed_tool_call_count":0}}'
$execSuccess = @{
    kind = "taskspace_exec_result"; status = "completed"; outer_call_id = "exec-success"
    map_id = "map"; map_revision_at_dispatch = 2; reads = @(); hosted_results = @()
    client_results = @(
        @{ call_index = 0; action_id = "patch-action"; node_id = "edit"; tool = "apply_patch"; outcome = "succeeded"; result = @{ type = "function"; output = "Success. Updated the following files" } }
        @{ call_index = 1; action_id = "test-action"; node_id = "verify"; tool = "exec_command"; outcome = "succeeded"; result = @{ type = "function"; output = "tests passed" } }
    )
} | ConvertTo-Json -Compress -Depth 20
$rows = @(
    New-Call "taskspace_control" "bootstrap" $bootstrap
    New-Call "apply_patch" "bootstrap-patch" @{ input = $multiFilePatch }
    New-Call "exec_command" "bootstrap-post" @{ cmd = "pytest" }
    New-Output "bootstrap" "completed"
    New-Output "bootstrap-patch" "Success. Updated the following files"
    New-Output "bootstrap-post" "tests passed"

    New-Call "apply_patch" "prepare-fail" @{ input = $singlePatch }
    New-Call "exec_command" "post-skip" @{ cmd = "pytest" }
    New-Output "prepare-fail" "apply_patch verification failed: context mismatch"
    New-Output "post-skip" "TaskSpaceToolSkippedV2: skipped_due_to_prior_failure"

    New-Call "apply_patch" "multi-1" @{ input = $singlePatch }
    New-Call "apply_patch" "multi-2" @{ input = $singlePatch }
    New-Output "multi-1" $preflight
    New-Output "multi-2" $preflight

    New-Call "apply_patch" "commit-fail" @{ input = $singlePatch }
    New-Output "commit-fail" "patch commit failed: disk; rollback_status=best_effort_partial"

    New-Call "read_file" "read-1" @{ path = "src/a.py" }
    New-Output "read-1" "first content"
    New-Call "read_file" "read-2" @{ path = "src/a.py" }
    New-Output "read-2" "second content"

    New-Call "taskspace_exec" "exec-success" @{
        type = "work"
        tools = @(
            @{ tool = "apply_patch"; node_id = "edit"; input = $singlePatch }
            @{ tool = "exec_command"; node_id = "verify"; input = @{ cmd = "pytest" } }
        )
    }
    New-Output "exec-success" $execSuccess

    New-Call "taskspace_exec" "exec-rejected" @{
        type = "work"
        tools = @(@{ tool = "apply_patch"; node_id = "waiting"; input = $singlePatch })
    }
    New-Output "exec-rejected" 'taskspace_exec rejected: Tool action 0 targeted work node `waiting` in state `waiting`'

    [pscustomobject]@{
        type = "response_item"
        payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_exec"; call_id = "exec-invalid"; arguments = '{"type":"work","tools":[' }
    }
    New-Output "exec-invalid" "taskspace_exec rejected: invalid JSON syntax"
)
@($rows) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 30 } |
    Set-Content -LiteralPath $rolloutPath -Encoding UTF8

$metrics = Get-TaskspacePatchObservability $artifactDir $null
$expected = [ordered]@{
    single_patch_carrier_count = 2
    multi_patch_carrier_attempt_count = 0
    taskspace_exec_parse_failure_count = 1
    request_patch_count = 7
    patch_preflight_reject_count = 1
    patch_dispatch_result_count = 4
    max_request_patch_count = 2
    request_multi_patch_attempt_count = 1
    request_multi_patch_preflight_reject_count = 1
    multi_file_patch_count = 1
    patch_prepare_failure_count = 1
    patch_commit_failure_count = 1
    patch_partial_commit_count = 1
    post_patch_action_count = 3
    post_patch_skipped_count = 1
    unique_read_target_count = 1
    exact_repeat_read_after_visible_feedback_count = 1
}
$failures = New-Object System.Collections.Generic.List[string]
foreach ($entry in $expected.GetEnumerator()) {
    if ([int]$metrics.($entry.Key) -ne [int]$entry.Value) {
        $failures.Add("$($entry.Key): expected=$($entry.Value) actual=$($metrics.($entry.Key))")
    }
}
if ([double]$metrics.read_feedback_visibility_coverage -ne 1.0) {
    $failures.Add("read_feedback_visibility_coverage: expected=1 actual=$($metrics.read_feedback_visibility_coverage)")
}
$serialized = $metrics | ConvertTo-Json -Depth 20
foreach ($secret in @("src/a.py", "src/b.py", "src/c.py", "pytest", "context mismatch")) {
    if ($serialized.Contains($secret)) { $failures.Add("observer leaked payload text: $secret") }
}
if ($failures.Count -gt 0) { throw ($failures -join [Environment]::NewLine) }
Write-Output "patch observability self-test passed"
