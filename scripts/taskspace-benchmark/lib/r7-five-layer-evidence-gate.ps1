function Resolve-R7EvidencePath {
    param(
        [string]$RepoRoot,
        [string]$RelativePath,
        [string]$Label
    )

    $path = Join-Path $RepoRoot $RelativePath
    Assert-True (Test-Path -LiteralPath $path) "$Label is missing: $RelativePath"
    return (Resolve-Path -LiteralPath $path).Path
}

function Assert-R7EvidenceHash {
    param(
        [string]$Path,
        [string]$ExpectedSha256,
        [string]$Label
    )

    Assert-Equal (Get-Sha256 $Path) $ExpectedSha256 "$Label hash drifted"
}

function Get-R7ObservationSetSha256 {
    param(
        [object[]]$Runs,
        [string]$SnapshotRoot
    )

    $identityLines = foreach ($run in $Runs) {
        $fileName = "{0}-r{1}-{2}.json" -f $run.sample, $run.repeat, $run.arm
        $observationPath = Join-Path $SnapshotRoot (Join-Path "observations" $fileName)
        Assert-True (Test-Path -LiteralPath $observationPath -PathType Leaf) "Raw observation is missing: $observationPath"
        "{0}|{1}|{2}|{3}" -f $run.sample, $run.repeat, $run.arm, (Get-Sha256 $observationPath)
    }
    return Get-TextSha256 ($identityLines -join "`n")
}

function Get-R7SelectedObservationRows {
    param(
        [object[]]$Runs,
        [string]$SnapshotRoot
    )

    $selected = foreach ($run in $Runs) {
        $fileName = "{0}-r{1}-{2}.json" -f $run.sample, $run.repeat, $run.arm
        $observationPath = Join-Path $SnapshotRoot (Join-Path "observations" $fileName)
        $observation = Get-Content -Raw -Encoding UTF8 -LiteralPath $observationPath |
            ConvertFrom-Json -Depth 100
        $rows = @($observation.rows | Where-Object { [string]$_.side -eq [string]$run.run_side })
        Assert-Equal $rows.Count 1 "Raw observation must contain exactly one selected side"
        [pscustomobject]@{
            sample = [string]$run.sample
            repeat = [int]$run.repeat
            arm = [string]$run.arm
            row = $rows[0]
        }
    }
    return @($selected)
}

function Get-R7ToolSectionBytes {
    param([object]$Row)

    $tools = @($Row.section_cost.sections | Where-Object { [string]$_.kind -eq "tools" })
    Assert-Equal $tools.Count 1 "Observation must contain exactly one Tool section"
    return [int]$tools[0].bytes_per_request_mean
}

function Assert-R7Fla8RawEvidence {
    param(
        [string]$RepoRoot,
        [object]$Result
    )

    $matrixRoot = Resolve-R7EvidencePath $RepoRoot ([string]$Result.raw_evidence.snapshot_root) "FLA-8 evidence snapshot"
    $summaryPath = Join-Path $matrixRoot "summary.csv"
    $aggregatePath = Join-Path $matrixRoot "aggregate.csv"
    $tracePath = Join-Path $matrixRoot "trace-analysis.json"
    $manifestPath = Join-Path $matrixRoot "run-manifest.json"

    foreach ($path in @($summaryPath, $aggregatePath, $tracePath, $manifestPath)) {
        Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "FLA-8 raw artifact is missing: $path"
    }
    Assert-R7EvidenceHash $summaryPath ([string]$Result.raw_evidence.summary_csv_sha256) "FLA-8 summary.csv"
    Assert-R7EvidenceHash $aggregatePath ([string]$Result.raw_evidence.aggregate_csv_sha256) "FLA-8 aggregate.csv"
    Assert-R7EvidenceHash $tracePath ([string]$Result.raw_evidence.trace_analysis_sha256) "FLA-8 trace-analysis.json"
    Assert-R7EvidenceHash $manifestPath ([string]$Result.raw_evidence.run_manifest_sha256) "FLA-8 run-manifest.json"

    $summary = @(Import-Csv -LiteralPath $summaryPath)
    $aggregate = @(Import-Csv -LiteralPath $aggregatePath)
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath | ConvertFrom-Json -Depth 100
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50

    Assert-Equal $summary.Count ([int]$Result.run_count) "FLA-8 summary run count drifted"
    Assert-Equal @($trace.runs).Count ([int]$Result.run_count) "FLA-8 trace run count drifted"
    Assert-Equal ([int]$manifest.completed_run_count) ([int]$Result.run_count) "FLA-8 manifest is incomplete"
    Assert-Equal ([string]$manifest.repo_commit) ([string]$Result.subject_commit) "FLA-8 subject commit drifted"
    Assert-Equal @($summary | Where-Object observation_status -ne "complete").Count 0 "FLA-8 contains incomplete observations"
    Assert-Equal @($summary | Where-Object business_success -ne "True").Count 0 "FLA-8 business success is not 24/24"

    $taskspace = @($summary | Where-Object logical_mode -eq "taskspace")
    Assert-Equal $taskspace.Count 18 "FLA-8 TaskSpace run count drifted"
    Assert-Equal @($taskspace | Where-Object map_root_status -ne "closed").Count 0 "FLA-8 has an open TaskSpace root"
    Assert-Equal @($taskspace | Where-Object map_finish_status -ne "closed").Count 0 "FLA-8 has an open TaskSpace finish"
    Assert-Equal (($taskspace | Measure-Object -Property map_open_leaves -Sum).Sum) 0 "FLA-8 has open TaskSpace leaves"

    $requestComplex = @($summary | Where-Object {
        $_.sample -eq "subscription-billing-repair" -and $_.arm -eq "map-request"
    })
    Assert-Equal $requestComplex.Count 3 "FLA-8 map-request complex repeat count drifted"
    Assert-Equal @($requestComplex | Where-Object { [int]$_.multi_patch_attempts -lt 1 }).Count 0 "FLA-8 lost the 3/3 multi-Patch attempts"

    $requestRows = @($summary | Where-Object arm -eq "map-request")
    Assert-Equal (($requestRows | Measure-Object -Property taskspace_protocol_failure_requests -Sum).Sum) `
        ([int]$Result.trace_findings.map_request_taskspace_protocol_failure_requests) `
        "FLA-8 map-request protocol failure count drifted"
    Assert-Equal (($requestRows | Measure-Object -Property taskspace_state_failure_requests -Sum).Sum) `
        ([int]$Result.trace_findings.map_request_taskspace_state_failure_requests) `
        "FLA-8 map-request state failure count drifted"

    foreach ($armName in @("standard", "map-always", "map-append", "map-request")) {
        $armRows = @($summary | Where-Object arm -eq $armName)
        $armAggregate = @($aggregate | Where-Object {
            $_.scope -eq "all_samples_arm" -and $_.arm -eq $armName
        })
        Assert-Equal $armRows.Count 6 "FLA-8 $armName run count drifted"
        Assert-Equal $armAggregate.Count 1 "FLA-8 $armName aggregate row is missing"
        Assert-Equal ([int]$armAggregate[0].requests_total) ([int]$Result.arms.$armName.requests.total) "FLA-8 $armName request total drifted"
        Assert-Equal ([long]$armAggregate[0].input_tokens_total) ([long]$Result.arms.$armName.input_tokens.total) "FLA-8 $armName input total drifted"
    }
}

function Assert-R7Fla9RawEvidence {
    param(
        [string]$RepoRoot,
        [object]$Result,
        [object]$IntegratedConstraints
    )

    $matrixRoot = Resolve-R7EvidencePath $RepoRoot ([string]$Result.raw_evidence.snapshot_root) "FLA-9 evidence snapshot"
    $manifestPath = Join-Path $matrixRoot "run-manifest.json"
    Assert-True (Test-Path -LiteralPath $manifestPath -PathType Leaf) "FLA-9 raw run manifest is missing"
    Assert-R7EvidenceHash $manifestPath ([string]$Result.raw_evidence.run_manifest_sha256) "FLA-9 run-manifest.json"

    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    $runs = @($manifest.runs)
    Assert-Equal ([string]$manifest.status) "completed" "FLA-9 runner did not complete all planned invocations"
    Assert-Equal ([int]$manifest.completed_run_count) ([int]$manifest.planned_run_count) "FLA-9 invocation manifest is incomplete"
    Assert-Equal $runs.Count ([int]$Result.run_count) "FLA-9 raw run count drifted"
    Assert-Equal ([string]$manifest.repo_commit) ([string]$Result.subject_commit) "FLA-9 subject commit drifted"
    Assert-Equal (Get-R7ObservationSetSha256 $runs $matrixRoot) ([string]$Result.raw_evidence.observation_set_sha256) "FLA-9 observation set drifted"

    $selected = @(Get-R7SelectedObservationRows $runs $matrixRoot)
    $complete = @($selected | Where-Object { [string]$_.row.observation_status -eq "complete" })
    Assert-Equal $complete.Count 23 "FLA-9 must retain the one incomplete observation"
    Assert-Equal @($complete | Where-Object { -not [bool]$_.row.result.business_success }).Count 0 "FLA-9 completed observations contain a business failure"

    $taskspace = @($selected | Where-Object arm -ne "standard")
    Assert-Equal $taskspace.Count ([int]$Result.initialization.taskspace_runs) "FLA-9 TaskSpace run count drifted"
    Assert-Equal (($taskspace.row.actions | Measure-Object -Property initialization_carriers -Sum).Sum) ([int]$Result.initialization.attempts) "FLA-9 initialization attempts drifted"
    Assert-Equal (($taskspace.row.actions | Measure-Object -Property committed_initialization_carriers -Sum).Sum) ([int]$Result.initialization.commits) "FLA-9 initialization commits drifted"
    Assert-Equal (($taskspace.row.actions | Measure-Object -Property failed_initialization_carriers -Sum).Sum) ([int]$Result.initialization.failures) "FLA-9 initialization failures drifted"
    Assert-Equal @($taskspace | Where-Object {
        @($_.row.map.nodes | Where-Object kind -eq "task_root").Count -ne 1 -or
        @($_.row.map.nodes | Where-Object kind -eq "finish").Count -ne 1
    }).Count ([int]$Result.initialization.role_erasure_failures) "FLA-9 role partition evidence drifted"

    foreach ($armName in @("standard", "map-always", "map-append", "map-request")) {
        $armRows = @($selected | Where-Object arm -eq $armName)
        $armResult = $Result.arms.$armName
        Assert-Equal $armRows.Count ([int]$armResult.runs) "FLA-9 $armName run count drifted"
        Assert-Equal @($armRows | Where-Object { [bool]$_.row.result.business_success }).Count ([int]$armResult.successes) "FLA-9 $armName success count drifted"
        Assert-Equal (($armRows.row.actions | Measure-Object -Property provider_requests -Sum).Sum) ([int]$armResult.requests) "FLA-9 $armName request count drifted"
        if ($armName -ne "standard") {
            Assert-Equal (($armRows.row.actions | Measure-Object -Property control_protocol_failures -Sum).Sum) ([int]$armResult.protocol_failures) "FLA-9 $armName protocol failures drifted"
            Assert-Equal (($armRows.row.actions | Measure-Object -Property control_state_failures -Sum).Sum) ([int]$armResult.state_failures) "FLA-9 $armName state failures drifted"
        }
    }

    $standardToolBytes = @($selected | Where-Object arm -eq "standard" | ForEach-Object { Get-R7ToolSectionBytes $_.row } | Sort-Object -Unique)
    $taskspaceToolBytes = @($taskspace | ForEach-Object { Get-R7ToolSectionBytes $_.row } | Sort-Object -Unique)
    Assert-Equal $standardToolBytes.Count 1 "FLA-9 Standard Tool schema is not immutable"
    Assert-Equal $taskspaceToolBytes.Count 1 "FLA-9 TaskSpace Tool schema is not immutable"
    Assert-Equal $standardToolBytes[0] ([int]$Result.tool_schema.standard_bytes_per_request) "FLA-9 Standard Tool bytes drifted"
    Assert-Equal $taskspaceToolBytes[0] ([int]$Result.tool_schema.taskspace_candidate_bytes_per_request) "FLA-9 TaskSpace Tool bytes drifted"

    $incomplete = @($selected | Where-Object { [string]$_.row.observation_status -eq "incomplete" })
    Assert-Equal $incomplete.Count 1 "FLA-9 incomplete observation count drifted"
    Assert-Equal ([string]$incomplete[0].sample) "subscription-billing-repair" "FLA-9 blocker sample drifted"
    Assert-Equal ([int]$incomplete[0].repeat) 3 "FLA-9 blocker repeat drifted"
    Assert-Equal ([string]$incomplete[0].arm) "map-append" "FLA-9 blocker arm drifted"
    Assert-Equal ([string]$incomplete[0].row.result.agent_completion_status) "interrupted" "FLA-9 blocker completion status drifted"
    Assert-Equal ([string]$incomplete[0].row.map.root_task_status) "active" "FLA-9 blocker no longer retains the open Map evidence"

    $openRegressions = @($IntegratedConstraints.regression_invariants | Where-Object status -eq "open" | ForEach-Object id | Sort-Object)
    $declaredBlockers = @($Result.current_promotion_blockers | Sort-Object)
    Assert-Equal ($declaredBlockers -join ",") ($openRegressions -join ",") "FLA-9 result does not identify the complete current blocker set"
}
