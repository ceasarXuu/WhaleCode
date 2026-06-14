$ErrorActionPreference = "Stop"

function Read-TaskspaceJsonIfPresent {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $null }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-TaskspaceRuntimeMs {
    param($Object, [string[]]$Fields)
    foreach ($field in $Fields) {
        if ($Object -and $Object.PSObject.Properties.Name -contains $field -and $null -ne $Object.$field) {
            return [int64]$Object.$field
        }
    }
    0
}

function Test-TaskspaceProperty {
    param($Object, [string]$Name)
    $null -ne $Object -and $null -ne $Object.PSObject.Properties[$Name]
}

function Get-TaskspacePropertyValue {
    param($Object, [string]$Name, $Default = "")
    if (Test-TaskspaceProperty $Object $Name) { return $Object.PSObject.Properties[$Name].Value }
    $Default
}

function Get-TaskspaceArrayProperty {
    param($Object, [string]$Name)
    if (-not (Test-TaskspaceProperty $Object $Name)) { return @() }
    @($Object.PSObject.Properties[$Name].Value)
}

function Get-TaskspaceReconstructionClass {
    param(
        [bool]$HasInvalid,
        [int64]$TimeAfterInvalidMs,
        [int64]$SuiteWallMs,
        [int64]$AgentMs,
        [int64]$ValidationMs,
        [int64]$OracleMs,
        [int64]$DockerBuildMs,
        [int64]$DockerRunMs,
        [int64]$DockerCleanupMs,
        [string[]]$MissingFields
    )
    if (@($MissingFields).Count -gt 0) { return "unknown" }
    if ($HasInvalid -and ($SuiteWallMs -eq 0 -or $TimeAfterInvalidMs -ge [int64]($SuiteWallMs * 0.2))) { return "invalid_waste_bound" }
    if ($SuiteWallMs -gt 0 -and $AgentMs -ge [int64]($SuiteWallMs * 0.7)) { return "agent_bound" }
    if ($SuiteWallMs -gt 0 -and ($ValidationMs + $OracleMs) -ge [int64]($SuiteWallMs * 0.3)) { return "validator_bound" }
    if ($SuiteWallMs -gt 0 -and $DockerBuildMs -ge [int64]($SuiteWallMs * 0.15)) { return "docker_build_bound" }
    if ($DockerRunMs -gt $AgentMs -and $DockerRunMs -gt $ValidationMs) { return "docker_run_bound" }
    if ($DockerCleanupMs -gt 0 -and $SuiteWallMs -gt 0 -and $DockerCleanupMs -ge [int64]($SuiteWallMs * 0.1)) { return "storage_bound" }
    "mixed"
}

function Write-TaskspaceRuntimeReconstruction {
    param(
        [Parameter(Mandatory = $true)][string]$SuiteRoot,
        [string]$OutputRoot = ""
    )
    $SuiteRoot = [System.IO.Path]::GetFullPath($SuiteRoot)
    if (-not (Test-Path -LiteralPath $SuiteRoot)) { throw "SuiteRoot not found: $SuiteRoot" }
    if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
        $OutputRoot = Join-Path $SuiteRoot ("runtime-reconstruction\{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
    }
    New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
    $jsonPath = Join-Path $OutputRoot "runtime-reconstruction.json"
    $mdPath = Join-Path $OutputRoot "runtime-reconstruction.md"
    $suiteHealthPath = Join-Path $SuiteRoot "suite-health.json"
    $suiteTimingPath = Join-Path $SuiteRoot "suite-timing.json"
    $suiteHealth = Read-TaskspaceJsonIfPresent $suiteHealthPath
    $suiteTiming = Read-TaskspaceJsonIfPresent $suiteTimingPath
    $missing = New-Object System.Collections.Generic.List[string]
    if (-not $suiteHealth) { $missing.Add("suite-health.json") }
    if (-not $suiteTiming) { $missing.Add("suite-timing.json") }
    [object[]]$statuses = @(Get-TaskspaceArrayProperty $suiteHealth "sample_statuses")
    $sampleRows = New-Object System.Collections.Generic.List[object]
    $firstInvalidIndex = -1
    for ($i = 0; $i -lt $statuses.Count; $i++) {
        $status = $statuses[$i]
        $sampleRoot = [string](Get-TaskspacePropertyValue $status "sample_root" "")
        $skippedReason = [string](Get-TaskspacePropertyValue $status "skipped_reason" "")
        $abortScope = [string](Get-TaskspacePropertyValue $status "abort_scope" "")
        $runValidity = [string](Get-TaskspacePropertyValue $status "run_validity" "")
        $suiteLevelAbort = $abortScope -eq "suite" -and $runValidity -eq "invalid_harness" -and ([System.IO.Path]::GetFullPath($sampleRoot) -eq $SuiteRoot)
        $timingPath = if ($sampleRoot -and -not $suiteLevelAbort) { Join-Path $sampleRoot "sample-timing.json" } else { "" }
        $timing = Read-TaskspaceJsonIfPresent $timingPath
        if ($sampleRoot -and -not $timing -and -not $suiteLevelAbort -and [string]::IsNullOrWhiteSpace($skippedReason)) { $missing.Add("sample-timing:$sampleRoot") }
        $duration = Get-TaskspaceRuntimeMs $timing @("total_pair_duration_ms", "sample_wall_ms", "total_duration_ms")
        $invalid = ($runValidity -eq "invalid_harness")
        if ($invalid -and $firstInvalidIndex -lt 0) { $firstInvalidIndex = $i }
        $sampleRows.Add([pscustomobject]@{
                index = $i
                sample_id = [string](Get-TaskspacePropertyValue $status "sample_id" "")
                run_validity = $runValidity
                skipped_reason = $skippedReason
                duration_ms = $duration
                timing_path = $timingPath
            })
    }
    $suiteWallMs = Get-TaskspaceRuntimeMs $suiteTiming @("suite_wall_ms", "total_pair_duration_ms", "total_duration_ms")
    $timeAfterInvalid = 0
    if ($firstInvalidIndex -ge 0) {
        foreach ($row in @($sampleRows.ToArray() | Where-Object { [int]$_.index -gt $firstInvalidIndex })) {
            $timeAfterInvalid += [int64]$row.duration_ms
        }
    }
    $agentMs = Get-TaskspaceRuntimeMs $suiteTiming @("agent_execution_ms", "agent_duration_ms")
    $validationMs = Get-TaskspaceRuntimeMs $suiteTiming @("public_validation_ms", "public_validation_duration_ms")
    $oracleMs = Get-TaskspaceRuntimeMs $suiteTiming @("hidden_oracle_ms", "hidden_oracle_duration_ms")
    $dockerBuildMs = Get-TaskspaceRuntimeMs $suiteTiming @("docker_build_ms", "docker_build_duration_ms")
    $dockerRunMs = Get-TaskspaceRuntimeMs $suiteTiming @("docker_run_ms", "docker_run_duration_ms")
    $dockerCleanupMs = Get-TaskspaceRuntimeMs $suiteTiming @("docker_cleanup_ms", "docker_cleanup_duration_ms")
    foreach ($field in @("agent_duration_ms", "public_validation_duration_ms", "docker_build_duration_ms", "docker_run_duration_ms", "docker_cleanup_duration_ms")) {
        if ($suiteTiming -and -not (Test-TaskspaceProperty $suiteTiming $field)) { $missing.Add("missing_timing_field:$field") }
    }
    $classification = Get-TaskspaceReconstructionClass ($firstInvalidIndex -ge 0) $timeAfterInvalid $suiteWallMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs @($missing.ToArray())
    $artifact = [ordered]@{
        schema_version = 1
        suite_root = $SuiteRoot
        output_root = [System.IO.Path]::GetFullPath($OutputRoot)
        suite_health_path = $suiteHealthPath
        suite_timing_path = $suiteTimingPath
        bottleneck_classification = $classification
        suite_wall_ms = $suiteWallMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        first_invalid_sample_index = $firstInvalidIndex
        time_after_first_invalid_ms = $timeAfterInvalid
        missing_fields = @($missing.ToArray() | Sort-Object -Unique)
        sample_rows = @($sampleRows.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    $lines = @(
        "# TaskSpace Runtime Reconstruction",
        "",
        "- suite_root: $SuiteRoot",
        "- bottleneck_classification: $classification",
        "- suite_wall_ms: $suiteWallMs",
        "- first_invalid_sample_index: $firstInvalidIndex",
        "- time_after_first_invalid_ms: $timeAfterInvalid",
        "- missing_fields: $(if ($missing.Count -eq 0) { 'none' } else { (@($missing.ToArray() | Sort-Object -Unique) -join ', ') })",
        "",
        "## Samples"
    )
    foreach ($row in @($sampleRows.ToArray())) {
        $lines += "- $($row.index) $($row.sample_id): validity=$($row.run_validity) duration_ms=$($row.duration_ms) skipped=$($row.skipped_reason)"
    }
    $lines | Set-Content -LiteralPath $mdPath -Encoding UTF8
    [pscustomobject]@{ json_path = $jsonPath; markdown_path = $mdPath; artifact = [pscustomobject]$artifact }
}
