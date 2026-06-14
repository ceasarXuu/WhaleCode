$ErrorActionPreference = "Stop"

function New-TaskspaceCalibrationGateRow {
    param([string]$Name, [string]$Status, [string]$Reason = "", [string]$Artifact = "")
    [pscustomobject]@{
        name = $Name
        status = $Status
        reason = $Reason
        artifact = $Artifact
    }
}

function Test-TaskspaceJsonField {
    param($Object, [string]$Field)
    $Object -and $Object.PSObject.Properties.Name -contains $Field -and $null -ne $Object.$Field
}

function Get-TaskspaceJsonFile {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $null }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Test-TaskspaceCalibrationIdentityField {
    param($Object, [string]$Field, [string]$Expected, [string]$Scope, [string]$Artifact)
    if ([string]::IsNullOrWhiteSpace($Expected)) { return $null }
    if (-not (Test-TaskspaceJsonField $Object $Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_identity_field_missing:$Field" $Artifact
    }
    if ([string]$Object.$Field -ne $Expected) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_identity_mismatch:$Field" $Artifact
    }
    $null
}

function Test-TaskspaceOnePairTimingEvidence {
    param([string]$Root, [string]$ExpectedTaskListHash = "", [string]$ExpectedSourceVersion = "", [string]$ExpectedProfileHash = "")
    if ([string]::IsNullOrWhiteSpace($Root) -or -not (Test-Path -LiteralPath $Root)) {
        return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_root_missing" $Root
    }
    $pairTimingPath = Get-ChildItem -LiteralPath $Root -Filter "pair-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $sampleTimingPath = Get-ChildItem -LiteralPath $Root -Filter "sample-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $runtimeReportPath = Get-ChildItem -LiteralPath $Root -Filter "runtime-bottleneck.md" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $pairTimingPath -or -not $sampleTimingPath -or -not $runtimeReportPath) {
        return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_timing_artifact_missing" $Root
    }
    $pairTiming = Get-TaskspaceJsonFile $pairTimingPath.FullName
    foreach ($field in @("agent_duration_ms", "public_validation_duration_ms", "bottleneck_classification", "runtime_optimization_status")) {
        if (-not (Test-TaskspaceJsonField $pairTiming $field)) {
            return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_timing_field_missing:$field" $pairTimingPath.FullName
        }
    }
    foreach ($identityCheck in @(
            (Test-TaskspaceCalibrationIdentityField $pairTiming "task_list_hash" $ExpectedTaskListHash "one_pair_smoke" $pairTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $pairTiming "source_version" $ExpectedSourceVersion "one_pair_smoke" $pairTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $pairTiming "profile_hash" $ExpectedProfileHash "one_pair_smoke" $pairTimingPath.FullName)
        )) {
        if ($identityCheck) { return $identityCheck }
    }
    New-TaskspaceCalibrationGateRow "one_pair_smoke" "pass" "" $pairTimingPath.FullName
}

function Test-TaskspaceSerialCalibrationEvidence {
    param([string]$Root, [int]$MinimumSamples = 3, [string]$ExpectedTaskListHash = "", [string]$ExpectedSourceVersion = "", [string]$ExpectedProfileHash = "")
    if ([string]::IsNullOrWhiteSpace($Root) -or -not (Test-Path -LiteralPath $Root)) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_root_missing" $Root
    }
    $suiteTimingPath = Get-ChildItem -LiteralPath $Root -Filter "suite-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $calibrationReportPath = Get-ChildItem -LiteralPath $Root -Filter "runtime-calibration-report.md" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $suiteTimingPath -or -not $calibrationReportPath) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_artifact_missing" $Root
    }
    $suiteTiming = Get-TaskspaceJsonFile $suiteTimingPath.FullName
    if (-not (Test-TaskspaceJsonField $suiteTiming "sample_count") -or [int]$suiteTiming.sample_count -lt $MinimumSamples) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_sample_count_low" $suiteTimingPath.FullName
    }
    foreach ($field in @("timing_quality", "runtime_optimization_status", "bottleneck_classification", "wait_attribution_status")) {
        if (-not (Test-TaskspaceJsonField $suiteTiming $field)) {
            return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_field_missing:$field" $suiteTimingPath.FullName
        }
    }
    foreach ($identityCheck in @(
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "task_list_hash" $ExpectedTaskListHash "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "source_version" $ExpectedSourceVersion "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "profile_hash" $ExpectedProfileHash "serial_calibration" $suiteTimingPath.FullName)
        )) {
        if ($identityCheck) { return $identityCheck }
    }
    New-TaskspaceCalibrationGateRow "serial_calibration" "pass" "" $suiteTimingPath.FullName
}

function Test-TaskspaceParallelSmokeEvidence {
    param([string]$EquivalencePath, [string]$ExpectedTaskListHash = "", [string]$ExpectedSourceVersion = "", [string]$ExpectedProfileHash = "")
    $equivalence = Get-TaskspaceJsonFile $EquivalencePath
    if (-not $equivalence) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_equivalence_missing" $EquivalencePath
    }
    if (-not (Test-TaskspaceJsonField $equivalence "parallel_smoke_score_drift") -or [bool]$equivalence.parallel_smoke_score_drift) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_score_drift" $EquivalencePath
    }
    if (-not (Test-TaskspaceJsonField $equivalence "comparable") -or -not [bool]$equivalence.comparable) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_not_comparable" $EquivalencePath
    }
    if (-not (Test-TaskspaceJsonField $equivalence "drift_count") -or [int]$equivalence.drift_count -ne 0) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_drift_count_nonzero" $EquivalencePath
    }
    if (-not (Test-TaskspaceJsonField $equivalence "compared_sample_ids") -or @($equivalence.compared_sample_ids).Count -eq 0) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_compared_samples_missing" $EquivalencePath
    }
    if (-not (Test-TaskspaceJsonField $equivalence "required_sample_fields") -or @($equivalence.required_sample_fields).Count -eq 0) {
        return New-TaskspaceCalibrationGateRow "parallel_smoke" "fail" "parallel_required_sample_fields_missing" $EquivalencePath
    }
    foreach ($identityCheck in @(
            (Test-TaskspaceCalibrationIdentityField $equivalence "task_list_hash" $ExpectedTaskListHash "parallel_smoke" $EquivalencePath),
            (Test-TaskspaceCalibrationIdentityField $equivalence "source_version" $ExpectedSourceVersion "parallel_smoke" $EquivalencePath),
            (Test-TaskspaceCalibrationIdentityField $equivalence "profile_hash" $ExpectedProfileHash "parallel_smoke" $EquivalencePath)
        )) {
        if ($identityCheck) { return $identityCheck }
    }
    New-TaskspaceCalibrationGateRow "parallel_smoke" "pass" "" $EquivalencePath
}

function Invoke-TaskspaceCalibrationGate {
    param(
        [string]$OnePairSmokeRoot = "",
        [string]$SerialCalibrationRoot = "",
        [string]$ParallelEquivalencePath = "",
        [string]$ExpectedTaskListHash = "",
        [string]$ExpectedSourceVersion = "",
        [string]$ExpectedProfileHash = "",
        [string]$OutputPath = ""
    )
    $rows = @(
        Test-TaskspaceOnePairTimingEvidence $OnePairSmokeRoot $ExpectedTaskListHash $ExpectedSourceVersion $ExpectedProfileHash
        Test-TaskspaceSerialCalibrationEvidence $SerialCalibrationRoot 3 $ExpectedTaskListHash $ExpectedSourceVersion $ExpectedProfileHash
        Test-TaskspaceParallelSmokeEvidence $ParallelEquivalencePath $ExpectedTaskListHash $ExpectedSourceVersion $ExpectedProfileHash
    )
    $failed = @($rows | Where-Object { [string]$_.status -ne "pass" })
    $result = [pscustomobject]@{
        schema_version = 1
        status = if ($failed.Count -eq 0) { "pass" } else { "fail" }
        full_e3_allowed = ($failed.Count -eq 0)
        speed_claim_allowed = ($failed.Count -eq 0)
        expected_identity = [pscustomobject]@{
            task_list_hash = $ExpectedTaskListHash
            source_version = $ExpectedSourceVersion
            profile_hash = $ExpectedProfileHash
        }
        gates = @($rows)
        first_failure = if ($failed.Count -gt 0) { $failed[0] } else { $null }
        generated_at = (Get-Date).ToString("o")
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $result | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    }
    $result
}
