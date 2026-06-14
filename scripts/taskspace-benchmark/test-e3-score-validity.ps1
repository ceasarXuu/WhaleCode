param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\failure-taxonomy.ps1")
. (Join-Path $PSScriptRoot "lib\audit-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\aggregate-report.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\timing.ps1")
. (Join-Path $PSScriptRoot "lib\suite-status.ps1")

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
        [bool]$PretestFailure = $false,
        [bool]$TestsCompleted = $true,
        [bool]$PublicValidationSkipped = $false,
        [string]$PublicValidationSkipReason = "",
        [string]$PreAgentProbeStatus = "",
        [string]$PreAgentProbeHash = ""
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
        public_validation_skipped = $PublicValidationSkipped
        public_validation_skip_reason = $PublicValidationSkipReason
        pre_agent_validator_probe_status = $PreAgentProbeStatus
        pre_agent_validator_probe_hash = $PreAgentProbeHash
        metrics_taints = @()
        pretest_failure = $PretestFailure
        tests_started_seen = (-not $PretestFailure)
        tests_completed_seen = (-not $PretestFailure -and $TestsCompleted)
        validation_lifecycle_stage = if ($PretestFailure) { "unknown" } elseif ($TestsCompleted) { "tests_completed" } else { "tests_started" }
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
Assert-Outcome "clean agent timeout with validation skip" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 0 -PublicValidationSkipped $true -PublicValidationSkipReason "agent_exec_timeout" -PreAgentProbeStatus "passed" -PreAgentProbeHash ("a" * 64)) "agent_exec_timeout" $true
Assert-Outcome "agent timeout skip missing probe" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 0 -PublicValidationSkipped $true -PublicValidationSkipReason "agent_exec_timeout") "engineering_unclean" $false
Assert-Outcome "agent timeout skip failed probe" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 0 -PublicValidationSkipped $true -PublicValidationSkipReason "agent_exec_timeout" -PreAgentProbeStatus "failed" -PreAgentProbeHash ("a" * 64)) "engineering_unclean" $false
Assert-Outcome "validator timeout" (New-Metrics "left" "standard" -PublicExit 124 -Failures @("public_validation_timeout")) "engineering_unclean" $false
Assert-Outcome "docker failure before tests" (New-Metrics "left" "standard" -PublicExit 1 -Failures @("docker_run_failure") -PretestFailure $true) "engineering_unclean" $false
Assert-Outcome "docker run test failure after tests" (New-Metrics "left" "standard" -PublicExit 1 -Failures @("docker_run_failure")) "wrong" $true
Assert-Outcome "timeout plus docker before tests" (New-Metrics "left" "taskspace" -ExecTimedOut $true -PublicExit 1 -Failures @("docker_run_failure") -PretestFailure $true) "engineering_unclean" $false

$auditMissingEvidence = [pscustomobject]@{
    evidence_gate_failures = @()
    e3_gate_failures = @("e3_human_review_not_completed")
}
$auditMissingReasons = @(Get-TaskspaceEngineeringUncleanReasons (New-Metrics "left" "standard" -Success $true -PublicExit 0) $auditMissingEvidence)
Assert-True ($auditMissingReasons.Count -eq 0) "pure audit missing should not be engineering unclean"
Assert-True (Test-TaskspaceAuditPending $auditMissingEvidence ([pscustomobject]@{ completed = $false; failures = @("audit_review_missing") })) "pure audit missing was not classified as audit pending"
$auditInvalidReasons = @(Get-TaskspaceEngineeringUncleanReasons (New-Metrics "left" "standard" -Success $true -PublicExit 0) $auditMissingEvidence ([pscustomobject]@{ completed = $false; failures = @("audit_hash_mismatch") }))
Assert-True ($auditInvalidReasons -contains "e3_audit_review_invalid") "invalid audit review was not engineering unclean"

$auditPairDir = Join-Path $runDir "audit-pair"
New-Item -ItemType Directory -Force -Path $auditPairDir | Out-Null
$auditManifest = Write-TaskspaceAuditManifest $auditPairDir `
    ([pscustomobject]@{ repeat = 1; scenario = "score-validity-fixture"; human_review_required = $true }) `
    (New-Metrics "left" "standard" -Success $true -PublicExit 0) `
    (New-Metrics "right" "taskspace" -Success $true -PublicExit 0) `
    $auditMissingEvidence `
    ([pscustomobject]@{ invalid_pair = $false }) `
    ([pscustomobject]@{ completed = $false; failures = @("audit_review_missing"); source_path = "" })
Assert-True (-not [bool]$auditManifest.run_score_ready) "audit manifest did not mark missing review score-pending"
Assert-True (-not [bool]$auditManifest.run_score_valid) "audit manifest did not mark missing review score-invalid"
Assert-True ([bool]$auditManifest.audit_required) "audit manifest did not mark missing review audit-required"
Assert-True (-not [bool]$auditManifest.engineering_unclean) "audit manifest incorrectly marked missing review engineering-unclean"
Assert-True (@($auditManifest.engineering_unclean_reasons).Count -eq 0) "audit manifest incorrectly preserved missing review as hard reason"

$auditPendingAggregatePath = Join-Path $runDir "audit-pending-aggregate-report.md"
$auditPendingEvidence = [pscustomobject]@{
    reported_evidence_level = "E3-candidate"
    included_in_utility_aggregate = $false
    included_in_e3_aggregate = $false
    evidence_gate_failures = @()
    e3_gate_failures = @("e3_human_review_not_completed")
    failure_taxonomy = @("audit_unclean")
    utility_direction = "score_disabled"
    human_review_completed = $false
    human_review_decision = ""
    human_review_disagreement = $false
    run_score_ready = $false
    run_score_valid = $false
    audit_required = $true
    engineering_unclean = $false
    engineering_unclean_reasons = @()
    outcome_standard = "solved"
    outcome_taskspace = "solved"
}
Write-TaskspaceAggregateReport -Path $auditPendingAggregatePath -Reports @([pscustomobject]@{
            repeat = 1
            pair_dir = $runDir
            pair_report = "pair-report.md"
            evidence_target = "E3"
            evidence = $auditPendingEvidence
        })
$auditPendingAggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "aggregate.json") | ConvertFrom-Json
$auditPendingText = Get-Content -Raw -Encoding UTF8 -LiteralPath $auditPendingAggregatePath
Assert-True ([string]$auditPendingAggregate.run_validity -eq "valid") "audit-pending aggregate should not be invalid_harness"
Assert-True (-not [bool]$auditPendingAggregate.score_ready -and -not [bool]$auditPendingAggregate.score_valid) "audit-pending aggregate did not disable score readiness"
Assert-True ([string]$auditPendingAggregate.score_block_reason -eq "audit_required") "audit-pending aggregate did not report audit_required block"
Assert-True ([int]$auditPendingAggregate.engineering_unclean_count -eq 0) "audit-pending aggregate incorrectly counted engineering unclean"
Assert-True ([int]$auditPendingAggregate.audit_required_count -eq 1) "audit-pending aggregate audit_required_count mismatch"
Assert-True ($auditPendingText -match "score fields disabled because E3 human review is pending") "audit-pending aggregate did not render pending-audit note"

$pairTimeoutDir = Join-Path $runDir "audit-timeout-side-scope"
New-Item -ItemType Directory -Force -Path $pairTimeoutDir | Out-Null
$pairTimeoutEvidence = [pscustomobject]@{
    evidence_gate_failures = @()
    e3_gate_failures = @("public_validation_timeout")
    included_in_utility_aggregate = $false
    included_in_e3_aggregate = $false
}
$pairTimeoutManifest = Write-TaskspaceAuditManifest $pairTimeoutDir `
    ([pscustomobject]@{ repeat = 1; scenario = "score-validity-fixture"; human_review_required = $false }) `
    (New-Metrics "left" "standard" -Success $true -PublicExit 0) `
    (New-Metrics "right" "taskspace" -Success $false -PublicExit 124 -Failures @("public_validation_timeout") -TestsCompleted $false) `
    $pairTimeoutEvidence `
    ([pscustomobject]@{ invalid_pair = $false }) `
    $null
Assert-True ([string]$pairTimeoutManifest.outcome_standard -eq "solved") "pair-level timeout contaminated clean standard side outcome"
Assert-True ([string]$pairTimeoutManifest.outcome_taskspace -eq "engineering_unclean") "taskspace timeout side was not engineering unclean"
Assert-True ([bool]$pairTimeoutManifest.engineering_unclean) "pair-level timeout did not invalidate pair scoring"

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

$pairReportPath = Join-Path $runDir "invalid-pair-report.md"
$pairPromptGuard = [pscustomobject]@{ invalid_prompt = $false; manual_review_required = $false; hard_hits = @(); context_hits = @() }
$pairVariableControl = [pscustomobject]@{ failures = @() }
$pairManifest = [pscustomobject]@{ Expected = [pscustomobject]@{}; Thresholds = [pscustomobject]@{} }
$standardMetrics = New-Metrics "left" "standard" -Success $false -PublicExit 124 -Failures @("public_validation_timeout")
$taskspaceMetrics = New-Metrics "right" "taskspace" -Success $true -PublicExit 0
Write-TaskspacePairReport $pairReportPath $pairManifest $pairPromptGuard $pairVariableControl $invalidEvidence $standardMetrics $taskspaceMetrics ([pscustomobject]@{ PairDir = $runDir }) $null
$pairReportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $pairReportPath
Assert-True ($pairReportText -match "outcome: score_disabled") "invalid pair report did not disable score outcome"
Assert-True ($pairReportText -notmatch "taskspace_better|taskspace_worse|regressed") "invalid pair report leaked directional wording"
$invalidE3Evidence = $invalidEvidence.PSObject.Copy()
$invalidE3Evidence.human_review_completed = $true
$invalidE3Evidence.human_review_decision = "include_taskspace_better"
$pairReportPath = Join-Path $runDir "invalid-e3-pair-report.md"
$pairManifest = [pscustomobject]@{ EvidenceTarget = "E3"; HumanReviewRequired = $true; Expected = [pscustomobject]@{}; Thresholds = [pscustomobject]@{}; SampleOrigin = [pscustomobject]@{ type = "external_benchmark" }; E3 = [pscustomobject]@{ claim_scope = "fixture" } }
Write-TaskspacePairReport $pairReportPath $pairManifest $pairPromptGuard $pairVariableControl $invalidE3Evidence $standardMetrics $taskspaceMetrics ([pscustomobject]@{ PairDir = $runDir }) $null
$pairReportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $pairReportPath
Assert-True ($pairReportText -match "human_review_decision: score_disabled") "invalid E3 pair report did not mask directional human review decision"
Assert-True ($pairReportText -notmatch "taskspace_better|taskspace_worse|standard_better|regressed") "invalid E3 pair report leaked directional review wording"

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
Assert-True ([string]$timing.runtime_optimization_status -eq "blocked" -and @($timing.runtime_optimization_blockers | Where-Object { [string]$_ -match "missing_wait_attribution:model_queue_wait_ms" }).Count -eq 1) "pair timing did not block speed claims when wait attribution is missing"
Assert-True (@($timing.runtime_optimization_blockers | Where-Object { [string]$_ -match "missing_wait_attribution:process_launch_wait_ms" }).Count -eq 1) "pair timing did not report missing process launch wait when no process timing was observed"
$metricWithTiming = Add-TaskspaceMetricTimingFields $metricsBySide.left $validationTimingBySide.left
Assert-True ([int64]$metricWithTiming.public_validation_duration_ms -gt 0) "metric timing did not record validation duration"
$processTimingPairDir = Join-Path $RunRoot ("process-timing-pair-" + (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
New-Item -ItemType Directory -Force -Path $processTimingPairDir | Out-Null
$processMetrics = @{
    left = New-Metrics "left" "standard" -Success $true -PublicExit 0
    right = New-Metrics "right" "taskspace" -Success $true -PublicExit 0
}
$processMetrics.left | Add-Member -NotePropertyName process_launch_wait_ms -NotePropertyValue 11 -Force
$processMetrics.right | Add-Member -NotePropertyName process_launch_wait_ms -NotePropertyValue 13 -Force
$processValidationTiming = @{
    left = [pscustomobject]@{ logical_mode = "standard"; validation_started_at = $now; validation_finished_at = $now.AddSeconds(1); validation_exit_code = 0; validation_process_launch_wait_ms = 17; oracle_started_at = $now.AddSeconds(1); oracle_finished_at = $now.AddSeconds(2); oracle_exit_code = 0; engineering_unclean_reasons = @() }
    right = [pscustomobject]@{ logical_mode = "taskspace"; validation_started_at = $now; validation_finished_at = $now.AddSeconds(1); validation_exit_code = 0; validation_process_launch_wait_ms = 19; oracle_started_at = $now.AddSeconds(1); oracle_finished_at = $now.AddSeconds(2); oracle_exit_code = 0; engineering_unclean_reasons = @() }
}
$processTimingPath = Write-TaskspacePairTiming $processTimingPairDir 1 $now $now.AddSeconds(3) ([pscustomobject]@{ Id = "process-timing-fixture" }) $null $processMetrics $processValidationTiming @()
$processTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $processTimingPath | ConvertFrom-Json
Assert-True ([int64]$processTiming.process_launch_wait_ms -eq 60) "pair timing did not aggregate agent and validation process launch wait"
Assert-True (@($processTiming.runtime_optimization_blockers | Where-Object { [string]$_ -match "missing_wait_attribution:process_launch_wait_ms" }).Count -eq 0) "pair timing still reported process launch wait missing after observing process timing"
$skipTimingBySide = @{
    left = [pscustomobject]@{ logical_mode = "standard"; validation_started_at = $now; validation_finished_at = $now; validation_exit_code = 0; validation_skipped = $true; validation_skip_reason = "agent_exec_timeout"; oracle_started_at = $now; oracle_finished_at = $now; oracle_exit_code = 0; engineering_unclean_reasons = @() }
}
$skipMetricsBySide = @{
    left = New-Metrics "left" "standard" -Success $false -ExecTimedOut $true -PublicExit 0 -PublicValidationSkipped $true -PublicValidationSkipReason "agent_exec_timeout" -PreAgentProbeStatus "passed" -PreAgentProbeHash ("b" * 64)
}
$skipTimingPairDir = Join-Path $RunRoot ("skip-timing-pair-" + (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
New-Item -ItemType Directory -Force -Path $skipTimingPairDir | Out-Null
$skipTimingPath = Write-TaskspacePairTiming $skipTimingPairDir 1 $now $now.AddSeconds(1) ([pscustomobject]@{ Id = "skip-timing-fixture" }) $null $skipMetricsBySide $skipTimingBySide @()
$skipTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $skipTimingPath | ConvertFrom-Json
Assert-True (@($skipTiming.spans | Where-Object { [string]$_.phase -eq "public_validation" }).Count -eq 0) "validation skip should not emit real public_validation span"
Assert-True (@($skipTiming.spans | Where-Object { [string]$_.phase -eq "public_validation_skipped" }).Count -eq 1) "validation skip did not emit public_validation_skipped span"
$skipMetricWithTiming = Add-TaskspaceMetricTimingFields $skipMetricsBySide.left $skipTimingBySide.left
Assert-True ([int64]$skipMetricWithTiming.public_validation_duration_ms -eq 0) "validation skip should record zero validation duration"
$dockerResultPath = Join-Path $runDir "docker-build-result.json"
[pscustomobject]@{
    phases = @(
        [pscustomobject]@{ phase = "build"; duration_ms = 1000; started_at = $now.ToString("o"); finished_at = $now.AddSeconds(1).ToString("o"); timestamp = $now.AddSeconds(1).ToString("o") },
        [pscustomobject]@{ phase = "run"; duration_ms = 2000; started_at = $now.AddSeconds(1).ToString("o"); finished_at = $now.AddSeconds(3).ToString("o"); timestamp = $now.AddSeconds(3).ToString("o") },
        [pscustomobject]@{ phase = "inspect"; duration_ms = 300; started_at = $now.AddSeconds(3).ToString("o"); finished_at = $now.AddMilliseconds(3300).ToString("o"); timestamp = $now.AddMilliseconds(3300).ToString("o") },
        [pscustomobject]@{ phase = "cleanup_container"; duration_ms = 400; started_at = $now.AddMilliseconds(3300).ToString("o"); finished_at = $now.AddMilliseconds(3700).ToString("o"); timestamp = $now.AddMilliseconds(3700).ToString("o") },
        [pscustomobject]@{ phase = "cleanup_image"; duration_ms = 500; started_at = $now.AddMilliseconds(3700).ToString("o"); finished_at = $now.AddMilliseconds(4200).ToString("o"); timestamp = $now.AddMilliseconds(4200).ToString("o") }
    )
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $dockerResultPath -Encoding UTF8
$metricsBySide.left | Add-Member -NotePropertyName docker_build_result_path -NotePropertyValue $dockerResultPath -Force
$metricWithDockerTiming = Add-TaskspaceMetricTimingFields $metricsBySide.left $validationTimingBySide.left
Assert-True ([int64]$metricWithDockerTiming.docker_build_duration_ms -eq 1000) "metric timing did not record docker build duration"
Assert-True ([int64]$metricWithDockerTiming.docker_run_duration_ms -eq 2000) "metric timing did not record docker run duration"
Assert-True ([int64]$metricWithDockerTiming.docker_cleanup_duration_ms -eq 900) "metric timing did not aggregate docker cleanup duration"
Assert-True ([int64]$metricWithDockerTiming.docker_observed_duration_ms -eq 4200) "metric timing did not record full docker observed duration"
$pretestTimeoutMetrics = New-Metrics "left" "standard" -PublicExit 124 -PretestFailure $true
$pretestTimeoutMetrics | Add-Member -NotePropertyName tests_started_seen -NotePropertyValue $false -Force
$pretestTimeoutMetrics = Add-TaskspaceMetricTimingFields $pretestTimeoutMetrics $validationTimingBySide.left
Assert-True ([string]$pretestTimeoutMetrics.validation_timeout_phase -eq "pretest") "timeout without tests_started was not classified as pretest"
$testTimeoutMetrics = New-Metrics "right" "taskspace" -PublicExit 124
$testTimeoutMetrics | Add-Member -NotePropertyName tests_started_seen -NotePropertyValue $true -Force
$testTimeoutMetrics = Add-TaskspaceMetricTimingFields $testTimeoutMetrics $validationTimingBySide.right
Assert-True ([string]$testTimeoutMetrics.validation_timeout_phase -eq "tests") "timeout after tests_started was not classified as tests"
$sampleTimingPath = Write-TaskspaceSampleTiming $runDir "score-validity-fixture"
$sampleTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $sampleTimingPath | ConvertFrom-Json
Assert-True ([int]$sampleTiming.pair_count -eq 1) "sample timing did not aggregate pair timing"
Assert-True ([int64]$sampleTiming.public_validation_duration_ms -gt 0) "sample timing did not aggregate validation duration"
$missingTimingPair = Join-Path $runDir "pair-002"
New-Item -ItemType Directory -Force -Path $missingTimingPair | Out-Null
$badTimingPair = Join-Path $runDir "pair-003"
New-Item -ItemType Directory -Force -Path $badTimingPair | Out-Null
"not-json" | Set-Content -LiteralPath (Join-Path $badTimingPair "pair-timing.json") -Encoding UTF8
$sampleTimingPath = Write-TaskspaceSampleTiming $runDir "score-validity-fixture"
$sampleTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $sampleTimingPath | ConvertFrom-Json
Assert-True ([int]$sampleTiming.missing_pair_timing_count -eq 1) "sample timing did not record missing pair timing"
Assert-True ([int]$sampleTiming.timing_parse_error_count -eq 1) "sample timing did not record malformed pair timing"
Assert-True ([string]$sampleTiming.runtime_optimization_status -eq "blocked" -and [string]$sampleTiming.timing_quality -eq "incomplete") "sample timing did not block runtime optimization when pair timing evidence was incomplete"
$suiteTimingPath = Write-TaskspaceSuiteTiming $runDir @([pscustomobject]@{ sample_id = "score-validity-fixture" })
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteTimingPath | ConvertFrom-Json
Assert-True ([int]$suiteTiming.timing_sample_count -eq 1) "suite timing did not aggregate sample timing"
Assert-True ([int64]$suiteTiming.total_pair_duration_ms -gt 0) "suite timing did not record total pair duration"
Assert-True ([string]$suiteTiming.runtime_optimization_status -eq "blocked" -and [string]$suiteTiming.timing_quality -eq "incomplete") "suite timing did not block when wait attribution was missing"
Assert-True (@($suiteTiming.runtime_optimization_blockers | Where-Object { [string]$_ -match "missing_wait_attribution:model_queue_wait_ms" }).Count -gt 0) "suite timing did not expose missing wait attribution blocker"
$suiteRoot = Join-Path $runDir "suite-fixture"
$suiteSamples = Join-Path $suiteRoot "samples"
New-Item -ItemType Directory -Force -Path (Join-Path $suiteSamples "sample-a") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $suiteSamples "sample-b") | Out-Null
"not-json" | Set-Content -LiteralPath (Join-Path $suiteSamples "sample-a\sample-timing.json") -Encoding UTF8
$suiteTimingPath = Write-TaskspaceSuiteTiming $suiteRoot @([pscustomobject]@{ sample_id = "sample-a" }, [pscustomobject]@{ sample_id = "sample-b" })
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteTimingPath | ConvertFrom-Json
Assert-True ([int]$suiteTiming.missing_sample_timing_count -eq 1) "suite timing did not record missing sample timing"
Assert-True ([int]$suiteTiming.timing_parse_error_count -eq 1) "suite timing did not record malformed sample timing"
Assert-True ([string]$suiteTiming.runtime_optimization_status -eq "blocked" -and [string]$suiteTiming.timing_quality -eq "incomplete") "suite timing did not block runtime optimization when sample timing evidence was incomplete"
$suiteRootFromStatus = Join-Path $runDir "suite-status-fixture"
New-Item -ItemType Directory -Force -Path $suiteRootFromStatus | Out-Null
$missingStatusRoot = Join-Path $suiteRootFromStatus "samples\sample-missing"
$suiteTimingPath = Write-TaskspaceSuiteTiming $suiteRootFromStatus @([pscustomobject]@{ sample_id = "sample-missing"; sample_root = $missingStatusRoot; run_validity = "invalid_harness" })
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteTimingPath | ConvertFrom-Json
Assert-True ([int]$suiteTiming.missing_sample_timing_count -eq 1) "suite timing did not use sample status to record missing timing"
$helperStatus = New-TaskspaceSuiteChildFailureStatus $null "sample-helper-missing" "task-dir" 3 "" (Join-Path $suiteRootFromStatus "samples\sample-helper-missing")
Assert-True ($helperStatus.PSObject.Properties.Name -contains "sample_root") "suite child failure status did not preserve sample_root"
$completeSuiteStatus = [pscustomobject]@{ sample_id = "sample-valid"; run_validity = "valid"; phase = "completed"; attempted_pairs = 5; completed_pairs = 5 }
$suiteScoreSummary = Get-TaskspaceSuiteScoreValiditySummary @($completeSuiteStatus, $helperStatus) 5
Assert-True ([int]$suiteScoreSummary.completed_child_processes -eq 1) "suite score summary did not count completed child"
Assert-True ([int]$suiteScoreSummary.score_valid_child_runs -eq 1) "suite score summary did not count valid child"
Assert-True ([int]$suiteScoreSummary.score_invalid_child_runs -eq 1) "suite score summary did not count invalid child"
Assert-True (-not [bool]$suiteScoreSummary.suite_score_valid -and [string]$suiteScoreSummary.first_score_invalid_run -eq "sample-helper-missing") "suite score summary did not identify first invalid sample"
$suiteTimingPath = Write-TaskspaceSuiteTiming $suiteRootFromStatus @($helperStatus)
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteTimingPath | ConvertFrom-Json
Assert-True ([int]$suiteTiming.missing_sample_timing_count -eq 1) "suite timing did not use helper-generated status to record missing timing"
$suiteAbortRoot = Join-Path $runDir "suite-abort-fixture"
$abortPairDir = Join-Path $suiteAbortRoot "samples\sample-a\run\pair-001"
New-Item -ItemType Directory -Force -Path $abortPairDir | Out-Null
[pscustomobject]@{ skipped_repeats = @(2, 3, 4, 5) } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $abortPairDir "pair-abort.json") -Encoding UTF8
Assert-True ((Get-TaskspaceSuiteRemainingSkippedPairs $suiteAbortRoot) -eq 4) "suite skipped pair helper did not count pair-abort skipped repeats"
$observedTimingDir = Join-Path $suiteAbortRoot "samples\sample-a"
[pscustomobject]@{ pair_count = 1; total_pair_duration_ms = 120000 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $observedTimingDir "sample-timing.json") -Encoding UTF8
$timeSaved = Get-TaskspaceSuiteExpectedTimeSaved $suiteAbortRoot @([pscustomobject]@{ sample_id = "sample-b"; skipped_reason = "suite_repeated_infra_signature" }) 5
Assert-True ([double]$timeSaved.expected_time_saved_minutes -eq 18.0 -and [int]$timeSaved.skipped_pair_equivalent_count -eq 9) "suite expected time saved did not use observed pair baseline"
$noBaselineSaved = Get-TaskspaceSuiteExpectedTimeSaved (Join-Path $runDir "suite-no-baseline") @([pscustomobject]@{ sample_id = "sample-b"; skipped_reason = "suite_repeated_infra_signature" }) 5
Assert-True ($null -eq $noBaselineSaved.expected_time_saved_minutes -and [string]$noBaselineSaved.expected_time_saved_basis -eq "no_serial_baseline") "suite expected time saved did not explain missing baseline"

if ($failures.Count -gt 0) {
    Write-Host "E3 score-validity self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 score-validity self-test: PASS"
Write-Host "RunRoot: $runDir"
