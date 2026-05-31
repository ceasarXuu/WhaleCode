function Get-TaskspaceDiffText {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$DiffPath
    )
    Push-Location $RepoDir
    try {
        $diff = git diff -- .
        $diff | Set-Content -LiteralPath $DiffPath -Encoding UTF8
        return ($diff -join "`n")
    } finally {
        Pop-Location
    }
}

function Export-TaskspaceObservabilityIfAvailable {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$JsonlPath,
        [Parameter(Mandatory = $true)][datetime]$StartedAt,
        [AllowEmptyString()][string]$ThreadId = ""
    )
    $rollout = Find-LatestRollout $StartedAt $ThreadId
    if (-not $rollout) {
        return [pscustomobject]@{ exit_code = -1; rollout_path = ""; observability_json = ""; observability = $null }
    }
    $rolloutCopy = Join-Path $ArtifactDir "rollout.jsonl"
    Copy-Item -LiteralPath $rollout.FullName -Destination $rolloutCopy -Force
    $obsDir = New-Dir (Join-Path $ArtifactDir "observability")
    $stdoutPath = Join-Path $ArtifactDir "observability.stdout.log"
    $stderrPath = Join-Path $ArtifactDir "observability.stderr.log"
    $exportScript = Join-Path $RepoRoot "scripts\export-action-map-observability.ps1"
    $exitCode = Invoke-RealProcess "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $exportScript, "-RolloutPath", $rolloutCopy, "-JsonlPath", $JsonlPath, "-OutputDir", $obsDir) $RepoDir $stdoutPath $stderrPath 180
    $jsonPath = Join-Path $obsDir "action-map-observability.json"
    $obs = if (Test-Path -LiteralPath $jsonPath) { Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonPath | ConvertFrom-Json } else { $null }
    [pscustomobject]@{ exit_code = $exitCode; rollout_path = $rolloutCopy; observability_json = $jsonPath; observability = $obs }
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
    $obs = if ($ObservabilityResult) { $ObservabilityResult.observability } else { $null }
    $graphHealth = Get-TaskspaceGraphHealth $obs
    [pscustomobject]@{
        mode = $Side.Name
        logical_mode = $Side.LogicalMode
        exec_exit_code = $Exec.exit_code
        public_validation_exit_code = $Validation.exit_code
        hidden_oracle_exit_code = $Oracle.exit_code
        wall_time_ms = $Exec.wall_time_ms
        tool_call_count = $commandStats.Completed
        failed_tool_call_count = $commandStats.Failed
        changed_paths = @(Get-ChangedPathsFromDiff $diffText)
        business_success = ($Exec.exit_code -eq 0 -and $Validation.exit_code -eq 0 -and $Oracle.exit_code -eq 0)
        invalid_prompt = $false
        invalid_pair = $false
        harness_failure = $false
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
        subagent_results = if ($obs) { @($obs.nodes | ForEach-Object { @($_.results | Where-Object { $_.sourceThreadId }) }).Count } else { 0 }
        open_leaf_nodes = $graphHealth.OpenLeafNodeCount
        ordinary_before_binding = if ($Side.LogicalMode -eq "taskspace" -and $ObservabilityResult) {
            $rolloutPath = [string]$ObservabilityResult.rollout_path
            $rolloutText = if ($rolloutPath -and (Test-Path -LiteralPath $rolloutPath)) { Get-Content -Raw -Encoding UTF8 -LiteralPath $rolloutPath } else { "" }
            (Get-SuccessfulTaskspaceOrdering $rolloutText).OrdinaryToolBeforeBinding
        } else { $false }
        observability_json = if ($ObservabilityResult) { $ObservabilityResult.observability_json } else { "" }
        rollout_path = if ($ObservabilityResult) { $ObservabilityResult.rollout_path } else { "" }
    }
}
