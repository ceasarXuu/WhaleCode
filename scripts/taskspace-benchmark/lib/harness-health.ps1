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
    $script:TaskspaceDiskThresholdError = ""
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_BYTES)) {
        try { return [int64]$env:TASKSPACE_MIN_FREE_BYTES } catch {
            $script:TaskspaceDiskThresholdError = "Invalid TASKSPACE_MIN_FREE_BYTES: $env:TASKSPACE_MIN_FREE_BYTES"
            return [int64](20GB)
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_GIB)) {
        try { return [int64]([double]$env:TASKSPACE_MIN_FREE_GIB * 1GB) } catch {
            $script:TaskspaceDiskThresholdError = "Invalid TASKSPACE_MIN_FREE_GIB: $env:TASKSPACE_MIN_FREE_GIB"
            return [int64](20GB)
        }
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

function Get-TaskspaceSupplementalDiskPaths {
    $paths = New-Object System.Collections.Generic.List[string]
    foreach ($candidate in @($env:UV_CACHE_DIR, $env:WHALE_TASKSPACE_UV_CACHE_DIR)) {
        if (-not [string]::IsNullOrWhiteSpace([string]$candidate)) { $paths.Add([string]$candidate) }
    }
    $dockerCommand = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerCommand -and -not [string]::IsNullOrWhiteSpace([string]$dockerCommand.Source)) { $paths.Add([string]$dockerCommand.Source) }
    if (Test-Path -LiteralPath "D:\whale-docker") { $paths.Add("D:\whale-docker") }
    @($paths.ToArray())
}

function Invoke-TaskspaceShortCommand {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSeconds = 20
    )
    $job = Start-Job -ScriptBlock {
        param([string]$InnerCommand, [string[]]$InnerArguments)
        $output = & $InnerCommand @InnerArguments 2>&1
        [pscustomobject]@{ exit_code = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }; output = @($output | ForEach-Object { [string]$_ }) }
    } -ArgumentList $Command, $Arguments
    if (-not (Wait-Job -Job $job -Timeout $TimeoutSeconds)) {
        Stop-Job -Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        return [pscustomobject]@{ exit_code = 124; output = @("timeout after $TimeoutSeconds seconds") }
    }
    $result = Receive-Job -Job $job
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    if ($null -eq $result) { return [pscustomobject]@{ exit_code = 1; output = @("no output") } }
    $result | Select-Object -First 1
}

function Get-TaskspaceDockerStorageChecks {
    param([int64]$MinimumFreeBytes = (Get-TaskspaceMinimumFreeBytes))
    $checks = New-Object System.Collections.Generic.List[object]
    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }
    if (-not (Get-Command wsl -ErrorAction SilentlyContinue)) { return @($checks.ToArray()) }
    $info = Invoke-TaskspaceShortCommand "wsl" @("-d", $distro, "--", "sh", "-lc", "docker info --format '{{.DockerRootDir}}' 2>/dev/null || true") 20
    $dockerRoot = (@($info.output) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace([string]$dockerRoot)) { $dockerRoot = "/var/lib/docker" }
    $paths = @("/", [string]$dockerRoot) | Sort-Object -Unique
    foreach ($path in $paths) {
        $df = Invoke-TaskspaceShortCommand "wsl" @("-d", $distro, "--", "df", "-Pk", $path) 20
        $line = @($df.output | Where-Object { [string]$_ -match "^\S+\s+\d+\s+\d+\s+\d+\s+\d+%" } | Select-Object -Last 1)
        if ([int]$df.exit_code -ne 0 -or -not $line) {
            $checks.Add([pscustomobject]@{ kind = "wsl_df"; distro = $distro; path = $path; status = "fail"; free_bytes = 0; required_free_bytes = $MinimumFreeBytes; message = (@($df.output) -join " ") })
            continue
        }
        $parts = ([string]$line).Trim() -split "\s+"
        $free = [int64]$parts[3] * 1024
        $checks.Add([pscustomobject]@{
                kind = "wsl_df"
                distro = $distro
                path = $path
                docker_root = [string]$dockerRoot
                free_bytes = $free
                required_free_bytes = $MinimumFreeBytes
                free_gib = [math]::Round($free / 1GB, 2)
                required_free_gib = [math]::Round($MinimumFreeBytes / 1GB, 2)
                status = if ($free -lt $MinimumFreeBytes) { "fail" } else { "pass" }
            })
    }
    @($checks.ToArray())
}

function New-TaskspaceDiskHealth {
    param(
        [Parameter(Mandatory = $true)][string[]]$Paths,
        [string]$Stage = "preflight"
    )
    $minimum = Get-TaskspaceMinimumFreeBytes
    $hostChecks = Get-TaskspaceDiskSpaceChecks @($Paths + (Get-TaskspaceSupplementalDiskPaths))
    $dockerChecks = Get-TaskspaceDockerStorageChecks $minimum
    $findings = New-Object System.Collections.Generic.List[object]
    if (-not [string]::IsNullOrWhiteSpace($script:TaskspaceDiskThresholdError)) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "disk_space_threshold_invalid"; message = $script:TaskspaceDiskThresholdError; path = ""; stage = $Stage })
    }
    $allChecks = @($hostChecks) + @($dockerChecks)
    foreach ($space in @($allChecks | Where-Object { [string]$_.status -eq "fail" })) {
        $label = if ($space.PSObject.Properties.Name -contains "root") { [string]$space.root } else { "$($space.distro):$($space.path)" }
        $findings.Add([pscustomobject]@{
                severity = "fail"; stable_code = "disk_space_low"; stage = $Stage
                message = "Free disk space below TaskSpace preflight minimum on ${label}: $($space.free_gib) GiB available, $($space.required_free_gib) GiB required"
                path = [string]$space.path; root = $label; free_bytes = [int64]$space.free_bytes; required_free_bytes = [int64]$space.required_free_bytes
            })
    }
    $hardFindings = @($findings.ToArray())
    [pscustomobject]@{ schema_version = 1; status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }; run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }; findings = @($findings.ToArray()); disk_space_checks = @($hostChecks); docker_storage_checks = @($dockerChecks); generated_at = (Get-Date).ToString("o") }
}

function New-TaskspaceHarnessAbortSummaryLines {
    param([string]$Title, [string]$Phase, $Finding, $Signature, [string]$HealthPath)
    $lines = @("# $Title", "", "- run_validity: invalid_harness", "- abort_phase: $Phase", "- reason: $($Finding.message)", "- infra_signature: $($Signature.key)", "- harness_health: $HealthPath")
    if ([string]$Finding.stable_code -eq "disk_space_low") {
        $lines += @(
            "- failed_root: $($Finding.root)",
            "- failed_path: $($Finding.path)",
            "- free_gib: $([math]::Round([int64]$Finding.free_bytes / 1GB, 2))",
            "- required_gib: $([math]::Round([int64]$Finding.required_free_bytes / 1GB, 2))",
            "- override_bytes_env: TASKSPACE_MIN_FREE_BYTES",
            "- override_gib_env: TASKSPACE_MIN_FREE_GIB",
            "- likely_cleanup_paths: run root, repo target directories, Docker/WSL data root, D:\whale-docker"
        )
    } elseif ([string]$Finding.stable_code -eq "disk_space_threshold_invalid") {
        $lines += @(
            "- override_bytes_env: TASKSPACE_MIN_FREE_BYTES",
            "- override_gib_env: TASKSPACE_MIN_FREE_GIB"
        )
    }
    $lines
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
    $timeoutPhaseMatch = [regex]::Match($combined, "(?m)^taskspace_validation_timeout_phase=([^\r\n]+)\s*$")
    $testsStartedAtMatch = [regex]::Match($combined, "(?m)^taskspace_tests_started_at=([^\r\n]+)\s*$")
    $testsCompletedAtMatch = [regex]::Match($combined, "(?m)^taskspace_tests_completed_at=([^\r\n]+)\s*$")
    [pscustomobject]@{
        tests_started_seen = ($combined -match "(?m)^validator_tests_started=true\s*$")
        tests_completed_seen = ($combined -match "(?m)^validator_tests_completed=true\s*$")
        validation_lifecycle_stage = $stage
        validation_timeout_phase = if ($timeoutPhaseMatch.Success) { $timeoutPhaseMatch.Groups[1].Value.Trim() } else { "" }
        tests_started_at = if ($testsStartedAtMatch.Success) { $testsStartedAtMatch.Groups[1].Value.Trim() } else { "" }
        tests_completed_at = if ($testsCompletedAtMatch.Success) { $testsCompletedAtMatch.Groups[1].Value.Trim() } else { "" }
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
    [string]$Signature.stable_code -in @("relative_materialized_path", "path_unresolvable", "validator_source_missing", "uv_cache_missing", "docker_backend_unavailable", "runtime_manifest_missing", "validator_probe_failed", "workspace_baseline_git_failed", "workspace_fixture_copy_failed", "workspace_materialization_failed", "disk_space_low", "disk_space_threshold_invalid")
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
    $diskHealth = New-TaskspaceDiskHealth @($spacePaths.ToArray()) "preflight"
    foreach ($finding in @($diskHealth.findings)) { $findings.Add($finding) }
    $hardFindings = @($findings.ToArray() | Where-Object { [string]$_.severity -eq "fail" })
    [pscustomobject]@{
        schema_version = 1
        status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }
        run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }
        findings = @($findings.ToArray())
        checked_paths = @($checked.ToArray())
        disk_space_checks = @($diskHealth.disk_space_checks)
        docker_storage_checks = @($diskHealth.docker_storage_checks)
        generated_at = (Get-Date).ToString("o")
        run_dir = $RunDir
        scenario_base_dir = $ScenarioBaseDir
    }
}
