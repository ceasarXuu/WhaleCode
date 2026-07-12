param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\metrics-extractor-selftest" }
$runDir = New-Dir (Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function New-TestFile([string]$Path, [string]$Text = "artifact") {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    Set-Content -LiteralPath $Path -Encoding UTF8 -Value $Text
}

$repo = New-Dir (Join-Path $runDir "repo")
Push-Location $repo
try {
    git init | Out-Null
    git config user.email "taskspace-test@example.local" | Out-Null
    git config user.name "TaskSpace Test" | Out-Null
    New-TestFile (Join-Path $repo "src\main.py") "print('base')"
    git add . | Out-Null
    git commit -m "base" | Out-Null
    New-TestFile (Join-Path $repo "src\main.py") "print('changed')"
    New-TestFile (Join-Path $repo ".tbench-testing\lib\python3.11\site-packages\pyarrow\ignored.py") "ignored"
    New-TestFile (Join-Path $repo ".tbench-testing\external-validator-source\probe.txt") "ignored"
    New-TestFile (Join-Path $repo "notes.txt") "real untracked"
} finally {
    Pop-Location
}

$diffPath = Join-Path $runDir "git-diff.patch"
$diffText = Get-TaskspaceDiffText $repo $diffPath
$inventory = @(Get-TaskspaceChangedFileInventory $repo $diffText)
$paths = @($inventory | ForEach-Object { [string]$_.path })
Assert-True ($paths -contains "src/main.py") "tracked source change was not reported"
Assert-True ($paths -contains "notes.txt") "real untracked file was not reported"
Assert-True (@($paths | Where-Object { $_ -like ".tbench-testing/*" }).Count -eq 0) "runtime .tbench-testing files leaked into changed inventory"
Assert-True (@($paths | Where-Object { $_ -like "*external-validator-source*" }).Count -eq 0) "ignored runtime validator-looking files leaked into changed inventory"
$vanishedRows = @{}
Add-TaskspaceChangedPath $vanishedRows $repo "vanished/.python-version" "??" "git_status"
Assert-True ($vanishedRows.ContainsKey("vanished/.python-version")) "vanished changed path was not represented"
Assert-True ([string]$vanishedRows["vanished/.python-version"].hash_status -eq "missing") "vanished changed path did not stay hash_status=missing"

$artifactDir = New-Dir (Join-Path $runDir "large-rollout-artifacts")
$jsonlPath = Join-Path $artifactDir "whale-exec.jsonl"
New-TestFile $jsonlPath ""
$rolloutPath = Join-Path $artifactDir "rollout.jsonl"
$rolloutWriter = [System.IO.StreamWriter]::new($rolloutPath, $false, [System.Text.UTF8Encoding]::new($false))
try {
    $rolloutWriter.WriteLine('{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":20,"cached_input_tokens":80}}}}')
    $rolloutWriter.WriteLine('{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","call_id":"control-1","arguments":"{\"action\":\"initialize_then_actions\"}"}}')
    $rolloutWriter.WriteLine('{"type":"event_msg","payload":{"type":"task_context_event_recorded","id":"task-event-1","sequence":1,"eventType":"function_call","rawPayload":{"type":"function_call","name":"taskspace_control","call_id":"control-2","arguments":"{\"action\":\"finish_then_end\",\"final_candidate\":\"done\"}"}}}')
    $rolloutWriter.WriteLine('{"type":"event_msg","payload":{"type":"task_context_event_recorded","id":"task-event-2","sequence":2,"eventType":"message","originalRole":"assistant","rawPayload":{"type":"message","role":"assistant","phase":"final_answer","content":[{"type":"output_text","text":"done"}]}}}')
    $rolloutWriter.WriteLine('malformed-line')
    $rolloutWriter.WriteLine('{"type":"event_msg","payload":{"type":"fixture","content":"' + ('x' * 2097152) + '"}}')
} finally {
    $rolloutWriter.Dispose()
}
$oldThreshold = $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES
try {
    $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES = "1048576"
    $cost = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $artifactDir -JsonlPath $jsonlPath -ObservabilityJsonPath ""
} finally {
    if ($null -eq $oldThreshold) { Remove-Item Env:\TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES = $oldThreshold }
}
Assert-True ([string]$cost.cost_scan_policy.rollout_scan_mode -eq "streaming_large_rollout") "large rollout did not use streaming scan policy"
Assert-True ([string]$cost.cost_scan_policy.rollout_effective_scan_path -eq [string]$rolloutPath) "large rollout was removed from the effective scan path"
Assert-True ([int]$cost.request_summary.model_request_count -eq 1) "streaming request extractor lost a valid token event"
Assert-True ([int]$cost.taskspace_control_usage.taskspace_control_count -eq 2) "streaming control extractor lost a canonical tool call"
Assert-True ((Get-TaskspaceAgentCompletionEvidence "" "taskspace" $rolloutPath).agent_final_observed) "canonical final Agent message was not detected"
Assert-True (Test-Path -LiteralPath $cost.cost_scan_policy_path) "cost scan policy artifact was not written"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace metrics extractor harness self-test: FAIL"
    $failures | ForEach-Object { Write-Host " - $_" }
    Write-Host "RunRoot: $runDir"
    exit 1
}

Write-Host "TaskSpace metrics extractor harness self-test: PASS"
Write-Host "RunRoot: $runDir"
