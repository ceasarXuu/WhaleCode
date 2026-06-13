$script:TaskspaceInvalidHarnessExitCode = 3

function New-TaskspaceInfraSignature {
    param(
        [string]$Category = "harness_materialization_failure",
        [string]$Stage = "unknown",
        [string]$StableCode = "unknown",
        [string]$Message = "",
        [string]$Side = "",
        [string]$Artifact = ""
    )
    [pscustomobject]@{
        schema_version = 1
        category = $Category
        stage = $Stage
        stable_code = $StableCode
        normalized_message = $Message
        side = $Side
        artifact = $Artifact
        key = "$Category/$StableCode"
    }
}

function Test-TaskspaceFullyQualifiedPath {
    param([AllowEmptyString()][string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    if (-not [System.IO.Path]::IsPathRooted($Path)) { return $false }
    $root = [System.IO.Path]::GetPathRoot($Path)
    -not [string]::IsNullOrWhiteSpace($root)
}

function Test-TaskspaceResolvablePathFrom {
    param(
        [Parameter(Mandatory = $true)][string]$BaseDir,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $candidate = if (Test-TaskspaceFullyQualifiedPath $Path) { $Path } else { Join-Path $BaseDir $Path }
    Test-Path -LiteralPath $candidate
}

function Get-TaskspaceMinimumFreeBytes {
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_BYTES)) {
        return [int64]$env:TASKSPACE_MIN_FREE_BYTES
    }
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_GIB)) {
        return [int64]([double]$env:TASKSPACE_MIN_FREE_GIB * 1GB)
    }
    [int64](20GB)
}

function Get-TaskspaceExistingPathForDisk {
    param([AllowEmptyString()][string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return "" }
    $candidate = [System.IO.Path]::GetFullPath($Path)
    while (-not [string]::IsNullOrWhiteSpace($candidate) -and -not (Test-Path -LiteralPath $candidate)) {
        $parent = Split-Path -Parent $candidate
        if ($parent -eq $candidate) { break }
        $candidate = $parent
    }
    if (Test-Path -LiteralPath $candidate) { return $candidate }
    ""
}

function Get-TaskspaceDiskSpaceChecks {
    param([Parameter(Mandatory = $true)][string[]]$Paths)
    $minimum = Get-TaskspaceMinimumFreeBytes
    $checks = New-Object System.Collections.Generic.List[object]
    $seenRoots = @{}
    foreach ($path in @($Paths)) {
        $existing = Get-TaskspaceExistingPathForDisk $path
        if ([string]::IsNullOrWhiteSpace($existing)) { continue }
        $root = [System.IO.Path]::GetPathRoot($existing)
        if ([string]::IsNullOrWhiteSpace($root) -or $seenRoots.ContainsKey($root)) { continue }
        $seenRoots[$root] = $true
        try {
            $drive = New-Object System.IO.DriveInfo($root)
            $free = [int64]$drive.AvailableFreeSpace
            $checks.Add([pscustomobject]@{
                    root = $root
                    path = $existing
                    free_bytes = $free
                    required_free_bytes = $minimum
                    free_gib = [math]::Round($free / 1GB, 2)
                    required_free_gib = [math]::Round($minimum / 1GB, 2)
                    status = if ($free -lt $minimum) { "fail" } else { "pass" }
                })
        } catch {
            $checks.Add([pscustomobject]@{
                    root = $root
                    path = $existing
                    free_bytes = 0
                    required_free_bytes = $minimum
                    free_gib = 0
                    required_free_gib = [math]::Round($minimum / 1GB, 2)
                    status = "fail"
                    error = [string]$_.Exception.Message
                })
        }
    }
    @($checks.ToArray())
}

function Get-TaskspaceHarnessTextSignature {
    param(
        [AllowEmptyString()][string]$Text = "",
        [string]$Stage = "validator_pretest",
        [string]$Side = "",
        [string]$Artifact = ""
    )
    if ($Text -match "docker command is required|Docker backend unavailable|Requested WSL Docker backend is unavailable|Requested native Docker backend is unavailable|Unsupported TASKSPACE_DOCKER_BACKEND|getpwnam\(root\) failed|getpwuid\(0\) failed|Wsl/Service/E_UNEXPECTED|I/O error @util\.cpp") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "docker_backend_unavailable" "Docker backend unavailable" $Side $Artifact
    }
    if ($Text -match "Resolve-Path|Cannot find path|PathNotFound") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "path_unresolvable" "Path resolution failed" $Side $Artifact
    }
    if ($Text -match "run-tests script not found|validator script not found") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "validator_source_missing" "Validator source missing" $Side $Artifact
    }
    if ($Text -match "uv[-_ ]cache|uv-x86_64|install\.sh") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "uv_cache_missing" "uv cache unavailable" $Side $Artifact
    }
    return $null
}

function Get-TaskspaceValidationText {
    param($Validation)
    $combined = ""
    foreach ($path in @($Validation.stdout_path, $Validation.stderr_path)) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            $combined += "`n" + (Get-Content -Raw -Encoding UTF8 -LiteralPath $path)
        }
    }
    $combined
}

function Get-TaskspaceValidatorProbeResult {
    param($Validation)
    $combined = Get-TaskspaceValidationText $Validation
    $probePath = ""
    $probeMatch = [regex]::Match($combined, "(?m)^validator_probe_result_path=(.+)$")
    if ($probeMatch.Success) { $probePath = $probeMatch.Groups[1].Value.Trim() }
    $json = if ($probePath -and (Test-Path -LiteralPath $probePath)) {
        try { Get-Content -Raw -Encoding UTF8 -LiteralPath $probePath | ConvertFrom-Json } catch { $null }
    } else { $null }
    [pscustomobject]@{
        path = $probePath
        json = $json
    }
}

function Get-TaskspaceValidationLifecycle {
    param($Validation)
    $combined = Get-TaskspaceValidationText $Validation
    $stages = @([regex]::Matches($combined, "(?m)^validator_lifecycle_stage=([^\r\n]+)\s*$") | ForEach-Object { $_.Groups[1].Value.Trim() })
    $stage = if ($stages.Count -gt 0) { [string]$stages[-1] } else { "unknown" }
    [pscustomobject]@{
        tests_started_seen = ($combined -match "(?m)^validator_tests_started=true\s*$")
        tests_completed_seen = ($combined -match "(?m)^validator_tests_completed=true\s*$")
        validation_lifecycle_stage = $stage
    }
}

function Get-TaskspaceInfraSignatureFromMetrics {
    param($Metrics)
    if ($null -eq $Metrics) { return $null }
    if ($Metrics.PSObject.Properties.Name -contains "infra_signature" -and $Metrics.infra_signature) {
        return $Metrics.infra_signature
    }
    foreach ($failure in @($Metrics.validator_environment_failures)) {
        if ([string]::IsNullOrWhiteSpace([string]$failure)) { continue }
        if ([string]$failure -match "docker") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "docker_backend_unavailable" "Docker backend unavailable" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "path_unresolvable|Resolve-Path") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "path_unresolvable" "Path resolution failed" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "uv_cache") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "uv_cache_missing" "uv cache unavailable" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "validator_source") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "validator_source_missing" "Validator source missing" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
    }
    $null
}

function Test-TaskspaceHardInfraSignature {
    param($Signature)
    if ($null -eq $Signature) { return $false }
    [string]$Signature.stable_code -in @("relative_materialized_path", "path_unresolvable", "validator_source_missing", "uv_cache_missing", "docker_backend_unavailable", "runtime_manifest_missing", "validator_probe_failed", "workspace_baseline_git_failed", "workspace_fixture_copy_failed", "workspace_materialization_failed", "disk_space_low")
}

function Get-TaskspaceSentinelAbortDecision {
    param(
        [Parameter(Mandatory = $true)]$StandardMetrics,
        [Parameter(Mandatory = $true)]$TaskspaceMetrics
    )
    $standardPretest = ($StandardMetrics.PSObject.Properties.Name -contains "pretest_failure" -and [bool]$StandardMetrics.pretest_failure)
    $taskspacePretest = ($TaskspaceMetrics.PSObject.Properties.Name -contains "pretest_failure" -and [bool]$TaskspaceMetrics.pretest_failure)
    $standardSig = Get-TaskspaceInfraSignatureFromMetrics $StandardMetrics
    $taskspaceSig = Get-TaskspaceInfraSignatureFromMetrics $TaskspaceMetrics
    $standardHard = $standardPretest -and (Test-TaskspaceHardInfraSignature $standardSig)
    $taskspaceHard = $taskspacePretest -and (Test-TaskspaceHardInfraSignature $taskspaceSig)
    $sameKey = ($standardSig -and $taskspaceSig -and [string]$standardSig.key -eq [string]$taskspaceSig.key)
    if ($standardHard -and $taskspaceHard -and $sameKey) {
        return [pscustomobject]@{ abort = $true; reason = "same_infra_signature_both_sides"; signature = $standardSig }
    }
    if ($standardHard) {
        return [pscustomobject]@{ abort = $true; reason = "standard_pretest_infra_failure"; signature = $standardSig }
    }
    if ($taskspaceHard) {
        return [pscustomobject]@{ abort = $true; reason = "taskspace_pretest_infra_failure"; signature = $taskspaceSig }
    }
    [pscustomobject]@{ abort = $false; reason = ""; signature = $null }
}

function Write-TaskspaceHarnessHealth {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Health
    )
    ($Health | ConvertTo-Json -Depth 20) | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-TaskspaceHarnessHealth {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$RunDir,
        [string]$ScenarioBaseDir = ""
    )
    $findings = New-Object System.Collections.Generic.List[object]
    $checked = New-Object System.Collections.Generic.List[object]
    foreach ($pathInfo in @(
        @{ name = "prompt_path"; path = [string]$Manifest.PromptPath; required = $true },
        @{ name = "fixture_dir"; path = [string]$Manifest.FixtureDir; required = $true }
    )) {
        $exists = -not [string]::IsNullOrWhiteSpace($pathInfo.path) -and (Test-Path -LiteralPath $pathInfo.path)
        $checked.Add([pscustomobject]@{ name = $pathInfo.name; path = $pathInfo.path; exists = $exists; fully_qualified = (Test-TaskspaceFullyQualifiedPath $pathInfo.path) })
        if ($pathInfo.required -and -not $exists) {
            $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "path_unresolvable"; message = "$($pathInfo.name) is missing"; path = $pathInfo.path })
        }
    }
    $external = $Manifest.ExternalBenchmark
    if ($external -and $external.PSObject.Properties.Name -contains "adapter_metadata") {
        $meta = $external.adapter_metadata
        foreach ($prop in @("uv_cache_root", "validator_source_dir", "fixture_source")) {
            if (-not ($meta.PSObject.Properties.Name -contains $prop)) { continue }
            $path = [string]$meta.$prop
            $exists = -not [string]::IsNullOrWhiteSpace($path) -and (Test-Path -LiteralPath $path)
            $fq = Test-TaskspaceFullyQualifiedPath $path
            $checked.Add([pscustomobject]@{ name = $prop; path = $path; exists = $exists; fully_qualified = $fq })
            if (-not $fq) {
                $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "relative_materialized_path"; message = "$prop must be absolute"; path = $path })
            } elseif (-not $exists) {
                $code = if ($prop -eq "uv_cache_root") { "uv_cache_missing" } elseif ($prop -eq "validator_source_dir") { "validator_source_missing" } else { "path_unresolvable" }
                $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = $code; message = "$prop is missing"; path = $path })
            }
        }
    }
    $spacePaths = New-Object System.Collections.Generic.List[string]
    $spacePaths.Add($RunDir)
    if (-not [string]::IsNullOrWhiteSpace($ScenarioBaseDir)) { $spacePaths.Add($ScenarioBaseDir) }
    foreach ($pathInfo in @($checked.ToArray())) {
        if (-not [string]::IsNullOrWhiteSpace([string]$pathInfo.path)) { $spacePaths.Add([string]$pathInfo.path) }
    }
    $dockerCommand = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerCommand -and -not [string]::IsNullOrWhiteSpace([string]$dockerCommand.Source)) {
        $spacePaths.Add([string]$dockerCommand.Source)
    }
    if (Test-Path -LiteralPath "D:\whale-docker") { $spacePaths.Add("D:\whale-docker") }
    $diskSpaceChecks = Get-TaskspaceDiskSpaceChecks @($spacePaths.ToArray())
    foreach ($space in @($diskSpaceChecks | Where-Object { [string]$_.status -eq "fail" })) {
        $findings.Add([pscustomobject]@{
                severity = "fail"
                stable_code = "disk_space_low"
                message = "Free disk space below TaskSpace preflight minimum on $($space.root): $($space.free_gib) GiB available, $($space.required_free_gib) GiB required"
                path = [string]$space.path
                root = [string]$space.root
                free_bytes = [int64]$space.free_bytes
                required_free_bytes = [int64]$space.required_free_bytes
            })
    }
    $hardFindings = @($findings.ToArray() | Where-Object { [string]$_.severity -eq "fail" })
    [pscustomobject]@{
        schema_version = 1
        status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }
        run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }
        findings = @($findings.ToArray())
        checked_paths = @($checked.ToArray())
        disk_space_checks = @($diskSpaceChecks)
        generated_at = (Get-Date).ToString("o")
        run_dir = $RunDir
        scenario_base_dir = $ScenarioBaseDir
    }
}
