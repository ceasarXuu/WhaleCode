if (-not (Get-Command Get-TaskspaceValidationLifecycle -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "harness-health.ps1")
}
if (-not (Get-Command Write-TaskspaceCostInstrumentationArtifacts -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "cost-instrumentation.ps1")
}
if (-not (Get-Command Get-TaskspaceCanonicalResponseItem -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "canonical-rollout.ps1")
}

function Get-TaskspaceDiffText {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$DiffPath
    )
    Push-Location $RepoDir
    try {
        $baselineRef = "refs/taskspace-benchmark/baseline"
        $baselineCommit = [string](& git rev-parse --verify $baselineRef 2>$null)
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($baselineCommit)) {
            throw "workspace_diff_baseline_missing: $baselineRef"
        }
        $headCommit = [string](& git rev-parse --verify HEAD 2>$null)
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($headCommit)) {
            throw "workspace_diff_head_missing"
        }
        $status = @(& git status --porcelain=v1 --untracked-files=all -- .)
        if ($LASTEXITCODE -ne 0) { throw "workspace_diff_status_failed" }
        $diff = git diff $baselineCommit -- .
        if ($LASTEXITCODE -ne 0) { throw "workspace_diff_capture_failed" }
        Set-Content -LiteralPath $DiffPath -Encoding UTF8 -Value $diff
        $evidencePath = Join-Path (Split-Path -Parent $DiffPath) "workspace-change-baseline.json"
        [pscustomobject]@{
            schema_version = "taskspace-workspace-change-baseline-v1"
            baseline_ref = $baselineRef
            baseline_commit = $baselineCommit.Trim()
            final_head_commit = $headCommit.Trim()
            head_advanced = ($baselineCommit.Trim() -ne $headCommit.Trim())
            worktree_dirty = ($status.Count -gt 0)
            status_entry_count = $status.Count
            diff_bytes = [System.Text.Encoding]::UTF8.GetByteCount(($diff -join "`n"))
        } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $evidencePath -Encoding UTF8
        return ($diff -join "`n")
    } finally {
        Pop-Location
    }
}

function Get-TaskspaceChangedPaths {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [string]$DiffText = ""
    )
    @((Get-TaskspaceChangedFileInventory $RepoDir $DiffText) | ForEach-Object { $_.path })
}

function Test-TaskspaceOrdinaryToolBeforeBindingInRollout {
    param([AllowEmptyString()][string]$RolloutPath = "")
    if ([string]::IsNullOrWhiteSpace($RolloutPath)) { return $false }
    if (-not (Test-Path -LiteralPath $RolloutPath -PathType Leaf)) { return $false }

    $bindingEstablished = $false
    foreach ($line in [System.IO.File]::ReadLines($RolloutPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $event = $line | ConvertFrom-Json } catch { continue }
        $payload = $event.payload
        if ($null -eq $payload) { continue }
        $payloadType = Get-TaskspaceRolloutPayloadType $payload
        if ($payloadType -eq "lease_created" -or
            ($payloadType -eq "taskspace_trace_event_recorded" -and
                [string]$payload.kind -eq "mechanical_blank_map_initialized")) {
            $bindingEstablished = $true
            continue
        }
        $responseItem = Get-TaskspaceCanonicalResponseItem $event
        if ($null -eq $responseItem) { continue }
        $responseType = [string]$responseItem.type
        if ($responseType -in @("function_call", "custom_tool_call") -and
            [string]$responseItem.name -ne "taskspace_control" -and
            -not $bindingEstablished) {
            return $true
        }
        if ($responseType -eq "local_shell_call" -and -not $bindingEstablished) {
            return $true
        }
    }
    return $false
}

function Get-TaskspaceAgentCompletionEvidence {
    param(
        [AllowEmptyString()][string]$JsonlPath = "",
        [AllowEmptyString()][string]$LogicalMode = "",
        [AllowEmptyString()][string]$RolloutPath = ""
    )
    $lastAgentMessageIndex = -1
    $lastAgentActionIndex = -1
    $agentMessageCount = 0
    $lastActionability = ""
    $rowIndex = 0
    if (-not [string]::IsNullOrWhiteSpace($JsonlPath) -and (Test-Path -LiteralPath $JsonlPath -PathType Leaf)) {
        foreach ($line in [System.IO.File]::ReadLines($JsonlPath)) {
            $rowIndex++
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try { $row = $line | ConvertFrom-Json } catch { continue }
            if ([string]$row.type -notin @("item.started", "item.completed") -or $null -eq $row.item) { continue }
            $itemType = [string]$row.item.type
            if ([string]$row.type -eq "item.completed" -and $itemType -eq "agent_message") {
                $lastAgentMessageIndex = $rowIndex
                $agentMessageCount++
                continue
            }
            if ($itemType -eq "error") {
                $message = [string]$row.item.message
                if ($message -match 'TaskSpaceProviderResponseActionabilityV1 actionability=([a-z_]+)') {
                    $lastActionability = [string]$Matches[1]
                }
                continue
            }
            if ($itemType -notin @("todo_list", "reasoning")) {
                $lastAgentActionIndex = $rowIndex
            }
        }
    }
    $terminalAgentMessage = $lastAgentMessageIndex -gt $lastAgentActionIndex
    $taskspaceFinalCandidateObserved = $false
    $taskCompleteObserved = $false
    if (-not [string]::IsNullOrWhiteSpace($RolloutPath) -and
        (Test-Path -LiteralPath $RolloutPath -PathType Leaf)) {
        foreach ($line in [System.IO.File]::ReadLines($RolloutPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try { $row = $line | ConvertFrom-Json } catch { continue }
            $payload = $row.payload
            $payloadType = Get-TaskspaceRolloutPayloadType $payload
            if ($null -ne $payload -and $payloadType -eq "task_complete") {
                $taskCompleteObserved = $true
            }
            if ($null -ne $payload -and
                $payloadType -eq "taskspace_trace_event_recorded" -and
                [string]$payload.kind -eq "provider_response_actionability") {
                foreach ($tag in @($payload.tags)) {
                    if ([string]$tag -match '^response_actionability:([a-z_]+)$') {
                        $lastActionability = [string]$Matches[1]
                    }
                }
            }
            $responseItem = Get-TaskspaceCanonicalResponseItem $row
            if ($LogicalMode -eq "taskspace" -and
                $null -ne $responseItem -and
                [string]$responseItem.type -eq "message" -and
                [string]$responseItem.role -eq "assistant" -and
                [string]$responseItem.phase -eq "final_answer") {
                $taskspaceFinalCandidateObserved = $true
            }
        }
    }
    $finalObserved = if ($LogicalMode -eq "taskspace") {
        $taskCompleteObserved -or $taskspaceFinalCandidateObserved -or
            ($terminalAgentMessage -and $lastActionability -eq "final_candidate")
    } else {
        $taskCompleteObserved -or $terminalAgentMessage
    }
    [pscustomobject]@{
        agent_final_observed = [bool]$finalObserved
        agent_completion_source = if ($taskCompleteObserved) {
            "task_complete_event"
        } elseif ($finalObserved -and $LogicalMode -eq "taskspace") {
            "taskspace_final_candidate"
        } elseif ($finalObserved) {
            "terminal_agent_message"
        } else {
            "none"
        }
        last_agent_message_source = if ($lastAgentMessageIndex -ge 0) { "agent_message" } else { "none" }
        agent_message_count = [int]$agentMessageCount
        last_provider_response_actionability = $lastActionability
    }
}

function Get-TaskspaceRolloutToolStats {
    param([AllowEmptyString()][string]$RolloutPath = "")
    $callNames = @{}
    $ordinaryCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $controlCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $failedCallIds = [System.Collections.Generic.HashSet[string]]::new()
    if ([string]::IsNullOrWhiteSpace($RolloutPath) -or -not (Test-Path -LiteralPath $RolloutPath -PathType Leaf)) {
        return [pscustomobject]@{ Completed = 0; Failed = 0; Control = 0; Availability = "missing" }
    }
    foreach ($line in [System.IO.File]::ReadLines($RolloutPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $evt = $line | ConvertFrom-Json } catch { continue }
        $payload = Get-TaskspaceCanonicalResponseItem $evt
        if ($null -eq $payload) { continue }
        $payloadType = [string]$payload.type
        if ($payloadType -in @("function_call", "custom_tool_call")) {
            $callId = [string]$payload.call_id
            if ([string]::IsNullOrWhiteSpace($callId)) { $callId = "rollout-call-$($callNames.Count)" }
            $name = [string]$payload.name
            if ([string]::IsNullOrWhiteSpace($name)) { $name = "unknown" }
            $callNames[$callId] = $name
            if ($name -eq "taskspace_control") {
                [void]$controlCallIds.Add($callId)
            } else {
                [void]$ordinaryCallIds.Add($callId)
            }
            continue
        }
        if ($payloadType -in @("function_call_output", "custom_tool_call_output")) {
            $callId = [string]$payload.call_id
            if ([string]::IsNullOrWhiteSpace($callId)) { continue }
            $name = if ($callNames.ContainsKey($callId)) { [string]$callNames[$callId] } else { "" }
            if ($name -eq "taskspace_control") { continue }
            $output = [string]$payload.output
            if ($output -match "Tool call failed" -or
                $output -match "local_validator_infra_failure" -or
                $output -match "(?m)^Exit code:\s*(?!0\b)\d+" -or
                $output -match '"exit_code"\s*:\s*(?!0\b)\d+') {
                [void]$failedCallIds.Add($callId)
            }
        }
    }
    [pscustomobject]@{
        Completed = $ordinaryCallIds.Count
        Failed = $failedCallIds.Count
        Control = $controlCallIds.Count
        Availability = "measured"
    }
}

function Get-TaskspaceObservabilityToolStats {
    param($Observability = $null)
    if ($null -eq $Observability) {
        return [pscustomobject]@{ Completed = 0; Failed = 0; Control = 0; Availability = "missing" }
    }
    $completed = 0
    foreach ($node in @($Observability.nodes)) {
        foreach ($result in @($node.results)) {
            if ([string]$result.kind -eq "main_tool_call") { $completed++ }
        }
    }
    if ($completed -gt 0) {
        return [pscustomobject]@{ Completed = $completed; Failed = 0; Control = 0; Availability = "observability_results" }
    }
    if ($Observability.PSObject.Properties.Name -contains "summary" -and
        $Observability.summary.PSObject.Properties.Name -contains "runtimeEventCounts") {
        $counts = $Observability.summary.runtimeEventCounts
        $functionCalls = if ($counts.PSObject.Properties.Name -contains "function_call") { [int]$counts.function_call } else { 0 }
        $customCalls = if ($counts.PSObject.Properties.Name -contains "custom_tool_call") { [int]$counts.custom_tool_call } else { 0 }
        $controlCalls = if ($counts.PSObject.Properties.Name -contains "taskspace_control") { [int]$counts.taskspace_control } else { 0 }
        $completed = [Math]::Max(0, $functionCalls + $customCalls - $controlCalls)
        if ($completed -gt 0) {
            return [pscustomobject]@{ Completed = $completed; Failed = 0; Control = $controlCalls; Availability = "observability_runtime_counts" }
        }
    }
    [pscustomobject]@{ Completed = 0; Failed = 0; Control = 0; Availability = "unavailable" }
}

function Test-TaskspaceIgnoredChangedPath {
    param([AllowEmptyString()][string]$Path = "")
    $normalized = $Path.Trim().Trim('"').Replace("\", "/")
    if ([string]::IsNullOrWhiteSpace($normalized)) { return $true }
    $segments = @($normalized -split "/" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    foreach ($segment in $segments) {
        if ($segment -in @(".git", ".tbench-testing", ".venv", "venv", "node_modules", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache")) {
            return $true
        }
    }
    return $false
}

function Add-TaskspaceChangedPath {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Rows,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Status,
        [Parameter(Mandatory = $true)][string]$Source
    )
    $normalized = $Path.Trim().Trim('"').Replace("\", "/")
    if ([string]::IsNullOrWhiteSpace($normalized)) { return }
    if (Test-TaskspaceIgnoredChangedPath $normalized) { return }
    $absolute = Join-Path $RepoDir ($normalized.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
    if (Test-Path -LiteralPath $absolute -PathType Container) {
        $repoRoot = (Resolve-Path -LiteralPath $RepoDir).Path
        foreach ($file in @(Get-ChildItem -LiteralPath $absolute -Recurse -Force -File -ErrorAction SilentlyContinue)) {
            $relative = $file.FullName.Substring($repoRoot.Length).TrimStart("\", "/").Replace("\", "/")
            if (Test-TaskspaceIgnoredChangedPath $relative) { continue }
            Add-TaskspaceChangedPath $Rows $RepoDir $relative $Status $Source
        }
        return
    }
    $sha = ""
    $size = $null
    $hashStatus = "missing"
    $hashError = ""
    $hashErrorId = ""
    $hashRetries = 0
    if (Test-Path -LiteralPath $absolute -PathType Leaf) {
        for ($attempt = 0; $attempt -lt 3; $attempt++) {
            try {
                if ($attempt -gt 0) {
                    $hashRetries++
                    Start-Sleep -Milliseconds 100
                }
                $fileInfo = Get-Item -LiteralPath $absolute -ErrorAction Stop
                $size = [int64]$fileInfo.Length
                $sha = (Get-FileHash -Algorithm SHA256 -LiteralPath $absolute -ErrorAction Stop).Hash.ToLowerInvariant()
                $hashStatus = "hashed"
                $hashError = ""
                $hashErrorId = ""
                break
            } catch {
                $hashStatus = "read_error"
                $hashError = [string]$_.Exception.Message
                $hashErrorId = [string]$_.FullyQualifiedErrorId
                if ($hashErrorId -match "PathNotFound|ItemNotFound" -or $hashError -match "Could not find item|Cannot find path") {
                    $hashStatus = "missing"
                    $hashError = ""
                    $hashErrorId = ""
                    break
                }
                if ($hashError -match "being used by another process|cannot access the file|in use" -or $hashErrorId -match "FileReadError") {
                    $hashStatus = "unavailable_locked"
                }
            }
        }
    }
    $critical = $false
    foreach ($pattern in @("(?i)(^|/)run-tests\.sh$", "(?i)(^|/)tests/", "(?i)oewn\.sqlite$", "(?i)(^|/)external-validator-source/")) {
        if ($normalized -match $pattern) {
            $critical = $true
            break
        }
    }
    if ($Rows.ContainsKey($normalized)) {
        $existing = $Rows[$normalized]
        $sources = @([string]$existing.source -split "," | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        if ($sources -notcontains $Source) { $sources += $Source }
        $existing.source = ($sources -join ",")
        if ($sha) { $existing.sha256 = $sha }
        if ($null -ne $size) { $existing.size_bytes = $size }
        if ($Status -ne "diff") { $existing.status = $Status }
        if ($hashStatus -ne "missing") { $existing.hash_status = $hashStatus }
        if ($hashError) { $existing.hash_error = $hashError }
        if ($hashErrorId) { $existing.hash_error_id = $hashErrorId }
        $existing.hash_retries = [int]$hashRetries
        $existing.critical_artifact = ([bool]$existing.critical_artifact -or $critical)
        return
    }
    $Rows[$normalized] = [pscustomobject]@{
        path = $normalized
        status = $Status
        source = $Source
        sha256 = $sha
        size_bytes = $size
        hash_status = $hashStatus
        hash_error = $hashError
        hash_error_id = $hashErrorId
        hash_retries = [int]$hashRetries
        critical_artifact = $critical
    }
}

function Get-TaskspaceChangedFileInventory {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [string]$DiffText = ""
    )
    $rows = @{}
    foreach ($path in @(Get-ChangedPathsFromDiff $DiffText)) {
        if (Test-TaskspaceIgnoredChangedPath $path) { continue }
        Add-TaskspaceChangedPath $rows $RepoDir $path "diff" "git_diff"
    }
    Push-Location $RepoDir
    try {
        foreach ($line in @(git status --porcelain=v1 --untracked-files=all -- .)) {
            if ([string]::IsNullOrWhiteSpace($line) -or $line.Length -lt 4) { continue }
            $status = $line.Substring(0, 2)
            $path = $line.Substring(3).Trim()
            if ($path.Contains(" -> ")) { $path = ($path -split ' -> ')[-1].Trim() }
            if (Test-TaskspaceIgnoredChangedPath $path) { continue }
            Add-TaskspaceChangedPath $rows $RepoDir $path $status "git_status"
        }
    } finally {
        Pop-Location
    }
    @($rows.Values | Sort-Object path)
}

function Test-TaskspaceValidatorEnvironmentMismatch {
    param([Parameter(Mandatory = $true)]$Validation)
    $combined = ""
    foreach ($path in @($Validation.stdout_path, $Validation.stderr_path)) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            $combined += "`n" + (Get-Content -Raw -Encoding UTF8 -LiteralPath $path)
        }
    }
    if ($combined -match "validator_runtime=terminal_bench_docker_app" -and $combined -match "container_workdir=/app") {
        return $false
    }
    if ($combined -match "validator_runtime_probe=terminal_bench_docker_wrapper") {
        return $true
    }
    $patterns = @(
        "validator_runtime=windows_git_bash_non_docker",
        "platform win32",
        "apt-get: command not found",
        "\.tbench-testing/bin/activate",
        "\\app\\",
        "invalid reference format",
        "failed to build",
        "docker command is required"
    )
    foreach ($pattern in $patterns) {
        if ($combined -match $pattern) { return $true }
    }
    return $false
}

function Get-TaskspaceDockerValidationResult {
    param([Parameter(Mandatory = $true)]$Validation)
    $combined = ""
    foreach ($path in @($Validation.stdout_path, $Validation.stderr_path)) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            $combined += "`n" + (Get-Content -Raw -Encoding UTF8 -LiteralPath $path)
        }
    }
    $resultPath = ""
    $match = [regex]::Match($combined, "(?m)^docker_build_result_path=(.+)$")
    if ($match.Success) { $resultPath = $match.Groups[1].Value.Trim() }
    $json = if ($resultPath -and (Test-Path -LiteralPath $resultPath)) {
        try { Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json } catch { $null }
    } else { $null }
    $classifications = @()
    if ($json -and $json.PSObject.Properties.Name -contains "phases") {
        $classifications = @($json.phases | Where-Object { [string]$_.classification -notin @("ok", "cache_hit") } | ForEach-Object { [string]$_.classification } | Sort-Object -Unique)
    }
    if ($Validation.PSObject.Properties.Name -contains "exit_code" -and [int]$Validation.exit_code -eq 124) {
        $classifications += "public_validation_timeout"
    }
    $fallbackSignature = Get-TaskspaceHarnessTextSignature $combined "validator_pretest" "" ""
    if ($fallbackSignature) { $classifications += [string]$fallbackSignature.stable_code }
    $cleanupPath = ""
    $cleanupMatch = [regex]::Match($combined, "validation_cleanup_result_path=([^\r\n]+)")
    if ($cleanupMatch.Success) { $cleanupPath = $cleanupMatch.Groups[1].Value.Trim() }
    $cleanupJson = if ($cleanupPath -and (Test-Path -LiteralPath $cleanupPath)) {
        try { Get-Content -Raw -Encoding UTF8 -LiteralPath $cleanupPath | ConvertFrom-Json } catch { $null }
    } else { $null }
    if ($cleanupJson -and $cleanupJson.PSObject.Properties.Name -contains "classification" -and [string]$cleanupJson.classification -ne "ok") {
        $classifications += [string]$cleanupJson.classification
    }
    [pscustomobject]@{
        path = $resultPath
        json = $json
        cleanup_path = $cleanupPath
        cleanup_json = $cleanupJson
        classifications = @($classifications | Sort-Object -Unique)
    }
}

function Resolve-TaskspaceRolloutSource {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][datetime]$StartedAt,
        [AllowEmptyString()][string]$ThreadId = ""
    )
    $persisted = Join-Path $ArtifactDir "rollout.jsonl"
    if (Test-Path -LiteralPath $persisted -PathType Leaf) {
        return Get-Item -LiteralPath $persisted
    }
    Find-LatestRollout $StartedAt $ThreadId
}

function Export-TaskspaceObservabilityIfAvailable {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$JsonlPath,
        [Parameter(Mandatory = $true)][datetime]$StartedAt,
        [Parameter(Mandatory = $true)][string]$WhalePath,
        [AllowEmptyString()][string]$ThreadId = ""
    )
    $rollout = Resolve-TaskspaceRolloutSource $ArtifactDir $StartedAt $ThreadId
    $rolloutPath = if ($rollout -and $rollout.PSObject.Properties.Name -contains "FullName") { [string]$rollout.FullName } else { "" }
    if ([string]::IsNullOrWhiteSpace($rolloutPath) -or -not (Test-Path -LiteralPath $rolloutPath)) {
        return [pscustomobject]@{ exit_code = -1; rollout_path = ""; observability_json = ""; observability = $null; availability = "rollout_not_found" }
    }
    $rolloutCopy = Join-Path $ArtifactDir "rollout.jsonl"
    if (([System.IO.Path]::GetFullPath($rolloutPath)) -ne ([System.IO.Path]::GetFullPath($rolloutCopy))) {
        Copy-Item -LiteralPath $rolloutPath -Destination $rolloutCopy -Force
    }
    $obsDir = New-Dir (Join-Path $ArtifactDir "observability")
    $stdoutPath = Join-Path $ArtifactDir "observability.stdout.log"
    $stderrPath = Join-Path $ArtifactDir "observability.stderr.log"
    $exportScript = Join-Path $RepoRoot "scripts\export-action-map-observability.ps1"
    $exitCode = Invoke-RealProcess "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $exportScript, "-RolloutPath", $rolloutCopy, "-JsonlPath", $JsonlPath, "-OutputDir", $obsDir, "-WhalePath", $WhalePath, "-ArtifactRoot", $RepoDir) $RepoDir $stdoutPath $stderrPath 180
    $jsonPath = Join-Path $obsDir "action-map-observability.json"
    $obs = if (Test-Path -LiteralPath $jsonPath) { Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonPath | ConvertFrom-Json } else { $null }
    $availability = if ($obs -and $obs.source -and $obs.source.replay) { [string]$obs.source.replay.availability } elseif ($exitCode -eq 0) { "replay_proof_missing" } else { "replay_failed" }
    $errorCode = if ($obs -and $obs.source -and $obs.source.replay) { [string]$obs.source.replay.error_code } else { "" }
    [pscustomobject]@{ exit_code = $exitCode; rollout_path = $rolloutCopy; observability_json = $jsonPath; observability = $obs; availability = $availability; replay_error_code = $errorCode }
}

function Get-TaskspaceBenchmarkMetrics {
    param(
        [Parameter(Mandatory = $true)]$Side,
        [Parameter(Mandatory = $true)]$Exec,
        [Parameter(Mandatory = $true)]$Validation,
        [Parameter(Mandatory = $true)]$Oracle,
        $ObservabilityResult = $null
    )
    $jsonlText = if (Test-Path -LiteralPath $Exec.jsonl_path) { Get-Content -Raw -Encoding UTF8 -LiteralPath $Exec.jsonl_path } else { "" }
    $commandStats = Get-CommandStats $jsonlText
    $diffPath = Join-Path $Side.ArtifactDir "git-diff.patch"
    $diffText = Get-TaskspaceDiffText $Side.RepoDir $diffPath
    $changedInventory = @(Get-TaskspaceChangedFileInventory $Side.RepoDir $diffText)
    $metricsWarnings = @($changedInventory | Where-Object { [string]$_.hash_status -notin @("hashed", "missing") } | ForEach-Object {
            "metrics_hash_$($_.hash_status):$($_.path)"
        })
    $metricsTaints = @($changedInventory | Where-Object {
            [bool]$_.critical_artifact -and [string]$_.hash_status -notin @("hashed", "missing")
        } | ForEach-Object {
            "metrics_critical_artifact_unhashed:$($_.path)"
    })
    $obs = if ($ObservabilityResult) { $ObservabilityResult.observability } else { $null }
    $observabilityAvailability = if ($ObservabilityResult -and $ObservabilityResult.PSObject.Properties.Name -contains "availability") { [string]$ObservabilityResult.availability } elseif ($Side.LogicalMode -eq "taskspace") { "missing" } else { "not_applicable" }
    $observabilityReplayFailed = $Side.LogicalMode -eq "taskspace" -and $observabilityAvailability -eq "replay_failed"
    if ($observabilityReplayFailed) {
        $metricsTaints += "observability_replay_failed:$([string]$ObservabilityResult.replay_error_code)"
    }
    $activeSentinelWarnings = @()
    if ($obs -and $obs.PSObject.Properties.Name -contains "sentinelWarnings") {
        $activeSentinelWarnings = @($obs.sentinelWarnings | Where-Object { [string]$_.status -eq "active" })
    }
    $dockerResult = Get-TaskspaceDockerValidationResult $Validation
    $lifecycle = Get-TaskspaceValidationLifecycle $Validation
    $probeResult = Get-TaskspaceValidatorProbeResult $Validation
    $validationText = Get-TaskspaceValidationText $Validation
    $fallbackSignature = Get-TaskspaceHarnessTextSignature $validationText "validator_pretest" $Side.Name $Validation.stderr_path
    $probeSignature = $null
    if ($probeResult.json -and $probeResult.json.PSObject.Properties.Name -contains "failure_signature" -and $probeResult.json.failure_signature) {
        $probeSignature = $probeResult.json.failure_signature
    }
    $infraSignature = if ($probeSignature) { $probeSignature } else { $fallbackSignature }
    $pretestFailure = ([int]$Validation.exit_code -ne 0 -and -not [bool]$lifecycle.tests_started_seen)
    if ($pretestFailure -and $null -eq $infraSignature) {
        $infraSignature = New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "no_tests_started_marker" "Validation failed before tests_started marker" $Side.Name $Validation.stderr_path
    }
    $validatorEnvironmentFailures = @($dockerResult.classifications | Where-Object {
            $classification = [string]$_
            -not ($classification -eq "docker_run_failure" -and [bool]$lifecycle.tests_started_seen -and [bool]$lifecycle.tests_completed_seen)
        })
    $graphHealth = Get-TaskspaceGraphHealth $obs
    $graphHealthReport = New-TaskspaceGraphHealthReport $obs $Side.Name $Side.LogicalMode
    $graphHealthPath = Join-Path $Side.ArtifactDir "graph-health.json"
    Write-TaskspaceGraphHealthReport $graphHealthReport $graphHealthPath
    $observabilityJsonPath = if ($ObservabilityResult) { [string]$ObservabilityResult.observability_json } else { "" }
    $costInstrumentation = Write-TaskspaceCostInstrumentationArtifacts $Side.ArtifactDir $Exec.jsonl_path $observabilityJsonPath
    $activeReplacementReport = $costInstrumentation.active_context_replacement_report
    if ($activeReplacementReport -and [int]$activeReplacementReport.active_projection_uniqueness_violation_count -gt 0) {
        $metricsTaints += "active_projection_not_unique:$([int]$activeReplacementReport.active_projection_uniqueness_violation_count)"
    }
    $agentCompletion = Get-TaskspaceAgentCompletionEvidence `
        $Exec.jsonl_path `
        $Side.LogicalMode `
        ([string]$costInstrumentation.cost_scan_policy.rollout_effective_scan_path)
    $profileHardStopSeen = $false
    $lifecycleScanPaths = @(
        [string]$costInstrumentation.cost_scan_policy.rollout_effective_scan_path,
        [string]$Exec.last_message_path,
        [string]$Exec.stderr_path
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path -LiteralPath $_) } | Sort-Object -Unique
    foreach ($scanPath in $lifecycleScanPaths) {
        if (Select-String -LiteralPath $scanPath -SimpleMatch -Quiet -Pattern @(
                "TaskSpaceProviderBudgetHardStopV1",
                "TaskSpace provider budget hard stop",
                "provider_request_hard_limit_exceeded"
            )) {
            $profileHardStopSeen = $true
            break
        }
    }
    $rolloutToolStats = Get-TaskspaceRolloutToolStats ([string]$costInstrumentation.cost_scan_policy.rollout_effective_scan_path)
    $observabilityToolStats = Get-TaskspaceObservabilityToolStats $obs
    $toolCallCount = [Math]::Max([int]$commandStats.Completed, [Math]::Max([int]$rolloutToolStats.Completed, [int]$observabilityToolStats.Completed))
    $failedToolCallCount = [Math]::Max([int]$commandStats.Failed, [Math]::Max([int]$rolloutToolStats.Failed, [int]$observabilityToolStats.Failed))
    $subagentThreadIds = @()
    if ($obs) {
        $subagentThreadIds = @($obs.nodes | ForEach-Object {
                @($_.leases | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.agentThreadId) } | ForEach-Object { [string]$_.agentThreadId })
            } | Sort-Object -Unique)
    }
    $metrics = [pscustomobject]@{
        mode = $Side.Name
        logical_mode = $Side.LogicalMode
        exec_exit_code = $Exec.exit_code
        exec_timed_out = ($Exec.PSObject.Properties.Name -contains "timed_out" -and [bool]$Exec.timed_out)
        agent_final_observed = [bool]$agentCompletion.agent_final_observed
        agent_completion_source = [string]$agentCompletion.agent_completion_source
        last_agent_message_source = [string]$agentCompletion.last_agent_message_source
        agent_message_count = [int]$agentCompletion.agent_message_count
        last_provider_response_actionability = [string]$agentCompletion.last_provider_response_actionability
        taskspace_profile_hard_stop_seen = [bool]$profileHardStopSeen
        public_validation_exit_code = $Validation.exit_code
        hidden_oracle_exit_code = $Oracle.exit_code
        wall_time_ms = $Exec.wall_time_ms
        tool_call_count = $toolCallCount
        failed_tool_call_count = $failedToolCallCount
        rollout_tool_call_count = $rolloutToolStats.Completed
        rollout_failed_tool_call_count = $rolloutToolStats.Failed
        rollout_control_tool_call_count = $rolloutToolStats.Control
        rollout_tool_call_availability = [string]$rolloutToolStats.Availability
        observability_tool_call_count = $observabilityToolStats.Completed
        observability_failed_tool_call_count = $observabilityToolStats.Failed
        observability_tool_call_availability = [string]$observabilityToolStats.Availability
        token_summary_path = [string]$costInstrumentation.token_summary_path
        cost_scan_policy_path = [string]$costInstrumentation.cost_scan_policy_path
        rollout_scan_mode = [string]$costInstrumentation.cost_scan_policy.rollout_scan_mode
        rollout_bytes = $costInstrumentation.cost_scan_policy.rollout_bytes
        rollout_scan_max_bytes = $costInstrumentation.cost_scan_policy.rollout_scan_max_bytes
        request_summary_path = [string]$costInstrumentation.request_summary_path
        provider_input_visibility_path = [string]$costInstrumentation.provider_input_visibility_path
        taskspace_control_usage_path = [string]$costInstrumentation.taskspace_control_usage_path
        context_projection_summary_path = [string]$costInstrumentation.context_projection_summary_path
        projection_events_path = [string]$costInstrumentation.projection_events_path
        token_summary_availability = [string]$costInstrumentation.token_summary.availability
        jsonl_bytes = $costInstrumentation.provider_input_visibility.jsonl_bytes
        provider_input_tokens_per_jsonl_kb = $costInstrumentation.provider_input_visibility.provider_input_tokens_per_jsonl_kb
        provider_total_tokens_per_jsonl_kb = $costInstrumentation.provider_input_visibility.provider_total_tokens_per_jsonl_kb
        model_request_count_source = [string]$costInstrumentation.request_summary.model_request_count_source
        token_usage_record_count = $costInstrumentation.request_summary.token_usage_record_count
        model_request_count = $costInstrumentation.request_summary.model_request_count
        input_tokens = $costInstrumentation.token_summary.input_tokens
        output_tokens = $costInstrumentation.token_summary.output_tokens
        cached_input_tokens = $costInstrumentation.token_summary.cached_input_tokens
        uncached_input_tokens = $costInstrumentation.token_summary.uncached_input_tokens
        avg_input_tokens_per_request = $costInstrumentation.request_summary.avg_input_tokens_per_request
        avg_output_tokens_per_request = $costInstrumentation.request_summary.avg_output_tokens_per_request
        max_input_tokens_per_request = $costInstrumentation.request_summary.max_input_tokens_per_request
        p95_input_tokens_per_request = $costInstrumentation.request_summary.p95_input_tokens_per_request
        first_input_tokens_per_request = $costInstrumentation.request_summary.first_input_tokens_per_request
        last_input_tokens_per_request = $costInstrumentation.request_summary.last_input_tokens_per_request
        max_output_tokens_per_request = $costInstrumentation.request_summary.max_output_tokens_per_request
        p95_output_tokens_per_request = $costInstrumentation.request_summary.p95_output_tokens_per_request
        first_output_tokens_per_request = $costInstrumentation.request_summary.first_output_tokens_per_request
        last_output_tokens_per_request = $costInstrumentation.request_summary.last_output_tokens_per_request
        rollout_trace_request_availability = [string]$costInstrumentation.request_summary.rollout_trace.availability
        rollout_trace_model_request_count = $costInstrumentation.request_summary.rollout_trace.model_request_count
        rollout_trace_input_tokens = $costInstrumentation.request_summary.rollout_trace.input_tokens
        rollout_trace_output_tokens = $costInstrumentation.request_summary.rollout_trace.output_tokens
        rollout_trace_max_input_tokens_per_request = $costInstrumentation.request_summary.rollout_trace.max_input_tokens_per_request
        rollout_trace_p95_input_tokens_per_request = $costInstrumentation.request_summary.rollout_trace.p95_input_tokens_per_request
        rollout_trace_first_input_tokens_per_request = $costInstrumentation.request_summary.rollout_trace.first_input_tokens_per_request
        rollout_trace_last_input_tokens_per_request = $costInstrumentation.request_summary.rollout_trace.last_input_tokens_per_request
        runtime_boundary_forbidden_marker_count = @($costInstrumentation.exact_payload_scan_events | Where-Object {
                -not [string]::IsNullOrWhiteSpace([string]$_.runtime_boundary_forbidden_markers) -and
                [string]$_.runtime_boundary_forbidden_markers -ne "none"
            }).Count
        exact_payload_scan_event_count = @($costInstrumentation.exact_payload_scan_events).Count
        active_projection_count_max = if ($activeReplacementReport) { [int]$activeReplacementReport.active_projection_count_max } else { 0 }
        active_projection_uniqueness_violation_count = if ($activeReplacementReport) { [int]$activeReplacementReport.active_projection_uniqueness_violation_count } else { 0 }
        projection_message_tail_violation_count = if ($activeReplacementReport) { [int]$activeReplacementReport.projection_message_tail_violation_count } else { 0 }
        taskspace_control_count_source = [string]$costInstrumentation.taskspace_control_usage.taskspace_control_count_source
        taskspace_control_count_source_mismatch = [bool]$costInstrumentation.taskspace_control_usage.taskspace_control_count_source_mismatch
        whale_exec_taskspace_control_count = [int]$costInstrumentation.taskspace_control_usage.whale_exec_taskspace_control_count
        rollout_taskspace_control_count = [int]$costInstrumentation.taskspace_control_usage.rollout_taskspace_control_count
        taskspace_control_count = [int]$costInstrumentation.taskspace_control_usage.taskspace_control_count
        native_taskspace_control_count = [int]$costInstrumentation.taskspace_control_usage.native_taskspace_control_count
        action_contract_taskspace_control_count = [int]$costInstrumentation.taskspace_control_usage.action_contract_taskspace_control_count
        carrier_action_count = [int]$costInstrumentation.taskspace_control_usage.carrier_action_count
        carrier_failure_count = [int]$costInstrumentation.taskspace_control_usage.carrier_failure_count
        carrier_state_failure_count = [int]$costInstrumentation.taskspace_control_usage.carrier_state_failure_count
        carrier_protocol_failure_count = [int]$costInstrumentation.taskspace_control_usage.carrier_protocol_failure_count
        carrier_argument_failure_count = [int]$costInstrumentation.taskspace_control_usage.carrier_argument_failure_count
        carrier_resource_failure_count = [int]$costInstrumentation.taskspace_control_usage.carrier_resource_failure_count
        control_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_failure_count
        control_preflight_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_preflight_failure_count
        control_handler_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_handler_failure_count
        control_protocol_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_protocol_failure_count
        control_state_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_state_failure_count
        control_argument_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_argument_failure_count
        control_resource_failure_count = [int]$costInstrumentation.taskspace_control_usage.control_resource_failure_count
        ordinary_gate_failure_count = [int]$costInstrumentation.taskspace_control_usage.ordinary_gate_failure_count
        taskspace_boundary_failure_count = [int]$costInstrumentation.taskspace_control_usage.taskspace_boundary_failure_count
        committed_control_count = [int]$costInstrumentation.taskspace_control_usage.committed_control_count
        graph_revision_commit_count = [int]$costInstrumentation.taskspace_control_usage.graph_revision_commit_count
        state_commit_count = [int]$costInstrumentation.taskspace_control_usage.state_commit_count
        state_commit_count_source = [string]$costInstrumentation.taskspace_control_usage.state_commit_count_source
        runtime_state_commit_count = [int]$costInstrumentation.taskspace_control_usage.runtime_state_commit_count
        runtime_output_ref_created_count = [int]$costInstrumentation.taskspace_control_usage.runtime_output_ref_created_count
        runtime_output_ref_slice_read_count = [int]$costInstrumentation.taskspace_control_usage.runtime_output_ref_slice_read_count
        taskspace_runtime_event_count = [int]$costInstrumentation.taskspace_control_usage.taskspace_runtime_event_count
        active_sentinel_warning_count = [int]$activeSentinelWarnings.Count
        active_sentinel_warning_types = @($activeSentinelWarnings | ForEach-Object { [string]$_.sentinelType } | Sort-Object -Unique)
        context_projection_availability = [string]$costInstrumentation.context_projection_summary.availability
        projection_count = [int]$costInstrumentation.context_projection_summary.projection_count
        projection_tokens = $costInstrumentation.context_projection_summary.projection_tokens_total
        projection_tokens_max = $costInstrumentation.context_projection_summary.projection_tokens_max
        projection_protected_miss_count = [int]$costInstrumentation.context_projection_summary.protected_miss_count
        large_output_replay_count = [int]$costInstrumentation.replay_summary.large_output_replay_count
        largest_tool_output_bytes = [int64]$costInstrumentation.replay_summary.largest_tool_output_bytes
        raw_output_in_prompt_violation = [bool]$costInstrumentation.replay_summary.raw_output_in_prompt_violation
        changed_file_inventory = @($changedInventory)
        changed_paths = @($changedInventory | ForEach-Object { $_.path })
        metrics_warnings = @($metricsWarnings)
        metrics_taints = @($metricsTaints)
        observability_availability = $observabilityAvailability
        observability_replay_error_code = if ($ObservabilityResult) { [string]$ObservabilityResult.replay_error_code } else { "" }
        docker_build_result_path = $dockerResult.path
        docker_cache_enabled = ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_enabled" -and [bool]$dockerResult.json.cache_enabled)
        docker_cache_eligible = ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_eligible" -and [bool]$dockerResult.json.cache_eligible)
        docker_cache_hit = ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_hit" -and [bool]$dockerResult.json.cache_hit)
        docker_cache_bypass_reason = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_bypass_reason") { [string]$dockerResult.json.cache_bypass_reason } else { "" }
        docker_cache_key = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_key") { [string]$dockerResult.json.cache_key } else { "" }
        docker_cache_image = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_image") { [string]$dockerResult.json.cache_image } else { "" }
        docker_cache_lock_wait_ms = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_lock_wait_ms") { [int64]$dockerResult.json.cache_lock_wait_ms } else { $null }
        docker_cache_manifest_path = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "cache_manifest_path") { [string]$dockerResult.json.cache_manifest_path } else { "" }
        dockerfile_from_images = if ($dockerResult.json -and $dockerResult.json.PSObject.Properties.Name -contains "dockerfile_from_images") { @($dockerResult.json.dockerfile_from_images) } else { @() }
        validation_cleanup_result_path = $dockerResult.cleanup_path
        validator_environment_failures = @($validatorEnvironmentFailures)
        validator_environment_mismatch = (Test-TaskspaceValidatorEnvironmentMismatch $Validation)
        validator_probe_result_path = $probeResult.path
        validator_probe_status = if ($probeResult.json -and $probeResult.json.PSObject.Properties.Name -contains "status") { [string]$probeResult.json.status } else { "" }
        tests_started_seen = [bool]$lifecycle.tests_started_seen
        tests_completed_seen = [bool]$lifecycle.tests_completed_seen
        validation_lifecycle_stage = [string]$lifecycle.validation_lifecycle_stage
        validation_timeout_phase = [string]$lifecycle.validation_timeout_phase
        tests_started_at = [string]$lifecycle.tests_started_at
        tests_completed_at = [string]$lifecycle.tests_completed_at
        public_validation_reached_tests = [bool]$lifecycle.tests_started_seen
        pretest_failure = [bool]$pretestFailure
        infra_signature = $infraSignature
        business_success = ($Exec.exit_code -eq 0 -and $Validation.exit_code -eq 0 -and $Oracle.exit_code -eq 0 -and -not $observabilityReplayFailed)
        invalid_prompt = $false
        invalid_pair = $false
        harness_failure = $observabilityReplayFailed
        diff_path = $diffPath
        jsonl_path = $Exec.jsonl_path
        last_message_path = $Exec.last_message_path
        stderr_path = $Exec.stderr_path
        validation_stdout_path = $Validation.stdout_path
        validation_stderr_path = $Validation.stderr_path
        oracle_stdout_path = $Oracle.stdout_path
        oracle_stderr_path = $Oracle.stderr_path
        oracle_isolation_level = $Oracle.oracle_isolation_level
        maps = if ($obs) { @($obs.maps).Count } else { 0 }
        nodes = if ($obs) { @($obs.nodes).Count } else { 0 }
        edges = if ($obs) { @($obs.edges).Count } else { 0 }
        edge_order_violations = $graphHealth.EdgeOrderViolationCount
        spawn_agent_calls = if ($obs) { @($obs.toolCalls | Where-Object { $_.tool -eq "spawn_agent" -and $_.status -eq "completed" }).Count } else { 0 }
        subagent_results = if ($obs) { @($obs.nodes | ForEach-Object { @($_.results | Where-Object { $subagentThreadIds -contains [string]$_.sourceThreadId }) }).Count } else { 0 }
        open_leaf_nodes = $graphHealth.OpenLeafNodeCount
        ordinary_before_binding = if ($Side.LogicalMode -eq "taskspace" -and $ObservabilityResult) {
            $rolloutPath = [string]$ObservabilityResult.rollout_path
            Test-TaskspaceOrdinaryToolBeforeBindingInRollout $rolloutPath
        } else { $false }
        graph_health_path = $graphHealthPath
        graph_health_warnings = @($graphHealthReport.warnings)
        decision_count = [int]$graphHealthReport.decision_count
        decision_density = [double]$graphHealthReport.decision_density
        accepted_results = [int]$graphHealthReport.accepted_result_count
        unreviewed_results = [int]$graphHealthReport.unreviewed_result_count
        questioned_or_invalid_results = [int]$graphHealthReport.questioned_or_invalid_result_count
        result_adoption_rate = if ($null -ne $graphHealthReport.result_adoption_rate) { [double]$graphHealthReport.result_adoption_rate } else { 0.0 }
        result_adoption_metric_state = if ($graphHealthReport.PSObject.Properties.Name -contains "metric_availability") { [string]$graphHealthReport.metric_availability.result_adoption } else { "unknown" }
        subagent_decision_yield = [double]$graphHealthReport.subagent_decision_yield
        observability_json = if ($ObservabilityResult) { $ObservabilityResult.observability_json } else { "" }
        rollout_path = if ($ObservabilityResult) { $ObservabilityResult.rollout_path } else { "" }
    }
    Set-TaskspaceLifecycleClassification $metrics | Out-Null
    $metrics
}

function Set-TaskspaceLifecycleClassification {
    param([Parameter(Mandatory = $true)]$Metrics)

    if (-not ($Metrics.PSObject.Properties.Name -contains "taskspace_profile_hard_stop_seen")) {
        $legacyHardStopSeen = $false
        foreach ($propertyName in @("rollout_path", "last_message_path", "stderr_path")) {
            if (-not ($Metrics.PSObject.Properties.Name -contains $propertyName)) { continue }
            $scanPath = [string]$Metrics.$propertyName
            if ([string]::IsNullOrWhiteSpace($scanPath) -or -not (Test-Path -LiteralPath $scanPath)) { continue }
            if (Select-String -LiteralPath $scanPath -SimpleMatch -Quiet -Pattern @(
                    "TaskSpaceProviderBudgetHardStopV1",
                    "TaskSpace provider budget hard stop",
                    "provider_request_hard_limit_exceeded"
                )) {
                $legacyHardStopSeen = $true
                break
            }
        }
        $Metrics | Add-Member -NotePropertyName taskspace_profile_hard_stop_seen -NotePropertyValue $legacyHardStopSeen -Force
    }
    $execTimedOut = ($Metrics.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$Metrics.exec_timed_out)
    $profileHardStop = ($Metrics.PSObject.Properties.Name -contains "taskspace_profile_hard_stop_seen" -and [bool]$Metrics.taskspace_profile_hard_stop_seen)
    $execExitCode = if ($Metrics.PSObject.Properties.Name -contains "exec_exit_code") { [int]$Metrics.exec_exit_code } else { -1 }
    $samplingInterrupted = ($execTimedOut -or $profileHardStop -or $execExitCode -ne 0)
    $interruptionSource = if ($profileHardStop) {
        "taskspace_profile_hard_stop"
    } elseif ($execTimedOut) {
        "agent_exec_timeout"
    } elseif ($execExitCode -ne 0) {
        "exec_nonzero"
    } else {
        "none"
    }
    $agentFinalObserved = ($Metrics.PSObject.Properties.Name -contains "agent_final_observed" -and [bool]$Metrics.agent_final_observed)
    $completionStatus = if ($samplingInterrupted) { "interrupted" } elseif ($agentFinalObserved) { "complete" } else { "incomplete" }
    $validationSkipped = ($Metrics.PSObject.Properties.Name -contains "public_validation_skipped" -and [bool]$Metrics.public_validation_skipped)
    $publicExitCode = if ($Metrics.PSObject.Properties.Name -contains "public_validation_exit_code") { [int]$Metrics.public_validation_exit_code } else { -1 }
    $oracleExitCode = if ($Metrics.PSObject.Properties.Name -contains "hidden_oracle_exit_code") { [int]$Metrics.hidden_oracle_exit_code } else { -1 }
    $externalValidationStatus = if ($validationSkipped) {
        "skipped"
    } elseif ($publicExitCode -eq 124) {
        "timeout"
    } elseif ($publicExitCode -eq 0 -and $oracleExitCode -eq 0) {
        "passed"
    } else {
        "failed"
    }
    $businessSuccess = ($Metrics.PSObject.Properties.Name -contains "business_success" -and [bool]$Metrics.business_success)
    $utilityEligible = ($businessSuccess -and $completionStatus -eq "complete" -and -not $samplingInterrupted -and $externalValidationStatus -eq "passed")

    $Metrics | Add-Member -NotePropertyName sampling_interrupted -NotePropertyValue $samplingInterrupted -Force
    $Metrics | Add-Member -NotePropertyName interruption_source -NotePropertyValue $interruptionSource -Force
    $Metrics | Add-Member -NotePropertyName agent_completion_status -NotePropertyValue $completionStatus -Force
    $Metrics | Add-Member -NotePropertyName external_validation_status -NotePropertyValue $externalValidationStatus -Force
    $Metrics | Add-Member -NotePropertyName validator_source -NotePropertyValue "benchmark_public_and_hidden" -Force
    $Metrics | Add-Member -NotePropertyName utility_eligible -NotePropertyValue $utilityEligible -Force
    $Metrics
}
