$ErrorActionPreference = "Stop"

function New-TaskspaceCalibrationGateRow {
    param([string]$Name, [string]$Status, [string]$Reason = "", [string]$Artifact = "", $Details = $null)
    [pscustomobject]@{
        name = $Name
        status = $Status
        reason = $Reason
        artifact = $Artifact
        details = if ($Details) { $Details } else { [pscustomobject]@{} }
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

function Test-TaskspaceRequiredStringValue {
    param($Object, [string]$Field, [string]$Expected, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    if ([string]$Object.$Field -ne $Expected) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_invalid:$Field" $Artifact ([pscustomobject]@{ field = $Field; expected = $Expected; actual = [string]$Object.$Field })
    }
    $null
}

function Test-TaskspaceRequiredBooleanValue {
    param($Object, [string]$Field, [bool]$Expected, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    if ([bool]$Object.$Field -ne $Expected) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_invalid:$Field" $Artifact ([pscustomobject]@{ field = $Field; expected = $Expected; actual = [bool]$Object.$Field })
    }
    $null
}

function Test-TaskspaceRequiredBooleanField {
    param($Object, [string]$Field, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    if ($Object.$Field -isnot [bool]) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_invalid:$Field" $Artifact ([pscustomobject]@{ field = $Field; actual = $Object.$Field })
    }
    $null
}

function Test-TaskspaceRequiredIntegerValue {
    param($Object, [string]$Field, [int]$Expected, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    if ([int]$Object.$Field -ne $Expected) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_invalid:$Field" $Artifact ([pscustomobject]@{ field = $Field; expected = $Expected; actual = [int]$Object.$Field })
    }
    $null
}

function Test-TaskspaceRequiredNonEmptyStringValue {
    param($Object, [string]$Field, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field) -or [string]::IsNullOrWhiteSpace([string]$Object.$Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    $null
}

function Convert-TaskspaceFullPathString {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return "" }
    try { return [System.IO.Path]::GetFullPath($Path) } catch { return $Path }
}

function Test-TaskspaceRequiredPathValue {
    param($Object, [string]$Field, [string]$Expected, [string]$Scope, [string]$Artifact)
    if (-not (Test-TaskspaceJsonField $Object $Field) -or [string]::IsNullOrWhiteSpace([string]$Object.$Field)) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_missing:$Field" $Artifact
    }
    $expectedPath = Convert-TaskspaceFullPathString $Expected
    $actualPath = Convert-TaskspaceFullPathString ([string]$Object.$Field)
    if ($actualPath -ine $expectedPath) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_field_invalid:$Field" $Artifact ([pscustomobject]@{ field = $Field; expected = $expectedPath; actual = $actualPath })
    }
    $null
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

function Test-TaskspaceCalibrationEngineeringCleanTiming {
    param($Timing, [string]$Scope, [string]$Artifact)
    if ($Timing -and $Timing.PSObject.Properties.Name -contains "engineering_unclean" -and [bool]$Timing.engineering_unclean) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_engineering_unclean" $Artifact ([pscustomobject]@{
                engineering_unclean = $true
                engineering_unclean_reasons = if ($Timing.PSObject.Properties.Name -contains "engineering_unclean_reasons") { @($Timing.engineering_unclean_reasons) } else { @("engineering_unclean") }
            })
    }
    if ($Timing -and $Timing.PSObject.Properties.Name -contains "engineering_unclean_reasons" -and @($Timing.engineering_unclean_reasons).Count -gt 0) {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_engineering_unclean_reasons_present" $Artifact ([pscustomobject]@{
                engineering_unclean = $true
                engineering_unclean_reasons = @($Timing.engineering_unclean_reasons)
            })
    }
    if ($Timing -and $Timing.PSObject.Properties.Name -contains "bottleneck_classification" -and [string]$Timing.bottleneck_classification -eq "engineering_unclean_slow") {
        return New-TaskspaceCalibrationGateRow $Scope "fail" "$($Scope)_engineering_unclean_slow" $Artifact ([pscustomobject]@{
                engineering_unclean = $true
                bottleneck_classification = [string]$Timing.bottleneck_classification
            })
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
    $runtimeReportJsonPath = if ($runtimeReportPath) { [System.IO.Path]::ChangeExtension($runtimeReportPath.FullName, ".json") } else { "" }
    if (-not $pairTimingPath -or -not $sampleTimingPath -or -not $runtimeReportPath -or -not (Test-Path -LiteralPath $runtimeReportJsonPath)) {
        return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_timing_artifact_missing" $Root
    }
    $pairTiming = Get-TaskspaceJsonFile $pairTimingPath.FullName
    $runtimeReport = Get-TaskspaceJsonFile $runtimeReportJsonPath
    $cleanCheck = Test-TaskspaceCalibrationEngineeringCleanTiming $pairTiming "one_pair_smoke" $pairTimingPath.FullName
    if ($cleanCheck) { return $cleanCheck }
    foreach ($field in @("agent_duration_ms", "public_validation_duration_ms", "bottleneck_classification", "runtime_optimization_status")) {
        if (-not (Test-TaskspaceJsonField $pairTiming $field)) {
            return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_timing_field_missing:$field" $pairTimingPath.FullName
        }
    }
    foreach ($runtimeCheck in @(
            (Test-TaskspaceRequiredIntegerValue $runtimeReport "schema_version" 1 "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredNonEmptyStringValue $runtimeReport "generated_at" "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredPathValue $runtimeReport "report_path" $runtimeReportPath.FullName "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredPathValue $runtimeReport "timing_path" $pairTimingPath.FullName "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredBooleanValue $runtimeReport "score_valid" $true "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredBooleanField $runtimeReport "speedup_evidence_valid" "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredStringValue $runtimeReport "timing_quality" "complete" "one_pair_smoke" $runtimeReportJsonPath),
            (Test-TaskspaceRequiredNonEmptyStringValue $runtimeReport "runtime_optimization_status" "one_pair_smoke" $runtimeReportJsonPath)
        )) {
        if ($runtimeCheck) { return $runtimeCheck }
    }
    if (-not (Test-TaskspaceJsonField $runtimeReport "speedup_decision") -or [string]$runtimeReport.speedup_decision -eq "speedup_blocked_invalid_run") {
        return New-TaskspaceCalibrationGateRow "one_pair_smoke" "fail" "one_pair_smoke_speedup_decision_blocked" $runtimeReportJsonPath ([pscustomobject]@{ speedup_decision = if (Test-TaskspaceJsonField $runtimeReport "speedup_decision") { [string]$runtimeReport.speedup_decision } else { "" } })
    }
    foreach ($identityCheck in @(
            (Test-TaskspaceCalibrationIdentityField $pairTiming "task_list_hash" $ExpectedTaskListHash "one_pair_smoke" $pairTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $pairTiming "source_version" $ExpectedSourceVersion "one_pair_smoke" $pairTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $pairTiming "profile_hash" $ExpectedProfileHash "one_pair_smoke" $pairTimingPath.FullName)
        )) {
        if ($identityCheck) { return $identityCheck }
    }
    New-TaskspaceCalibrationGateRow "one_pair_smoke" "pass" "" $pairTimingPath.FullName ([pscustomobject]@{
            schema_version = [int]$runtimeReport.schema_version
            generated_at = [string]$runtimeReport.generated_at
            timing_path = [string]$runtimeReport.timing_path
            report_path = [string]$runtimeReport.report_path
            score_valid = [bool]$runtimeReport.score_valid
            timing_quality = [string]$runtimeReport.timing_quality
            runtime_optimization_status = [string]$runtimeReport.runtime_optimization_status
            speedup_evidence_valid = [bool]$runtimeReport.speedup_evidence_valid
            speedup_decision = if (Test-TaskspaceJsonField $runtimeReport "speedup_decision") { [string]$runtimeReport.speedup_decision } else { "" }
            speedup_decision_reason = if (Test-TaskspaceJsonField $runtimeReport "speedup_decision_reason") { [string]$runtimeReport.speedup_decision_reason } else { "" }
            runtime_optimization_blockers = if (Test-TaskspaceJsonField $runtimeReport "runtime_optimization_blockers") { @($runtimeReport.runtime_optimization_blockers) } else { @() }
            wait_attribution_missing_fields = if (Test-TaskspaceJsonField $runtimeReport "wait_attribution_missing_fields") { @($runtimeReport.wait_attribution_missing_fields) } else { @() }
            wait_attribution_unavailable_fields = if (Test-TaskspaceJsonField $runtimeReport "wait_attribution_unavailable_fields") { $runtimeReport.wait_attribution_unavailable_fields } else { [pscustomobject]@{} }
            speed_claim_allowed = ([bool]$runtimeReport.speedup_evidence_valid -and [string]$runtimeReport.speedup_decision -notlike "speedup_blocked_*")
        })
}

function Test-TaskspaceSerialCalibrationEvidence {
    param([string]$Root, [int]$MinimumSamples = 3, [string]$ExpectedTaskListHash = "", [string]$ExpectedSourceVersion = "", [string]$ExpectedProfileHash = "")
    if ([string]::IsNullOrWhiteSpace($Root) -or -not (Test-Path -LiteralPath $Root)) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_root_missing" $Root
    }
    $suiteTimingPath = Get-ChildItem -LiteralPath $Root -Filter "suite-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $calibrationReportPath = Get-ChildItem -LiteralPath $Root -Filter "runtime-calibration-report.md" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $calibrationReportJsonPath = if ($calibrationReportPath) { [System.IO.Path]::ChangeExtension($calibrationReportPath.FullName, ".json") } else { "" }
    if (-not $suiteTimingPath -or -not $calibrationReportPath -or -not (Test-Path -LiteralPath $calibrationReportJsonPath)) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_artifact_missing" $Root
    }
    $suiteTiming = Get-TaskspaceJsonFile $suiteTimingPath.FullName
    $calibrationReport = Get-TaskspaceJsonFile $calibrationReportJsonPath
    $cleanCheck = Test-TaskspaceCalibrationEngineeringCleanTiming $suiteTiming "serial_calibration" $suiteTimingPath.FullName
    if ($cleanCheck) { return $cleanCheck }
    if (-not (Test-TaskspaceJsonField $suiteTiming "sample_count") -or [int]$suiteTiming.sample_count -lt $MinimumSamples) {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_sample_count_low" $suiteTimingPath.FullName
    }
    foreach ($field in @("timing_quality", "runtime_optimization_status", "bottleneck_classification", "wait_attribution_status")) {
        if (-not (Test-TaskspaceJsonField $suiteTiming $field)) {
            return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_field_missing:$field" $suiteTimingPath.FullName
        }
    }
    foreach ($suiteCheck in @(
            (Test-TaskspaceRequiredStringValue $suiteTiming "timing_quality" "complete" "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceRequiredNonEmptyStringValue $suiteTiming "runtime_optimization_status" "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceRequiredNonEmptyStringValue $suiteTiming "wait_attribution_status" "serial_calibration" $suiteTimingPath.FullName)
        )) {
        if ($suiteCheck) { return $suiteCheck }
    }
    foreach ($reportCheck in @(
            (Test-TaskspaceRequiredIntegerValue $calibrationReport "schema_version" 1 "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredNonEmptyStringValue $calibrationReport "generated_at" "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredPathValue $calibrationReport "report_path" $calibrationReportPath.FullName "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredPathValue $calibrationReport "timing_path" $suiteTimingPath.FullName "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredBooleanValue $calibrationReport "score_valid" $true "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredBooleanField $calibrationReport "speedup_evidence_valid" "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredStringValue $calibrationReport "timing_quality" "complete" "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredNonEmptyStringValue $calibrationReport "runtime_optimization_status" "serial_calibration" $calibrationReportJsonPath),
            (Test-TaskspaceRequiredNonEmptyStringValue $calibrationReport "wait_attribution_status" "serial_calibration" $calibrationReportJsonPath)
        )) {
        if ($reportCheck) { return $reportCheck }
    }
    if (-not (Test-TaskspaceJsonField $calibrationReport "speedup_decision") -or [string]$calibrationReport.speedup_decision -eq "speedup_blocked_invalid_run") {
        return New-TaskspaceCalibrationGateRow "serial_calibration" "fail" "serial_calibration_speedup_decision_blocked" $calibrationReportJsonPath ([pscustomobject]@{ speedup_decision = if (Test-TaskspaceJsonField $calibrationReport "speedup_decision") { [string]$calibrationReport.speedup_decision } else { "" } })
    }
    foreach ($identityCheck in @(
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "task_list_hash" $ExpectedTaskListHash "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "source_version" $ExpectedSourceVersion "serial_calibration" $suiteTimingPath.FullName),
            (Test-TaskspaceCalibrationIdentityField $suiteTiming "profile_hash" $ExpectedProfileHash "serial_calibration" $suiteTimingPath.FullName)
        )) {
        if ($identityCheck) { return $identityCheck }
    }
    New-TaskspaceCalibrationGateRow "serial_calibration" "pass" "" $suiteTimingPath.FullName ([pscustomobject]@{
            schema_version = [int]$calibrationReport.schema_version
            generated_at = [string]$calibrationReport.generated_at
            timing_path = [string]$calibrationReport.timing_path
            report_path = [string]$calibrationReport.report_path
            score_valid = [bool]$calibrationReport.score_valid
            timing_quality = [string]$suiteTiming.timing_quality
            runtime_optimization_status = [string]$suiteTiming.runtime_optimization_status
            wait_attribution_status = [string]$suiteTiming.wait_attribution_status
            speedup_evidence_valid = [bool]$calibrationReport.speedup_evidence_valid
            speedup_decision = [string]$calibrationReport.speedup_decision
            speedup_decision_reason = if (Test-TaskspaceJsonField $calibrationReport "speedup_decision_reason") { [string]$calibrationReport.speedup_decision_reason } else { "" }
            runtime_optimization_blockers = if (Test-TaskspaceJsonField $calibrationReport "runtime_optimization_blockers") { @($calibrationReport.runtime_optimization_blockers) } else { @() }
            wait_attribution_missing_fields = if (Test-TaskspaceJsonField $calibrationReport "wait_attribution_missing_fields") { @($calibrationReport.wait_attribution_missing_fields) } else { @() }
            wait_attribution_unavailable_fields = if (Test-TaskspaceJsonField $calibrationReport "wait_attribution_unavailable_fields") { $calibrationReport.wait_attribution_unavailable_fields } else { [pscustomobject]@{} }
            speed_claim_allowed = ([bool]$calibrationReport.speedup_evidence_valid -and [string]$calibrationReport.speedup_decision -notlike "speedup_blocked_*" -and [string]$suiteTiming.runtime_optimization_status -eq "ready" -and [string]$suiteTiming.wait_attribution_status -eq "complete" -and [string]$calibrationReport.runtime_optimization_status -eq "ready" -and [string]$calibrationReport.wait_attribution_status -eq "complete")
        })
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
    $speedRows = @($rows | Where-Object { $_.details -and ($_.details.PSObject.Properties.Name -contains "speed_claim_allowed") })
    $speedClaimAllowed = ($failed.Count -eq 0 -and $speedRows.Count -gt 0 -and @($speedRows | Where-Object { -not [bool]$_.details.speed_claim_allowed }).Count -eq 0)
    $result = [pscustomobject]@{
        schema_version = 1
        status = if ($failed.Count -eq 0) { "pass" } else { "fail" }
        full_e3_allowed = ($failed.Count -eq 0)
        speed_claim_allowed = $speedClaimAllowed
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
