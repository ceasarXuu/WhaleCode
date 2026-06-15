function Write-TaskspaceAtomicJson {
    param(
        [Parameter(Mandatory = $true)]$Value,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $dir = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    $tmp = "$Path.t$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    try {
        $jsonValue = if ($Value -is [System.Collections.IDictionary]) {
            $object = New-Object psobject
            foreach ($entry in @($Value.GetEnumerator())) {
                $object | Add-Member -NotePropertyName ([string]$entry.Key) -NotePropertyValue $entry.Value
            }
            $object
        } else {
            $Value
        }
        ($jsonValue | ConvertTo-Json -Depth 40) | Set-Content -LiteralPath $tmp -Encoding UTF8
        Move-Item -LiteralPath $tmp -Destination $Path -Force
    } finally {
        if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force }
    }
}

function Write-TaskspaceRunEvent {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$Event,
        [hashtable]$Data = @{}
    )
    $path = Join-Path $RunDir "events.jsonl"
    if (-not (Test-Path -LiteralPath $RunDir)) {
        New-Item -ItemType Directory -Path $RunDir -Force | Out-Null
    }
    $row = [ordered]@{
        schema_version = 1
        timestamp = (Get-Date).ToString("o")
        event = $Event
    }
    foreach ($key in @($Data.Keys | Sort-Object)) { $row[$key] = $Data[$key] }
    ($row | ConvertTo-Json -Depth 20 -Compress) | Add-Content -LiteralPath $path -Encoding UTF8
}

function Find-TaskspaceLatestRunDir {
    param(
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$ScenarioId
    )
    $scenarioRoot = Join-Path $RunRoot $ScenarioId
    if (-not (Test-Path -LiteralPath $scenarioRoot)) { return "" }
    $runs = @(Get-ChildItem -LiteralPath $scenarioRoot -Directory | Sort-Object Name -Descending)
    if ($runs.Count -eq 0) { return "" }
    $runs[0].FullName
}

function Read-TaskspaceRunStatus {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    $path = Join-Path $RunDir "run-status.json"
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
}

function Test-TaskspaceRunLockStale {
    param($RunStatus)
    if ($null -eq $RunStatus -or -not ($RunStatus.PSObject.Properties.Name -contains "heartbeat_at")) { return $true }
    $heartbeat = [datetime]::Parse([string]$RunStatus.heartbeat_at)
    $staleAfter = if ($RunStatus.PSObject.Properties.Name -contains "stale_after_seconds") { [int]$RunStatus.stale_after_seconds } else { 1800 }
    ((Get-Date) - $heartbeat).TotalSeconds -gt $staleAfter
}

function Initialize-TaskspaceBenchmarkRunState {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$ScenarioId,
        [Parameter(Mandatory = $true)][int]$Repeats,
        [Parameter(Mandatory = $true)][string]$EvidenceTarget,
        [string]$CommandLine = ""
    )
    $status = [ordered]@{
        schema_version = 1
        run_id = Split-Path -Leaf $RunDir
        scenario_id = $ScenarioId
        evidence_target = $EvidenceTarget
        phase = "initialized"
        created_at = (Get-Date).ToString("o")
        updated_at = (Get-Date).ToString("o")
        host = $env:COMPUTERNAME
        process_owner = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
        process_id = $PID
        command_line = $CommandLine
        argv = $CommandLine
        env_snapshot = [ordered]@{
            WHALE_HOME = $env:WHALE_HOME
            TASKSPACE_DOCKER_BACKEND = $env:TASKSPACE_DOCKER_BACKEND
            TASKSPACE_DOCKER_WSL_DISTRO = $env:TASKSPACE_DOCKER_WSL_DISTRO
        }
        repeats = $Repeats
        samples = @([ordered]@{
                sample_id = $ScenarioId
                status_path = (Join-Path $RunDir "sample-status.json")
            })
        attempted_pairs = 0
        completed_pairs = 0
        lock_owner = "$($env:COMPUTERNAME):$PID"
        heartbeat_at = (Get-Date).ToString("o")
        stale_after_seconds = 1800
        resume_decision = "new_run"
        final_aggregate_ready = $false
        resume_command = $CommandLine
        run_validity = "valid"
        diagnostic_comparison_enabled = $true
        exit_code = 0
        resume_allowed = $true
        force_rerun_required = $false
    }
    Write-TaskspaceAtomicJson $status (Join-Path $RunDir "run-status.json")
    Write-TaskspaceRunEvent $RunDir "run_initialized" @{ scenario_id = $ScenarioId; repeats = $Repeats; evidence_target = $EvidenceTarget }
    $status
}

function Set-TaskspaceBenchmarkRunPhase {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$Phase,
        [int]$AttemptedPairs = -1,
        [int]$CompletedPairs = -1,
        [bool]$FinalAggregateReady = $false
    )
    $path = Join-Path $RunDir "run-status.json"
    $status = if (Test-Path -LiteralPath $path) {
        $existing = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
        $map = [ordered]@{}
        foreach ($prop in @($existing.PSObject.Properties)) { $map[$prop.Name] = $prop.Value }
        $map
    } else {
        [ordered]@{ schema_version = 1; run_id = Split-Path -Leaf $RunDir; created_at = (Get-Date).ToString("o") }
    }
    $status["phase"] = $Phase
    $status["updated_at"] = (Get-Date).ToString("o")
    $status["heartbeat_at"] = (Get-Date).ToString("o")
    if ($AttemptedPairs -ge 0) { $status["attempted_pairs"] = $AttemptedPairs }
    if ($CompletedPairs -ge 0) { $status["completed_pairs"] = $CompletedPairs }
    $status["final_aggregate_ready"] = $FinalAggregateReady
    if ($Phase -eq "invalid_harness") {
        $status["run_validity"] = "invalid_harness"
        $status["diagnostic_comparison_enabled"] = $false
        $status["exit_code"] = 3
        $status["resume_allowed"] = $false
        $status["force_rerun_required"] = $true
    } elseif (-not $status.Contains("run_validity")) {
        $status["run_validity"] = "valid"
        $status["diagnostic_comparison_enabled"] = $true
        $status["exit_code"] = 0
        $status["resume_allowed"] = $true
        $status["force_rerun_required"] = $false
    }
    Write-TaskspaceAtomicJson $status $path
    Write-TaskspaceRunEvent $RunDir "run_phase_changed" @{ phase = $Phase; attempted_pairs = $status["attempted_pairs"]; completed_pairs = $status["completed_pairs"] }
}

function Set-TaskspaceSampleStatus {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$Phase,
        [int]$AttemptedPairs = 0,
        [int]$CompletedPairs = 0,
        [string]$IneligibleReason = "",
        [string]$EnvironmentFailureReason = "",
        [string]$AggregateReportPath = "",
        [string]$LastSuccessfulArtifact = "",
        [string]$ResumeCommand = ""
    )
    $status = [ordered]@{
        schema_version = 1
        sample_id = $SampleId
        phase = $Phase
        phase_started_at = (Get-Date).ToString("o")
        phase_updated_at = (Get-Date).ToString("o")
        phase_transition_log_path = (Join-Path $RunDir "events.jsonl")
        pair_cursor = $AttemptedPairs + 1
        attempted_pairs = $AttemptedPairs
        completed_pairs = $CompletedPairs
        ineligible_reason = $IneligibleReason
        environment_failure_reason = $EnvironmentFailureReason
        aggregate_report_path = $AggregateReportPath
        last_successful_artifact = $LastSuccessfulArtifact
        lock_owner = "$($env:COMPUTERNAME):$PID"
        heartbeat_at = (Get-Date).ToString("o")
        stale_after_seconds = 1800
        resume_decision = "new_or_updated_by_current_process"
        audit_status = if ($Phase -eq "audit_required") { "draft_written_pending_review" } elseif ($Phase -eq "finalize") { "completed_or_not_required" } else { "not_started" }
        finalize_idempotency_token = ""
        resume_command = $ResumeCommand
        run_validity = if ($Phase -eq "invalid_harness") { "invalid_harness" } elseif ($Phase -eq "ineligible") { "ineligible" } else { "valid" }
        diagnostic_comparison_enabled = ($Phase -ne "invalid_harness")
        exit_code = if ($Phase -eq "invalid_harness") { 3 } elseif ($Phase -eq "ineligible") { 2 } else { 0 }
        resume_allowed = ($Phase -ne "invalid_harness")
        force_rerun_required = ($Phase -eq "invalid_harness")
        abort_scope = if ($Phase -eq "invalid_harness") { "sample" } else { "none" }
        abort_phase = ""
        abort_signature = ""
    }
    Write-TaskspaceAtomicJson $status (Join-Path $RunDir "sample-status.json")
    Write-TaskspaceRunEvent $RunDir "sample_phase_changed" @{ sample_id = $SampleId; phase = $Phase; attempted_pairs = $AttemptedPairs; completed_pairs = $CompletedPairs }
    $status
}

function Set-TaskspaceInvalidHarnessStatus {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$AbortPhase,
        [Parameter(Mandatory = $true)][string]$Reason,
        $Signature = $null,
        [string]$ArtifactPath = "",
        [string]$ResumeCommand = "",
        [int]$AttemptedPairs = 0,
        [int]$CompletedPairs = 0
    )
    Set-TaskspaceBenchmarkRunPhase $RunDir "invalid_harness" $AttemptedPairs $CompletedPairs $false | Out-Null
    $status = Set-TaskspaceSampleStatus $RunDir $SampleId "invalid_harness" $AttemptedPairs $CompletedPairs "" $Reason "" $ArtifactPath $ResumeCommand
    $statusMap = [ordered]@{}
    if ($status -is [System.Collections.IDictionary]) {
        foreach ($entry in @($status.GetEnumerator())) { $statusMap[[string]$entry.Key] = $entry.Value }
    } else {
        foreach ($prop in @($status.PSObject.Properties)) { $statusMap[$prop.Name] = $prop.Value }
    }
    $statusMap["abort_scope"] = "sample"
    $statusMap["abort_phase"] = $AbortPhase
    $statusMap["abort_signature"] = if ($Signature) { [string]$Signature.key } else { "" }
    $statusMap["abort_reason"] = $Reason
    $statusMap["first_failure_artifact"] = $ArtifactPath
    Write-TaskspaceAtomicJson $statusMap (Join-Path $RunDir "sample-status.json")
    Write-TaskspaceRunEvent $RunDir "sample_aborted_by_guardrail" @{
        sample_id = $SampleId
        abort_phase = $AbortPhase
        reason = $Reason
        abort_signature = $statusMap["abort_signature"]
        artifact = $ArtifactPath
    }
    [pscustomobject]$statusMap
}
