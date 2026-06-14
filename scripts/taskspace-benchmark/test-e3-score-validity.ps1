param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\failure-taxonomy.ps1")
. (Join-Path $PSScriptRoot "lib\audit-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\aggregate-report.ps1")
. (Join-Path $PSScriptRoot "lib\timing.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-score-validity-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function New-Metrics {
    param(
        [string]$Mode,
        [string]$LogicalMode,
        [bool]$Success = $false,
        [bool]$ExecTimedOut = $false,
        [int]$PublicExit = 1,
        [string[]]$Failures = @(),
        [bool]$PretestFailure = $false
    )
    [pscustomobject]@{
        mode = $Mode
        logical_mode = $LogicalMode
        business_success = $Success
        exec_exit_code = 0
        exec_timed_out = $ExecTimedOut
        public_validation_exit_code = $PublicExit
        hidden_oracle_exit_code = 0
        wall_time_ms = 1000
        changed_paths = @("solution.py")
        validator_environment_failures = @($Failures)
        metrics_taints = @()
        pretest_failure = $PretestFailure
        tests_started_seen = (-not $PretestFailure)
        validation_lifecycle_stage = if ($PretestFailure) { "unknown" } else { "tests_completed" }
    }
}

function Assert-Outcome {
    param(
        [string]$Name,
        $Metrics,
        [string]$ExpectedOutcome,
        [bool]$ExpectedClean
    )
    $reasons = @(Get-TaskspaceEngineeringUncleanReasons $Metrics)
    $outcome = Get-TaskspaceAgentOutcome $Metrics $reasons
    Assert-True ($outcome -eq $ExpectedOutcome) "$Name expected outcome $ExpectedOutcome, got $outcome"
    Assert-True ((@(Get-TaskspaceEngineeringUncleanReasons $Metrics).Count -eq 0) -eq $ExpectedClean) "$Name clean-state mismatch"
}

Assert-Outcome "clean solved" (New-Metrics "left" "standard" -Success $true -PublicExit 0) "solved" $true
Assert-Outcome "clean wrong" (New-Metrics "left" "standard" -Success $false -PublicExit 2) "wrong" $true
Assert-Outcome "clean agent timeout" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 1) "agent_exec_timeout" $true
Assert-Outcome "validator timeout" (New-Metrics "left" "standard" -PublicExit 124 -Failures @("public_validation_timeout")) "engineering_unclean" $false
Assert-Outcome "docker failure" (New-Metrics "left" "standard" -PublicExit 1 -Failures @("docker_run_failure")) "engineering_unclean" $false
Assert-Outcome "timeout plus docker" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 1 -Failures @("docker_run_failure")) "engineering_unclean" $false

$auditMissingEvidence = [pscustomobject]@{
    evidence_gate_failures = @()
    e3_gate_failures = @("e3_human_review_not_completed")
}
$auditMissingReasons = @(Get-TaskspaceEngineeringUncleanReasons (New-Metrics "left" "standard" -Success $true -PublicExit 0) $auditMissingEvidence)
Assert-True ($auditMissingReasons -contains "e3_human_review_not_completed") "audit missing was not engineering unclean"

$auditPairDir = Join-Path $runDir "audit-pair"
New-Item -ItemType Directory -Force -Path $auditPairDir | Out-Null
$auditManifest = Write-TaskspaceAuditManifest $auditPairDir `
    ([pscustomobject]@{ repeat = 1; scenario = "score-validity-fixture"; human_review_required = $true }) `
    (New-Metrics "left" "standard" -Success $true -PublicExit 0) `
    (New-Metrics "right" "taskspace" -Success $true -PublicExit 0) `
    $auditMissingEvidence `
    ([pscustomobject]@{ invalid_pair = $false }) `
    ([pscustomobject]@{ completed = $false; failures = @("audit_review_missing"); source_path = "" })
Assert-True (-not [bool]$auditManifest.run_score_valid) "audit manifest did not mark missing review score-invalid"
Assert-True ([bool]$auditManifest.engineering_unclean) "audit manifest did not mark missing review engineering-unclean"
Assert-True (@($auditManifest.engineering_unclean_reasons) -contains "e3_human_review_not_completed") "audit manifest did not preserve missing review reason"

$aggregatePath = Join-Path $runDir "aggregate-report.md"
$invalidEvidence = [pscustomobject]@{
    reported_evidence_level = "E3-candidate"
    included_in_utility_aggregate = $false
    included_in_e3_aggregate = $false
    evidence_gate_failures = @()
    e3_gate_failures = @("public_validation_timeout")
    failure_taxonomy = @("engineering_unclean", "validator_slow_or_flaky")
    utility_direction = "score_disabled"
    human_review_completed = $false
    human_review_decision = ""
    human_review_disagreement = $false
    run_score_valid = $false
    engineering_unclean = $true
    engineering_unclean_reasons = @("public_validation_timeout")
    outcome_standard = "engineering_unclean"
    outcome_taskspace = "solved"
}
Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @([pscustomobject]@{
            repeat = 1
            pair_dir = $runDir
            pair_report = "pair-report.md"
            evidence_target = "E3"
            evidence = $invalidEvidence
        })
$aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "aggregate.json") | ConvertFrom-Json
$aggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath
Assert-True (-not [bool]$aggregate.score_valid) "aggregate did not set score_valid=false"
Assert-True (-not [bool]$aggregate.score_fields_enabled) "aggregate did not disable score fields"
Assert-True ([int]$aggregate.engineering_unclean_count -eq 1) "aggregate engineering_unclean_count mismatch"
Assert-True ([string]$aggregate.score_invalid_reason -eq "engineering_unclean") "aggregate score_invalid_reason mismatch"
Assert-True ($null -eq $aggregate.taskspace_better -and $null -eq $aggregate.standard_better) "aggregate did not null directional score fields"
Assert-True ($aggregateText -match "score_valid: False") "aggregate markdown did not render score_valid"
Assert-True ($aggregateText -notmatch "taskspace_better|standard_better|regressed|worse") "invalid aggregate rendered directional comparison wording"

$timingPairDir = Join-Path $runDir "pair-001"
New-Item -ItemType Directory -Force -Path $timingPairDir | Out-Null
$metricsBySide = @{
    left = New-Metrics "left" "standard" -Success $true -PublicExit 0
    right = New-Metrics "right" "taskspace" -Success $true -PublicExit 0
}
$now = Get-Date
$validationTimingBySide = @{
    left = [pscustomobject]@{ logical_mode = "standard"; validation_started_at = $now; validation_finished_at = $now.AddSeconds(1); validation_exit_code = 0; oracle_started_at = $now.AddSeconds(1); oracle_finished_at = $now.AddSeconds(2); oracle_exit_code = 0; engineering_unclean_reasons = @() }
    right = [pscustomobject]@{ logical_mode = "taskspace"; validation_started_at = $now; validation_finished_at = $now.AddSeconds(2); validation_exit_code = 0; oracle_started_at = $now.AddSeconds(2); oracle_finished_at = $now.AddSeconds(3); oracle_exit_code = 0; engineering_unclean_reasons = @() }
}
$timingPath = Write-TaskspacePairTiming $timingPairDir 1 $now $now.AddSeconds(5) ([pscustomobject]@{ Id = "timing-fixture" }) $null $metricsBySide $validationTimingBySide @()
$timing = Get-Content -Raw -Encoding UTF8 -LiteralPath $timingPath | ConvertFrom-Json
Assert-True ([int64]$timing.total_duration_ms -gt 0) "timing artifact did not record total duration"
Assert-True (@($timing.spans | Where-Object { [string]$_.phase -eq "public_validation" }).Count -eq 2) "timing artifact did not record both validation spans"

if ($failures.Count -gt 0) {
    Write-Host "E3 score-validity self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 score-validity self-test: PASS"
Write-Host "RunRoot: $runDir"
