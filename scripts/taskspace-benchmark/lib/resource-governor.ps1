$ErrorActionPreference = "Stop"

function New-TaskspaceResourceGovernorConfig {
    param(
        [int]$MaxParallelSamples = 1,
        [int]$MaxParallelPairsPerSample = 1,
        [int]$MaxParallelValidationsPerPair = 1,
        [int]$MaxDockerConcurrency = 1,
        [int]$MaxModelConcurrency = 1,
        [double]$DiskReserveGb = 0
    )
    $values = @{
        MaxParallelSamples = $MaxParallelSamples
        MaxParallelPairsPerSample = $MaxParallelPairsPerSample
        MaxParallelValidationsPerPair = $MaxParallelValidationsPerPair
        MaxDockerConcurrency = $MaxDockerConcurrency
        MaxModelConcurrency = $MaxModelConcurrency
    }
    $errors = New-Object System.Collections.Generic.List[string]
    foreach ($name in @($values.Keys | Sort-Object)) {
        if ([int]$values[$name] -lt 1) { $errors.Add("$name must be >= 1") }
    }
    if ($DiskReserveGb -lt 0) { $errors.Add("DiskReserveGb must be >= 0") }
    [pscustomobject]@{
        schema_version = 1
        max_parallel_samples = $MaxParallelSamples
        max_parallel_pairs_per_sample = $MaxParallelPairsPerSample
        max_parallel_validations_per_pair = $MaxParallelValidationsPerPair
        max_docker_concurrency = $MaxDockerConcurrency
        max_model_concurrency = $MaxModelConcurrency
        disk_reserve_gb = $DiskReserveGb
        disk_reserve_bytes = [int64]($DiskReserveGb * 1GB)
        valid = ($errors.Count -eq 0)
        errors = @($errors.ToArray())
    }
}

function Test-TaskspaceResourceGovernorSerialOnly {
    param($Config)
    $unsupported = New-Object System.Collections.Generic.List[string]
    if ([int]$Config.max_parallel_samples -ne 1) { $unsupported.Add("MaxParallelSamples") }
    if ([int]$Config.max_parallel_pairs_per_sample -ne 1) { $unsupported.Add("MaxParallelPairsPerSample") }
    if ([int]$Config.max_parallel_validations_per_pair -ne 1) { $unsupported.Add("MaxParallelValidationsPerPair") }
    if ([int]$Config.max_docker_concurrency -ne 1) { $unsupported.Add("MaxDockerConcurrency") }
    if ([int]$Config.max_model_concurrency -ne 1) { $unsupported.Add("MaxModelConcurrency") }
    [pscustomobject]@{
        serial_only = ($unsupported.Count -eq 0)
        unsupported_parallel_fields = @($unsupported.ToArray())
        status = if ($unsupported.Count -eq 0) { "pass" } else { "unsupported_parallelism" }
    }
}

function Test-TaskspaceDiskReservation {
    param(
        [Parameter(Mandatory = $true)][string[]]$Paths,
        [int64]$ReserveBytes = 0
    )
    $checks = New-Object System.Collections.Generic.List[object]
    $failures = New-Object System.Collections.Generic.List[object]
    foreach ($path in @($Paths | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })) {
        $existing = Get-TaskspaceExistingPathForDisk $path
        if ([string]::IsNullOrWhiteSpace($existing)) {
            $failure = [pscustomobject]@{ path = $path; status = "fail"; stable_code = "disk_reservation_path_unresolved"; free_bytes = 0; reserve_bytes = $ReserveBytes }
            $checks.Add($failure)
            $failures.Add($failure)
            continue
        }
        try {
            $drive = New-Object System.IO.DriveInfo([System.IO.Path]::GetPathRoot($existing))
            $free = [int64]$drive.AvailableFreeSpace
            $check = [pscustomobject]@{
                path = $existing
                root = [System.IO.Path]::GetPathRoot($existing)
                free_bytes = $free
                reserve_bytes = $ReserveBytes
                free_gib = [math]::Round($free / 1GB, 2)
                reserve_gib = [math]::Round($ReserveBytes / 1GB, 2)
                status = if ($free -lt $ReserveBytes) { "fail" } else { "pass" }
                stable_code = if ($free -lt $ReserveBytes) { "disk_reservation_insufficient" } else { "" }
            }
            $checks.Add($check)
            if ([string]$check.status -eq "fail") { $failures.Add($check) }
        } catch {
            $failure = [pscustomobject]@{ path = $existing; status = "fail"; stable_code = "disk_reservation_probe_failed"; error = [string]$_.Exception.Message; free_bytes = 0; reserve_bytes = $ReserveBytes }
            $checks.Add($failure)
            $failures.Add($failure)
        }
    }
    [pscustomobject]@{
        schema_version = 1
        status = if ($failures.Count -eq 0) { "pass" } else { "fail" }
        reserve_bytes = $ReserveBytes
        checks = @($checks.ToArray())
        failures = @($failures.ToArray())
    }
}

function New-TaskspaceResourceWaitSnapshot {
    param(
        [int64]$DockerTokenWaitMs = 0,
        [int64]$ValidationTokenWaitMs = 0,
        [int64]$DiskReservationWaitMs = 0,
        [int64]$CacheLockWaitMs = 0
    )
    [pscustomobject]@{
        docker_token_wait_ms = $DockerTokenWaitMs
        validation_token_wait_ms = $ValidationTokenWaitMs
        disk_reservation_wait_ms = $DiskReservationWaitMs
        cache_lock_wait_ms = $CacheLockWaitMs
        resource_wait_ms_total = [int64]($DockerTokenWaitMs + $ValidationTokenWaitMs + $DiskReservationWaitMs + $CacheLockWaitMs)
    }
}

function Write-TaskspaceParallelismArtifact {
    param(
        [Parameter(Mandatory = $true)][string]$SuiteRoot,
        [Parameter(Mandatory = $true)]$Config,
        $SerialGuard,
        $DiskReservation,
        $WaitSnapshot
    )
    if (-not $SerialGuard) { $SerialGuard = Test-TaskspaceResourceGovernorSerialOnly $Config }
    if (-not $WaitSnapshot) { $WaitSnapshot = New-TaskspaceResourceWaitSnapshot }
    $artifact = [ordered]@{
        schema_version = 1
        configured = $Config
        observed = [ordered]@{
            max_parallel_samples = 1
            max_parallel_pairs_per_sample = 1
            max_parallel_validations_per_pair = 1
            max_docker_concurrency = 1
            max_model_concurrency = 1
        }
        serial_only_status = [string]$SerialGuard.status
        unsupported_parallel_fields = @($SerialGuard.unsupported_parallel_fields)
        disk_reservation = $DiskReservation
        wait = $WaitSnapshot
        timing_comparison_valid = [bool]$SerialGuard.serial_only
        resource_governor_status = if ([bool]$Config.valid -and [bool]$SerialGuard.serial_only -and (-not $DiskReservation -or [string]$DiskReservation.status -eq "pass")) { "pass" } else { "blocked" }
        generated_at = (Get-Date).ToString("o")
    }
    $path = Join-Path $SuiteRoot "parallelism.json"
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}
