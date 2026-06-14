if (-not (Get-Command Get-TaskspaceValidationLifecycle -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "harness-health.ps1")
}

function Get-TaskspaceDiffText {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$DiffPath
    )
    Push-Location $RepoDir
    try {
        $diff = git diff -- .
        Set-Content -LiteralPath $DiffPath -Encoding UTF8 -Value $diff
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
    $absolute = Join-Path $RepoDir ($normalized.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
    if (Test-Path -LiteralPath $absolute -PathType Container) {
        $repoRoot = (Resolve-Path -LiteralPath $RepoDir).Path
        foreach ($file in @(Get-ChildItem -LiteralPath $absolute -Recurse -Force -File -ErrorAction SilentlyContinue)) {
            $relative = $file.FullName.Substring($repoRoot.Length).TrimStart("\", "/").Replace("\", "/")
            if ($relative -like ".git/*") { continue }
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
        $fileInfo = Get-Item -LiteralPath $absolute
        $size = [int64]$fileInfo.Length
        for ($attempt = 0; $attempt -lt 3; $attempt++) {
            try {
                if ($attempt -gt 0) {
                    $hashRetries++
                    Start-Sleep -Milliseconds 100
                    $fileInfo = Get-Item -LiteralPath $absolute -ErrorAction Stop
                    $size = [int64]$fileInfo.Length
                }
                $sha = (Get-FileHash -Algorithm SHA256 -LiteralPath $absolute -ErrorAction Stop).Hash.ToLowerInvariant()
                $hashStatus = "hashed"
                $hashError = ""
                $hashErrorId = ""
                break
            } catch {
                $hashStatus = "read_error"
                $hashError = [string]$_.Exception.Message
                $hashErrorId = [string]$_.FullyQualifiedErrorId
                if ($hashError -match "being used by another process|cannot access the file|in use") {
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
        Add-TaskspaceChangedPath $rows $RepoDir $path "diff" "git_diff"
    }
    Push-Location $RepoDir
    try {
        foreach ($line in @(git status --porcelain=v1 --untracked-files=all -- .)) {
            if ([string]::IsNullOrWhiteSpace($line) -or $line.Length -lt 4) { continue }
            $status = $line.Substring(0, 2)
            $path = $line.Substring(3).Trim()
            if ($path.Contains(" -> ")) { $path = ($path -split ' -> ')[-1].Trim() }
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
        $classifications = @($json.phases | Where-Object { [string]$_.classification -ne "ok" } | ForEach-Object { [string]$_.classification } | Sort-Object -Unique)
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
    $exitCode = Invoke-RealProcess "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $exportScript, "-RolloutPath", $rolloutCopy, "-JsonlPath", $JsonlPath, "-OutputDir", $obsDir, "-ArtifactRoot", $RepoDir) $RepoDir $stdoutPath $stderrPath 180
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
    $graphHealth = Get-TaskspaceGraphHealth $obs
    $graphHealthReport = New-TaskspaceGraphHealthReport $obs $Side.Name $Side.LogicalMode
    $graphHealthPath = Join-Path $Side.ArtifactDir "graph-health.json"
    Write-TaskspaceGraphHealthReport $graphHealthReport $graphHealthPath
    $subagentThreadIds = @()
    if ($obs) {
        $subagentThreadIds = @($obs.nodes | ForEach-Object {
                @($_.leases | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.agentThreadId) } | ForEach-Object { [string]$_.agentThreadId })
            } | Sort-Object -Unique)
    }
    [pscustomobject]@{
        mode = $Side.Name
        logical_mode = $Side.LogicalMode
        exec_exit_code = $Exec.exit_code
        exec_timed_out = ($Exec.PSObject.Properties.Name -contains "timed_out" -and [bool]$Exec.timed_out)
        public_validation_exit_code = $Validation.exit_code
        hidden_oracle_exit_code = $Oracle.exit_code
        wall_time_ms = $Exec.wall_time_ms
        tool_call_count = $commandStats.Completed
        failed_tool_call_count = $commandStats.Failed
        changed_file_inventory = @($changedInventory)
        changed_paths = @($changedInventory | ForEach-Object { $_.path })
        metrics_warnings = @($metricsWarnings)
        metrics_taints = @($metricsTaints)
        docker_build_result_path = $dockerResult.path
        validation_cleanup_result_path = $dockerResult.cleanup_path
        validator_environment_failures = @($dockerResult.classifications)
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
        subagent_results = if ($obs) { @($obs.nodes | ForEach-Object { @($_.results | Where-Object { $subagentThreadIds -contains [string]$_.sourceThreadId }) }).Count } else { 0 }
        open_leaf_nodes = $graphHealth.OpenLeafNodeCount
        ordinary_before_binding = if ($Side.LogicalMode -eq "taskspace" -and $ObservabilityResult) {
            $rolloutPath = [string]$ObservabilityResult.rollout_path
            $rolloutText = if ($rolloutPath -and (Test-Path -LiteralPath $rolloutPath)) { Get-Content -Raw -Encoding UTF8 -LiteralPath $rolloutPath } else { "" }
            (Get-SuccessfulTaskspaceOrdering $rolloutText).OrdinaryToolBeforeBinding
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
}
