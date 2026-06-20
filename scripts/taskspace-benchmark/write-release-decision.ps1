param(
    [Parameter(Mandatory = $true)][string]$RunDir,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\e3-identity.ps1")

function Read-ReleaseJson {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    try { Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json } catch { $null }
}

function Get-ReleaseBool {
    param($Object, [string]$Name, [bool]$DefaultValue = $false)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        if ($Object.$Name -is [bool]) { return [bool]$Object.$Name }
        if ($Object.$Name -is [string]) {
            $value = [string]$Object.$Name
            if ($value -eq "true") { return $true }
            if ($value -eq "false") { return $false }
            return $DefaultValue
        }
        return [bool]$Object.$Name
    }
    $DefaultValue
}

function Get-ReleaseInt {
    param($Object, [string]$Name, [int]$DefaultValue = 0)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        return [int]$Object.$Name
    }
    $DefaultValue
}

function Get-ReleaseString {
    param($Object, [string]$Name, [string]$DefaultValue = "")
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        return [string]$Object.$Name
    }
    $DefaultValue
}

function Read-ReleaseJsonl {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return @() }
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { [void]$rows.Add(($line | ConvertFrom-Json)) } catch {}
    }
    @($rows.ToArray())
}

function Add-ReleaseLine {
    param($Lines, [string]$Line)
    [void]$Lines.Add($Line)
}

function Test-ReleasePathUnderRoot {
    param([string]$Root, [string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $false }
    try {
        $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
        $pathFull = [System.IO.Path]::GetFullPath($Path)
        return $pathFull.StartsWith($rootFull + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
    } catch {
        return $false
    }
}

function Test-ReleaseEvidencePathAllowed {
    param([string]$RunRoot, [string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path -like "*://*") { return $false }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    if (Test-ReleasePathUnderRoot $RunRoot $Path) { return $true }
    try {
        $repoTarget = [System.IO.Path]::GetFullPath((Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) "target"))
        $pathFull = [System.IO.Path]::GetFullPath($Path)
        return $pathFull.StartsWith($repoTarget.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
    } catch {
        return $false
    }
}

function Get-ReleaseFileSha256 {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return "" }
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-ReleaseStableObjectHash {
    param($Value)
    $json = $Value | ConvertTo-Json -Depth 30 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-ReleaseStringSha256 {
    param([string]$Value)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Test-ReleaseReceiptHashChain {
    param([object[]]$Events)
    if ($Events.Count -eq 0) { return $false }
    $previousHash = ""
    foreach ($event in $Events) {
        $eventHash = Get-ReleaseString $event "event_hash"
        $declaredPrevious = Get-ReleaseString $event "previous_event_hash"
        if ([string]::IsNullOrWhiteSpace($eventHash) -or $declaredPrevious -ne $previousHash) { return $false }
        $copy = [ordered]@{}
        foreach ($property in $event.PSObject.Properties) {
            if ($property.Name -eq "event_hash") { continue }
            $copy[$property.Name] = $property.Value
        }
        if ((Get-ReleaseStableObjectHash $copy) -ne $eventHash) { return $false }
        $previousHash = $eventHash
    }
    $true
}

function Get-ReleaseArrayStrings {
    param($Object, [string]$Name)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        return @($Object.$Name | ForEach-Object { [string]$_ })
    }
    @()
}

function Read-ReleaseMetric {
    param([Parameter(Mandatory = $true)][string]$Path)
    $metric = Read-ReleaseJson $Path
    if (-not $metric) { return $null }
    [pscustomobject]@{
        path = $Path
        metric = $metric
        logical_mode = Get-ReleaseString $metric "logical_mode"
    }
}

function Get-ReleaseValidOutputRefCreatedEvents {
    param([Parameter(Mandatory = $true)][string]$Path)
    @(Read-ReleaseJsonl $Path | Where-Object {
            [string]$_.schema_version -eq "taskspace-output-ref-event-v1" -and
            [string]$_.kind -eq "output_ref.created" -and
            [string]$_.source -in @("observability_timeline", "observability_text_fallback", "taskspace.graph.final.json", "taskspace.graph.timeout.json", "graph-health.json") -and
            [string]$_.artifact_ref -match '^output-ref://sha256/[a-fA-F0-9]{64}$'
        })
}

$runRoot = (Resolve-Path -LiteralPath $RunDir).Path
if (-not $OutputPath) { $OutputPath = Join-Path $runRoot "release-decision.md" }
$jsonPath = [System.IO.Path]::ChangeExtension($OutputPath, ".json")
$repoRootForRelease = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$currentHeadForRelease = (& git -C $repoRootForRelease rev-parse HEAD 2>$null)

$costPath = Join-Path $runRoot "suite-cost-gate.json"
$aggregatePath = Join-Path $runRoot "aggregate.json"
$projectionPath = Join-Path $runRoot "context-projection-summary.json"
$mapPath = Join-Path $runRoot "suite-map-management-summary.json"
$routingPath = Join-Path $runRoot "suite-routing-summary.json"
$costDiagnosticsPath = Join-Path $runRoot "cost-diagnostics.json"
$runStatusPath = Join-Path $runRoot "run-status.json"
$eventsPath = Join-Path $runRoot "events.jsonl"
$suiteReceiptPath = Join-Path $runRoot "suite-receipt.jsonl"
$suiteRunnerAttestationPath = Join-Path $runRoot "suite-runner-attestation.json"
$outputRefEventsPath = Join-Path $runRoot "output-ref-events.jsonl"
$providerRequestEventsPath = Join-Path $runRoot "provider-request-events.jsonl"
$budgetEventsPath = Join-Path $runRoot "budget-events.jsonl"
$budgetQualityImpactEventsPath = Join-Path $runRoot "budget-quality-impact-events.jsonl"
$budgetQualityImpactSummaryPath = Join-Path $runRoot "budget_induced_quality_impact_summary.json"
$requestPhaseSummaryPath = Join-Path $runRoot "request-phase-summary.json"
$activeReplacementPath = Join-Path $runRoot "active-context-replacement-report.json"
$exactPayloadScanEventsPath = Join-Path $runRoot "exact-payload-scan-events.jsonl"
$stateCommitDisplacementPath = Join-Path $runRoot "state-commit-displacement.json"
$spawnNodeBudgetPath = Join-Path $runRoot "spawn-node-budget-summary.json"
$v005NonAgentGatesPath = Join-Path $runRoot "v005-non-agent-gates.json"
$v005CodeCompleteMarkerPath = Join-Path $runRoot "v005-code-complete.json"
$v005UserApprovalMarkerPath = Join-Path $runRoot "v005-user-approval.json"
$startGatePath = Join-Path $runRoot "start-gate\e3-start-gate.json"
$gateDecisionPath = Join-Path $runRoot "start-gate\gate-decision.json"
$requiredArtifacts = @(
    "token-summary.json",
    "request-summary.json",
    "taskspace-control-usage.json",
    "projection-events.jsonl",
    "output-ref-events.jsonl",
    "compaction-events.jsonl",
    "routing-decision.json",
    "suite-cost-gate.json",
    "suite-map-management-summary.json",
    "provider-request-events.jsonl",
    "budget-events.jsonl",
    "budget-quality-impact-events.jsonl",
    "budget_induced_quality_impact_summary.json",
    "request-phase-summary.json",
    "active-context-replacement-report.json",
    "exact-payload-scan-events.jsonl",
    "state-commit-displacement.json",
    "spawn-node-budget-summary.json",
    "v005-non-agent-gates.json",
    "v005-code-complete.json",
    "v005-user-approval.json",
    "start-gate\e3-start-gate.json",
    "start-gate\gate-decision.json",
    "suite-receipt.jsonl",
    "suite-runner-attestation.json"
)
$cost = Read-ReleaseJson $costPath
$aggregate = Read-ReleaseJson $aggregatePath
$projection = Read-ReleaseJson $projectionPath
$map = Read-ReleaseJson $mapPath
$routing = Read-ReleaseJson $routingPath
$costDiagnostics = Read-ReleaseJson $costDiagnosticsPath
$runStatus = Read-ReleaseJson $runStatusPath
$runEvents = @(Read-ReleaseJsonl $eventsPath)
$suiteReceiptEvents = @(Read-ReleaseJsonl $suiteReceiptPath)
$suiteRunnerAttestation = Read-ReleaseJson $suiteRunnerAttestationPath
$outputRefEvents = @(Read-ReleaseJsonl $outputRefEventsPath)
$providerRequestEvents = @(Read-ReleaseJsonl $providerRequestEventsPath)
$budgetEvents = @(Read-ReleaseJsonl $budgetEventsPath)
$budgetQualityImpactEvents = @(Read-ReleaseJsonl $budgetQualityImpactEventsPath)
$budgetQualityImpactSummary = Read-ReleaseJson $budgetQualityImpactSummaryPath
$requestPhaseSummary = Read-ReleaseJson $requestPhaseSummaryPath
$activeReplacement = Read-ReleaseJson $activeReplacementPath
$exactPayloadScanEvents = @(Read-ReleaseJsonl $exactPayloadScanEventsPath)
$stateCommitDisplacement = Read-ReleaseJson $stateCommitDisplacementPath
$spawnNodeBudget = Read-ReleaseJson $spawnNodeBudgetPath
$v005NonAgentGates = Read-ReleaseJson $v005NonAgentGatesPath
$v005CodeCompleteMarker = Read-ReleaseJson $v005CodeCompleteMarkerPath
$v005UserApprovalMarker = Read-ReleaseJson $v005UserApprovalMarkerPath
$startGate = Read-ReleaseJson $startGatePath
$gateDecision = Read-ReleaseJson $gateDecisionPath
$pairMetricPattern = '(?i)[\\/](pair-\d+)[\\/](left|right)[\\/]artifacts[\\/]metrics\.json$'
$metricRecords = @(Get-ChildItem -LiteralPath $runRoot -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq "metrics.json" -and $_.FullName -match $pairMetricPattern } |
    ForEach-Object { Read-ReleaseMetric $_.FullName } |
    Where-Object { $_ })
$standardMetrics = @($metricRecords | Where-Object { [string]$_.logical_mode -eq "standard" })
$taskspaceMetrics = @($metricRecords | Where-Object { [string]$_.logical_mode -eq "taskspace" })
$maxLargeReplay = 0
$runtimeOutputRefs = 0
foreach ($metric in $taskspaceMetrics) {
    $maxLargeReplay = [Math]::Max($maxLargeReplay, (Get-ReleaseInt $metric.metric "large_output_replay_count" 0))
    $runtimeOutputRefs += Get-ReleaseInt $metric.metric "runtime_output_ref_created_count" 0
}
$validOutputRefCreatedEvents = @(Get-ReleaseValidOutputRefCreatedEvents $outputRefEventsPath)
$runInitializedEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "run_initialized" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.scenario_id) -and
        [int]$_.repeats -gt 0 -and
        [string]$_.evidence_target -eq "E3"
    })
$routingDecisionEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "routing_decision_completed" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.mode) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.status) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.path) -and
        (Test-ReleasePathUnderRoot $runRoot ([string]$_.path)) -and
        (Test-Path -LiteralPath ([string]$_.path) -PathType Leaf)
    })
$latestRoutingDecisionEvent = @($runEvents | Where-Object { [string]$_.event -eq "routing_decision_completed" } | Select-Object -Last 1)
$routingDecisionPathExpected = Join-Path $runRoot "routing-decision.json"
$routingDecisionPass = ($latestRoutingDecisionEvent.Count -eq 1 `
    -and [int]$latestRoutingDecisionEvent[0].schema_version -eq 1 `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].timestamp) `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].mode) `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].status) `
    -and ([System.IO.Path]::GetFullPath([string]$latestRoutingDecisionEvent[0].path).Equals([System.IO.Path]::GetFullPath($routingDecisionPathExpected), [System.StringComparison]::OrdinalIgnoreCase)) `
    -and (Test-Path -LiteralPath $routingDecisionPathExpected -PathType Leaf))
$expectedFormalSampleSetId = "terminal-bench_E3-P0_3_5"
$expectedFormalSampleNames = @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log")
$formalPairCount = $expectedFormalSampleNames.Count * 5
$pairCompletedEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "pair_completed" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        [int]$_.repeat -gt 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.sample_id) -and
        [int]$_.sample_repeat_index -gt 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.standard_run_id) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.taskspace_run_id) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.pair_report) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.reported_evidence_level)
    })
$completedPairs = Get-ReleaseInt $runStatus "completed_pairs" 0
$validPairEvidence = New-Object System.Collections.Generic.List[object]
$pairEvidenceKeys = New-Object 'System.Collections.Generic.HashSet[string]'
$pairRepeatKeys = New-Object 'System.Collections.Generic.HashSet[string]'
$pairSampleRepeatKeys = New-Object 'System.Collections.Generic.HashSet[string]'
$formalPairCountsBySample = @{}
foreach ($name in $expectedFormalSampleNames) { $formalPairCountsBySample[$name] = 0 }
foreach ($event in $pairCompletedEvents) {
    $pairReport = [string]$event.pair_report
    if (-not (Test-ReleasePathUnderRoot $runRoot $pairReport)) { continue }
    if ([string]$event.reported_evidence_level -ne "E3") { continue }
    $pairDir = Split-Path -Parent $pairReport
    if (-not (Test-Path -LiteralPath $pairReport -PathType Leaf)) { continue }
    $pairLeaf = Split-Path -Leaf $pairDir
    if ($pairLeaf -notmatch '^pair-\d{3}$') { continue }
    $expectedPairLeaf = "pair-{0:D3}" -f [int]$event.repeat
    if ($pairLeaf -ne $expectedPairLeaf) { continue }
    $pairParent = Split-Path -Parent $pairDir
    if (-not ([System.IO.Path]::GetFullPath($pairParent).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar).Equals($runRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar), [System.StringComparison]::OrdinalIgnoreCase))) { continue }
    $pairKey = [System.IO.Path]::GetFullPath($pairDir).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar).ToLowerInvariant()
    $repeatKey = [string][int]$event.repeat
    if (-not $pairEvidenceKeys.Add($pairKey)) { continue }
    if (-not $pairRepeatKeys.Add($repeatKey)) { continue }
    $sampleId = [string]$event.sample_id
    if (-not ($expectedFormalSampleNames -contains $sampleId)) { continue }
    $sampleRepeatIndex = [int]$event.sample_repeat_index
    if ($sampleRepeatIndex -lt 1 -or $sampleRepeatIndex -gt 5) { continue }
    $sampleRepeatKey = "$sampleId#$sampleRepeatIndex"
    if (-not $pairSampleRepeatKeys.Add($sampleRepeatKey)) { continue }
    $formalPairCountsBySample[$sampleId] = [int]$formalPairCountsBySample[$sampleId] + 1
    $leftMetricPath = Join-Path $pairDir "left\artifacts\metrics.json"
    $rightMetricPath = Join-Path $pairDir "right\artifacts\metrics.json"
    if (-not (Test-Path -LiteralPath $leftMetricPath) -or -not (Test-Path -LiteralPath $rightMetricPath)) { continue }
    $leftRecord = Read-ReleaseMetric $leftMetricPath
    $rightRecord = Read-ReleaseMetric $rightMetricPath
    if (-not $leftRecord -or -not $rightRecord) { continue }
    $modes = @([string]$leftRecord.logical_mode, [string]$rightRecord.logical_mode)
    if (-not ($modes -contains "standard") -or -not ($modes -contains "taskspace")) { continue }
    $taskspaceRecord = if ([string]$leftRecord.logical_mode -eq "taskspace") { $leftRecord } else { $rightRecord }
    $taskspaceOutputRefEventsPath = Join-Path (Split-Path -Parent $taskspaceRecord.path) "output-ref-events.jsonl"
    $taskspaceOutputRefEvents = @(Get-ReleaseValidOutputRefCreatedEvents $taskspaceOutputRefEventsPath)
    $taskspaceRuntimeOutputRefs = Get-ReleaseInt $taskspaceRecord.metric "runtime_output_ref_created_count" 0
    [void]$validPairEvidence.Add([pscustomobject]@{
            pair_dir = $pairDir
            pair_report = $pairReport
            repeat = [int]$event.repeat
            sample_id = $sampleId
            sample_repeat_index = $sampleRepeatIndex
            standard_run_id = [string]$event.standard_run_id
            taskspace_run_id = [string]$event.taskspace_run_id
            taskspace_output_ref_events_path = $taskspaceOutputRefEventsPath
            taskspace_runtime_output_ref_created_count = [int]$taskspaceRuntimeOutputRefs
            taskspace_output_ref_created_event_count = [int]$taskspaceOutputRefEvents.Count
            output_ref_correlated = ($taskspaceRuntimeOutputRefs -gt 0 -and $taskspaceOutputRefEvents.Count -gt 0)
        })
}
$validOutputRefPairEvidence = @($validPairEvidence | Where-Object { [bool]$_.output_ref_correlated })
$formalPairSampleLedgerPass = ($pairSampleRepeatKeys.Count -eq $formalPairCount)
foreach ($name in $expectedFormalSampleNames) {
    if (-not $formalPairCountsBySample.ContainsKey($name) -or [int]$formalPairCountsBySample[$name] -ne 5) {
        $formalPairSampleLedgerPass = $false
    }
}
$runProvenancePass = ($runStatus `
    -and (Get-ReleaseInt $runStatus "schema_version" 0) -eq 1 `
    -and (Get-ReleaseString $runStatus "run_validity") -eq "valid" `
    -and (Get-ReleaseString $runStatus "evidence_target") -eq "E3" `
    -and (Get-ReleaseBool $runStatus "diagnostic_comparison_enabled") `
    -and (Get-ReleaseBool $runStatus "final_aggregate_ready") `
    -and $completedPairs -gt 0 `
    -and $runInitializedEvents.Count -gt 0 `
    -and $routingDecisionEvents.Count -gt 0 `
    -and $routingDecisionPass `
    -and $pairCompletedEvents.Count -eq $completedPairs `
    -and $validPairEvidence.Count -eq $completedPairs `
    -and $pairEvidenceKeys.Count -eq $completedPairs `
    -and $pairRepeatKeys.Count -eq $completedPairs `
    -and $formalPairSampleLedgerPass)

$runSampleSetId = Get-ReleaseString $runStatus "sample_set_id"
$runBenchmarkFamily = Get-ReleaseString $runStatus "benchmark_family"
$runRunnerEntrypoint = Get-ReleaseString $runStatus "runner_entrypoint"
$runRunnerProfileHash = Get-ReleaseString $runStatus "runner_profile_hash"
$runSourceVersion = Get-ReleaseString $runStatus "source_version"
$runTaskListHash = Get-ReleaseString $runStatus "task_list_hash"
$runRepeatsPerSample = Get-ReleaseInt $runStatus "repeats_per_sample" 0
$runArtifactOrigin = Get-ReleaseString $runStatus "artifact_origin"
$runRunnerScriptSha256 = Get-ReleaseString $runStatus "runner_script_sha256"
$runChildRunnerSha256 = Get-ReleaseString $runStatus "child_runner_sha256"
$runTaskListSha256 = Get-ReleaseString $runStatus "task_list_sha256"
$runSuiteManifestPath = Get-ReleaseString $runStatus "suite_manifest_path"
$runSuiteManifestSha256 = Get-ReleaseString $runStatus "suite_manifest_sha256"
$runSuiteReceiptPath = Get-ReleaseString $runStatus "suite_receipt_path"
$runSuiteReceiptSha256 = Get-ReleaseString $runStatus "suite_receipt_sha256"
$runSuiteRunnerAttestationPath = Get-ReleaseString $runStatus "suite_runner_attestation_path"
$runSuiteRunnerAttestationSha256 = Get-ReleaseString $runStatus "suite_runner_attestation_sha256"
$runApprovalMarkerSha256 = Get-ReleaseString $runStatus "approval_marker_sha256"
$runCodeCompleteMarkerSha256 = Get-ReleaseString $runStatus "code_complete_marker_sha256"
$runSampleNames = @(Get-ReleaseArrayStrings $runStatus "sample_names" | Sort-Object)
$expectedSortedSampleNames = @($expectedFormalSampleNames | Sort-Object)
$sampleNamesMatch = ($runSampleNames.Count -eq $expectedSortedSampleNames.Count)
if ($sampleNamesMatch) {
    for ($i = 0; $i -lt $runSampleNames.Count; $i++) {
        if ($runSampleNames[$i] -ne $expectedSortedSampleNames[$i]) {
            $sampleNamesMatch = $false
            break
        }
    }
}
$suiteManifest = Read-ReleaseJson $runSuiteManifestPath
$manifestTaskListPath = Get-ReleaseString $suiteManifest "task_list_path"
$taskListIdentitySource = "missing"
$derivedSampleSetId = ""
$derivedSampleNames = @()
$releaseTaskListDerivationPass = $false
if (-not [string]::IsNullOrWhiteSpace($manifestTaskListPath) -and (Test-Path -LiteralPath $manifestTaskListPath -PathType Leaf)) {
    $actualTaskListSha256 = Get-ReleaseFileSha256 $manifestTaskListPath
    try {
        $taskListDerivation = Get-TaskspaceE3SampleSetDerivation -Benchmark $runBenchmarkFamily -TaskListPath $manifestTaskListPath -Repeats $runRepeatsPerSample
        $derivedSampleSetId = Get-ReleaseString $taskListDerivation "sample_set_id"
        $derivedSampleNames = @(Get-ReleaseArrayStrings $taskListDerivation "sample_names" | Sort-Object)
        $taskListIdentitySource = "derived_from_task_list"
        $releaseTaskListDerivationPass = ((Get-ReleaseBool $taskListDerivation "formal_p0") `
            -and $derivedSampleSetId -eq $expectedFormalSampleSetId `
            -and $actualTaskListSha256 -eq $runTaskListSha256 `
            -and $actualTaskListSha256 -eq (Get-ReleaseString $suiteManifest "task_list_sha256"))
    } catch {
        $taskListIdentitySource = "derivation_failed"
    }
}
$derivedSampleNamesMatch = ($derivedSampleNames.Count -eq $expectedSortedSampleNames.Count)
if ($derivedSampleNamesMatch) {
    for ($i = 0; $i -lt $derivedSampleNames.Count; $i++) {
        if ($derivedSampleNames[$i] -ne $expectedSortedSampleNames[$i]) {
            $derivedSampleNamesMatch = $false
            break
        }
    }
}
$gateDecisionPass = ($gateDecision `
    -and (Get-ReleaseBool $gateDecision "full_e3_allowed") `
    -and (Get-ReleaseBool $gateDecision "v005_markers_passed") `
    -and (Get-ReleaseBool $gateDecision "calibration_gate_passed") `
    -and (Get-ReleaseString $gateDecision "task_list_hash") -eq $runTaskListHash `
    -and (Get-ReleaseString $gateDecision "source_version") -eq $runSourceVersion `
    -and (Get-ReleaseString $gateDecision "profile_hash") -eq $runRunnerProfileHash)
$startGatePass = ($startGate `
    -and [string]$startGate.status -eq "pass" `
    -and $startGate.PSObject.Properties.Name -contains "gate_decision" `
    -and (Get-ReleaseBool $startGate.gate_decision "full_e3_allowed") `
    -and (Get-ReleaseBool $startGate.gate_decision "v005_markers_passed") `
    -and (Get-ReleaseBool $startGate.gate_decision "calibration_gate_passed"))
$codeCompleteMarkerPass = ($v005CodeCompleteMarker `
    -and [string]$v005CodeCompleteMarker.status -eq "pass" `
    -and (Get-ReleaseInt $v005CodeCompleteMarker "schema_version" 0) -eq 1 `
    -and (Get-ReleaseBool $v005CodeCompleteMarker "code_complete") `
    -and (Get-ReleaseString $v005CodeCompleteMarker "task_list_hash") -eq $runTaskListHash `
    -and (Get-ReleaseString $v005CodeCompleteMarker "source_version") -eq $runSourceVersion `
    -and (Get-ReleaseString $v005CodeCompleteMarker "profile_hash") -eq $runRunnerProfileHash `
    -and (Get-ReleaseString $v005CodeCompleteMarker "sample_set_id") -eq $expectedFormalSampleSetId `
    -and -not [string]::IsNullOrWhiteSpace((Get-ReleaseString $v005CodeCompleteMarker "generated_at")) `
    -and -not [string]::IsNullOrWhiteSpace((Get-ReleaseString $v005CodeCompleteMarker "git_commit")) `
    -and -not [string]::IsNullOrWhiteSpace([string]$currentHeadForRelease) `
    -and (Get-ReleaseString $v005CodeCompleteMarker "git_commit") -eq [string]$currentHeadForRelease `
    -and @($v005CodeCompleteMarker.test_outputs).Count -gt 0 `
    -and @($v005CodeCompleteMarker.unfinished_p0_items).Count -eq 0)
$userApprovalMarkerPass = ($v005UserApprovalMarker `
    -and [string]$v005UserApprovalMarker.status -eq "pass" `
    -and (Get-ReleaseString $v005UserApprovalMarker "approved_command_category") -eq "full_e3" `
    -and (Get-ReleaseString $v005UserApprovalMarker "approved_sample_set_id") -eq $expectedFormalSampleSetId `
    -and (Get-ReleaseString $v005UserApprovalMarker "task_list_hash") -eq $runTaskListHash `
    -and (Get-ReleaseString $v005UserApprovalMarker "source_version") -eq $runSourceVersion `
    -and (Get-ReleaseString $v005UserApprovalMarker "profile_hash") -eq $runRunnerProfileHash `
    -and -not [string]::IsNullOrWhiteSpace((Get-ReleaseString $v005UserApprovalMarker "approval_source")) `
    -and -not [string]::IsNullOrWhiteSpace((Get-ReleaseString $v005UserApprovalMarker "approval_timestamp")))
if ($userApprovalMarkerPass) {
    try {
        $approvalTimestamp = [datetimeoffset]::Parse((Get-ReleaseString $v005UserApprovalMarker "approval_timestamp"))
        if ($approvalTimestamp -lt (Get-Date).AddHours(-24)) { $userApprovalMarkerPass = $false }
    } catch {
        $userApprovalMarkerPass = $false
    }
}
$actualApprovalMarkerSha256 = Get-ReleaseFileSha256 $v005UserApprovalMarkerPath
$actualCodeCompleteMarkerSha256 = Get-ReleaseFileSha256 $v005CodeCompleteMarkerPath
$actualSuiteManifestSha256 = Get-ReleaseFileSha256 $runSuiteManifestPath
$suiteManifestPass = ($suiteManifest `
    -and $runSuiteManifestSha256 -match '^[a-fA-F0-9]{64}$' `
    -and $runSuiteManifestSha256 -eq $actualSuiteManifestSha256 `
    -and (Get-ReleaseString $suiteManifest "artifact_origin") -eq "real_suite" `
    -and (Get-ReleaseString $suiteManifest "sample_set_id") -eq $runSampleSetId `
    -and (Get-ReleaseString $suiteManifest "benchmark") -eq $runBenchmarkFamily `
    -and (Get-ReleaseString $suiteManifest "runner_entrypoint") -eq $runRunnerEntrypoint `
    -and (Get-ReleaseString $suiteManifest "runner_script_sha256") -eq $runRunnerScriptSha256 `
    -and (Get-ReleaseString $suiteManifest "child_runner_sha256") -eq $runChildRunnerSha256 `
    -and (Get-ReleaseString $suiteManifest "task_list_sha256") -eq $runTaskListSha256 `
    -and (Get-ReleaseString $suiteManifest "task_list_hash") -eq $runTaskListHash `
    -and (Get-ReleaseString $suiteManifest "source_version") -eq $runSourceVersion `
    -and (Get-ReleaseString $suiteManifest "profile_hash") -eq $runRunnerProfileHash `
    -and (Get-ReleaseInt $suiteManifest "repeats" 0) -eq $runRepeatsPerSample)
$suiteProvenancePass = ($runArtifactOrigin -eq "real_suite" `
    -and $runRunnerScriptSha256 -match '^[a-fA-F0-9]{64}$' `
    -and $runChildRunnerSha256 -match '^[a-fA-F0-9]{64}$' `
    -and $runTaskListSha256 -match '^[a-fA-F0-9]{64}$' `
    -and $suiteManifestPass `
    -and $runApprovalMarkerSha256 -eq $actualApprovalMarkerSha256 `
    -and $runCodeCompleteMarkerSha256 -eq $actualCodeCompleteMarkerSha256)
$actualSuiteReceiptSha256 = Get-ReleaseFileSha256 $suiteReceiptPath
$receiptRunInitialized = @($suiteReceiptEvents | Where-Object { [string]$_.event -eq "run_initialized" })
$receiptSampleScheduled = @($suiteReceiptEvents | Where-Object { [string]$_.event -eq "sample_scheduled" })
$receiptSampleCompleted = @($suiteReceiptEvents | Where-Object { [string]$_.event -eq "sample_completed" })
$receiptFinalized = @($suiteReceiptEvents | Where-Object { [string]$_.event -eq "suite_finalized" })
$receiptRunnerAttestationGenerated = @($suiteReceiptEvents | Where-Object { [string]$_.event -eq "runner_attestation_generated" })
$suiteReceiptHashChainPass = Test-ReleaseReceiptHashChain $suiteReceiptEvents
$receiptScheduledSamples = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($event in @($receiptSampleScheduled)) {
    $sampleId = Get-ReleaseString $event "sample_id"
    if (-not [string]::IsNullOrWhiteSpace($sampleId)) { [void]$receiptScheduledSamples.Add($sampleId) }
}
$receiptCompletedPairsBySample = @{}
foreach ($name in $expectedFormalSampleNames) { $receiptCompletedPairsBySample[$name] = 0 }
foreach ($event in @($receiptSampleCompleted)) {
    $sampleId = Get-ReleaseString $event "sample_id"
    if ($receiptCompletedPairsBySample.ContainsKey($sampleId)) {
        $receiptCompletedPairsBySample[$sampleId] = [int]$receiptCompletedPairsBySample[$sampleId] + (Get-ReleaseInt $event "completed_pairs" 0)
    }
}
$suiteReceiptFormalSampleCoveragePass = $true
foreach ($name in $expectedFormalSampleNames) {
    if (-not $receiptScheduledSamples.Contains($name) -or [int]$receiptCompletedPairsBySample[$name] -lt 5) {
        $suiteReceiptFormalSampleCoveragePass = $false
        break
    }
}
$suiteReceiptPass = ($suiteReceiptEvents.Count -gt 0 `
    -and $suiteReceiptHashChainPass `
    -and $receiptRunInitialized.Count -eq 1 `
    -and $receiptFinalized.Count -eq 1 `
    -and $suiteReceiptFormalSampleCoveragePass `
    -and -not [string]::IsNullOrWhiteSpace($runSuiteReceiptSha256) `
    -and $runSuiteReceiptSha256 -eq $actualSuiteReceiptSha256 `
    -and (Test-ReleasePathUnderRoot $runRoot $runSuiteReceiptPath) `
    -and (Get-ReleaseString $receiptRunInitialized[0] "sample_set_id") -eq $runSampleSetId `
    -and (Get-ReleaseString $receiptRunInitialized[0] "runner_script_sha256") -eq $runRunnerScriptSha256 `
    -and (Get-ReleaseString $receiptRunInitialized[0] "child_runner_sha256") -eq $runChildRunnerSha256 `
    -and (Get-ReleaseString $receiptRunInitialized[0] "task_list_sha256") -eq $runTaskListSha256 `
    -and (Get-ReleaseString $receiptRunInitialized[0] "profile_hash") -eq $runRunnerProfileHash)
$actualSuiteRunnerAttestationSha256 = Get-ReleaseFileSha256 $suiteRunnerAttestationPath
$attestationCommandLine = Get-ReleaseString $suiteRunnerAttestation "command_line"
$attestationCommandLineSha256 = Get-ReleaseStringSha256 $attestationCommandLine
$attestationSuiteRoot = Get-ReleaseString $suiteRunnerAttestation "suite_root"
$attestationRunnerNonce = Get-ReleaseString $suiteRunnerAttestation "runner_nonce"
$attestationReceiptEventHashBefore = Get-ReleaseString $suiteRunnerAttestation "suite_receipt_event_hash_before_attestation"
$attestationCommandLinePass = ($attestationCommandLine -match 'run-taskspace-e3-suite\.ps1' `
    -and $attestationCommandLine -notmatch 'test-release-decision' `
    -and $attestationCommandLine -notmatch 'fixture')
$attestationSuiteRootPass = $false
if (-not [string]::IsNullOrWhiteSpace($attestationSuiteRoot)) {
    try {
        $attestationSuiteRootFull = [System.IO.Path]::GetFullPath($attestationSuiteRoot).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
        $runRootFullForAttestation = [System.IO.Path]::GetFullPath($runRoot)
        $attestationSuiteRootPass = $runRootFullForAttestation.StartsWith($attestationSuiteRootFull + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
    } catch {
        $attestationSuiteRootPass = $false
    }
}
$attestationReceiptEvents = @($receiptRunnerAttestationGenerated | Where-Object {
        [string]$_.runner_nonce -eq $attestationRunnerNonce -and
        [string]$_.suite_runner_attestation_sha256 -eq $actualSuiteRunnerAttestationSha256 -and
        [string]$_.command_line_sha256 -eq $attestationCommandLineSha256 -and
        [string]$_.suite_receipt_event_hash_before_attestation -eq $attestationReceiptEventHashBefore -and
        (Get-ReleaseInt $_ "process_id" 0) -eq (Get-ReleaseInt $suiteRunnerAttestation "process_id" 0)
    })
$attestationReceiptChainPass = ($attestationReceiptEvents.Count -eq 1 `
    -and -not [string]::IsNullOrWhiteSpace($attestationRunnerNonce) `
    -and -not [string]::IsNullOrWhiteSpace($attestationReceiptEventHashBefore))
$suiteRunnerAttestationPass = ($suiteRunnerAttestation `
    -and $runSuiteRunnerAttestationSha256 -match '^[a-fA-F0-9]{64}$' `
    -and $runSuiteRunnerAttestationSha256 -eq $actualSuiteRunnerAttestationSha256 `
    -and (Test-ReleasePathUnderRoot $runRoot $runSuiteRunnerAttestationPath) `
    -and (Get-ReleaseString $suiteRunnerAttestation "artifact_origin") -eq "real_suite_runner" `
    -and (Get-ReleaseString $suiteRunnerAttestation "runner_entrypoint") -eq $runRunnerEntrypoint `
    -and (Get-ReleaseString $suiteRunnerAttestation "runner_script_sha256") -eq $runRunnerScriptSha256 `
    -and (Get-ReleaseString $suiteRunnerAttestation "child_runner_sha256") -eq $runChildRunnerSha256 `
    -and (Get-ReleaseString $suiteRunnerAttestation "task_list_sha256") -eq $runTaskListSha256 `
    -and (Get-ReleaseString $suiteRunnerAttestation "suite_manifest_sha256") -eq $runSuiteManifestSha256 `
    -and (Get-ReleaseString $suiteRunnerAttestation "suite_receipt_sha256_before_attestation") -match '^[a-fA-F0-9]{64}$' `
    -and (Get-ReleaseString $suiteRunnerAttestation "profile_hash") -eq $runRunnerProfileHash `
    -and (Get-ReleaseString $suiteRunnerAttestation "sample_set_id") -eq $runSampleSetId `
    -and $attestationCommandLinePass `
    -and $attestationSuiteRootPass `
    -and $attestationReceiptChainPass `
    -and (Get-ReleaseInt $suiteRunnerAttestation "process_id" 0) -gt 0)
$formalE3IdentityPass = ($runStatus `
    -and $runSampleSetId -eq $expectedFormalSampleSetId `
    -and $releaseTaskListDerivationPass `
    -and $runBenchmarkFamily -eq "terminal-bench" `
    -and $runRunnerEntrypoint -eq "run-taskspace-e3-suite.ps1" `
    -and $runRepeatsPerSample -ge 5 `
    -and $completedPairs -eq $formalPairCount `
    -and $runInitializedEvents.Count -gt 0 `
    -and [int]$runInitializedEvents[0].repeats -ge 5 `
    -and $sampleNamesMatch `
    -and $derivedSampleNamesMatch `
    -and $startGatePass `
    -and $gateDecisionPass `
    -and $codeCompleteMarkerPass `
    -and $userApprovalMarkerPass `
    -and $suiteProvenancePass `
    -and $suiteReceiptPass)

$evidence = [ordered]@{
    cost_gate_path = $costPath
    aggregate_path = $aggregatePath
    projection_summary_path = $projectionPath
    map_summary_path = $mapPath
    routing_summary_path = $routingPath
    cost_diagnostics_path = $costDiagnosticsPath
    run_status_path = $runStatusPath
    suite_manifest_path = $runSuiteManifestPath
    suite_runner_attestation_path = $suiteRunnerAttestationPath
    events_path = $eventsPath
    output_ref_events_path = $outputRefEventsPath
    budget_quality_impact_events_path = $budgetQualityImpactEventsPath
    budget_quality_impact_summary_path = $budgetQualityImpactSummaryPath
    required_artifacts = @($requiredArtifacts)
    start_gate_path = $startGatePath
    gate_decision_path = $gateDecisionPath
    code_complete_marker_path = $v005CodeCompleteMarkerPath
    user_approval_marker_path = $v005UserApprovalMarkerPath
}
$qualityPass = ($aggregate -and (Get-ReleaseBool $aggregate "score_valid") -and (Get-ReleaseString $aggregate "run_validity") -eq "valid")
$costStatus = Get-ReleaseString $cost "status" "MISSING"
$directInputOutputRatio = [double](Get-ReleaseString $cost.ratios "direct_input_output_ratio" "999")
$walltimeRatio = [double](Get-ReleaseString $cost.ratios "walltime_ratio" "999")
$modelRequestCountRatio = [double](Get-ReleaseString $cost.ratios "model_request_count_ratio" "999")
$formalP0CostCleanPass = ($directInputOutputRatio -le 2.0 -and $walltimeRatio -le 2.0 -and $modelRequestCountRatio -le 2.0)
$formalP0CostPartialPass = ($directInputOutputRatio -le 3.0 -and $walltimeRatio -le 3.0 -and $modelRequestCountRatio -le 2.5)
$projectionPass = ($projection `
    -and (Get-ReleaseInt $projection "missing_taskspace_projection_count" 0) -eq 0 `
    -and (Get-ReleaseInt $projection "taskspace_projection_protected_miss_count" 0) -eq 0 `
    -and (Get-ReleaseInt $projection "active_projection_count" 0) -gt 0 `
    -and (Get-ReleaseInt $projection "shadow_projection_count" 0) -eq 0)
$mapPass = ($map -and (Get-ReleaseString $map "availability") -eq "measured" -and (Get-ReleaseInt $map "protected_miss_count" 0) -eq 0)
$routingPass = ($routing -and (Get-ReleaseString $routing "availability") -eq "measured" -and (Get-ReleaseInt $routing "routing_mistake_count" 0) -eq 0)
$outputRefPass = ($maxLargeReplay -eq 0 -and $runtimeOutputRefs -gt 0 -and $validOutputRefCreatedEvents.Count -gt 0 -and $validOutputRefPairEvidence.Count -eq $completedPairs)
$providerRequestPass = (@($providerRequestEvents | Where-Object {
            ([string]$_.schema_version -eq "taskspace-provider-request-event-v1" -or [string]$_.schema_version -eq "taskspace-provider-request-budget-event-v1") -and
            [string]$_.producer -eq "provider_lifecycle"
        }).Count -gt 0)
$budgetResponsePass = (@($budgetEvents | Where-Object { [string]$_.schema_version -eq "taskspace-budget-event-v1" -and ([string]$_.status -eq "pass" -or [bool]$_.budget_response_action_taken) }).Count -gt 0)
$validBudgetQualityImpactEvents = @($budgetQualityImpactEvents | Where-Object { [string]$_.schema_version -eq "taskspace-budget-quality-impact-v1" })
$derivedBudgetValidationSkipCount = @($validBudgetQualityImpactEvents | Where-Object {
        (Get-ReleaseBool $_ "budget_induced_validation_skip") -or [string]$_.final_classification -eq "validation_skip"
    }).Count
$derivedBudgetScoreIneligibleSolvedCount = @($validBudgetQualityImpactEvents | Where-Object {
        -not (Get-ReleaseBool $_ "score_eligible" $true) -and [string]$_.final_classification -eq "solved"
    }).Count
$derivedBlockedByBudgetCount = @($validBudgetQualityImpactEvents | Where-Object {
        [string]$_.final_classification -eq "blocked_by_budget"
    }).Count
$derivedManualOverrideCount = @($validBudgetQualityImpactEvents | Where-Object {
        Get-ReleaseBool $_ "manual_override_used"
    }).Count
$budgetQualityImpactSummaryMatchesEvents = ($budgetQualityImpactSummary `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_validation_skip_count" 0) -eq $derivedBudgetValidationSkipCount `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_score_ineligible_solved_count" 0) -eq $derivedBudgetScoreIneligibleSolvedCount `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "blocked_by_budget_samples_count" 0) -eq $derivedBlockedByBudgetCount `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "manual_override_used_count" 0) -eq $derivedManualOverrideCount)
$budgetQualityImpactPass = ($budgetQualityImpactSummary `
    -and $validBudgetQualityImpactEvents.Count -gt 0 `
    -and (Get-ReleaseBool $budgetQualityImpactSummary "budget_quality_impact_logged_for_every_budget_action") `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "budget_quality_impact_missing_count" 0) -eq 0 `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_validation_skip_count" 0) -eq 0 `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_score_ineligible_solved_count" 0) -eq 0 `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "blocked_by_budget_samples_count" 0) -eq 0 `
    -and (Get-ReleaseInt $budgetQualityImpactSummary "manual_override_used_count" 0) -eq 0 `
    -and $budgetQualityImpactSummaryMatchesEvents `
    -and $derivedBudgetValidationSkipCount -eq 0 `
    -and $derivedBudgetScoreIneligibleSolvedCount -eq 0 `
    -and $derivedBlockedByBudgetCount -eq 0 `
    -and $derivedManualOverrideCount -eq 0)
$requestPhasePass = ($requestPhaseSummary `
    -and (Get-ReleaseInt $requestPhaseSummary "provider_request_hook_coverage" 0) -ge 99 `
    -and (Get-ReleaseInt $requestPhaseSummary "provider_request_terminal_coverage" 0) -ge 99 `
    -and (Get-ReleaseInt $requestPhaseSummary "request_phase_attribution_coverage" 0) -ge 95 `
    -and (Get-ReleaseInt $requestPhaseSummary "unknown_request_phase_ratio" 100) -le 5 `
    -and (Get-ReleaseInt $requestPhaseSummary "expected_model_request_count" 0) -gt 0 `
    -and (Get-ReleaseInt $requestPhaseSummary "provider_request_distinct_count" 0) -ge (Get-ReleaseInt $requestPhaseSummary "expected_model_request_count" 0) `
    -and (Get-ReleaseInt $requestPhaseSummary "provider_request_terminal_count" 0) -ge (Get-ReleaseInt $requestPhaseSummary "expected_model_request_count" 0))
$activeReplacementScanId = Get-ReleaseString $activeReplacement "exact_payload_scan_event_id"
$activeReplacementRequestId = Get-ReleaseString $activeReplacement "request_id"
$activeReplacementPayloadHash = Get-ReleaseString $activeReplacement "provider_payload_sha256"
$matchingExactScanEvents = @($exactPayloadScanEvents | Where-Object {
        [string]$_.schema_version -eq "taskspace-exact-payload-scan-event-v1" -and
        [bool]$_.passed -and
        [string]$_.scan_event_id -eq $activeReplacementScanId -and
        [string]$_.provider_payload_sha256 -eq $activeReplacementPayloadHash -and
        (-not [string]::IsNullOrWhiteSpace($activeReplacementRequestId) -and [string]$_.request_id -eq $activeReplacementRequestId)
    })
$matchingProviderPayloadEvents = @($providerRequestEvents | Where-Object {
        ([string]$_.schema_version -eq "taskspace-provider-request-event-v1" -or [string]$_.schema_version -eq "taskspace-provider-request-budget-event-v1") -and
        [string]$_.producer -eq "provider_lifecycle" -and
        [string]$_.request_id -eq $activeReplacementRequestId -and
        [string]$_.provider_payload_sha256 -eq $activeReplacementPayloadHash
    })
$exactScanPass = ($matchingExactScanEvents.Count -gt 0 -and $matchingProviderPayloadEvents.Count -gt 0)
$activeReplacementPass = ($activeReplacement `
    -and (Get-ReleaseBool $activeReplacement "provider_payload_available") `
    -and -not [string]::IsNullOrWhiteSpace($activeReplacementPayloadHash) `
    -and -not [string]::IsNullOrWhiteSpace($activeReplacementRequestId) `
    -and (Get-ReleaseBool $activeReplacement "exact_payload_scan_passed") `
    -and -not [string]::IsNullOrWhiteSpace($activeReplacementScanId) `
    -and (Get-ReleaseBool $activeReplacement "replacement_confirmed") `
    -and -not (Get-ReleaseBool $activeReplacement "legacy_taskspace_history_present" $true) `
    -and (Get-ReleaseInt $activeReplacement "large_raw_output_tokens" 1) -eq 0 `
    -and (Get-ReleaseBool $activeReplacement "protected_items_present") `
    -and $exactScanPass)
$stateCommitDisplacementPass = ($stateCommitDisplacement `
    -and (Get-ReleaseString $stateCommitDisplacement "status") -eq "pass" `
    -and (Get-ReleaseBool $stateCommitDisplacement "has_displacement_denominator") `
    -and (Get-ReleaseInt $stateCommitDisplacement "legacy_state_action_attempt_count" 0) -gt 0 `
    -and (Get-ReleaseInt $stateCommitDisplacement "legacy_state_action_displaced_count" 0) -ge (Get-ReleaseInt $stateCommitDisplacement "legacy_state_action_attempt_count" 0) `
    -and (Get-ReleaseInt $stateCommitDisplacement "legacy_state_action_count" 999999) -le (Get-ReleaseInt $stateCommitDisplacement "legacy_state_action_budget" 0))
$spawnNodeBudgetPass = ($spawnNodeBudget -and (Get-ReleaseString $spawnNodeBudget "status") -eq "pass" -and (Get-ReleaseString $spawnNodeBudget "within_budget_status") -eq "pass")
$requiredV005NonAgentGates = @(
    "provider_request_hook",
    "runtime_budget_response",
    "budget_quality_impact",
    "active_context_replacement",
    "state_commit_displacement",
    "spawn_node_budget",
    "request_phase_attribution",
    "release_decision_fixture",
    "start_gate_fixture"
)
$v005NonAgentGatesPass = ($v005NonAgentGates -and (Get-ReleaseString $v005NonAgentGates "status") -eq "pass" -and (Get-ReleaseInt $v005NonAgentGates "schema_version" 0) -eq 1)
if ($v005NonAgentGatesPass) {
    foreach ($gateName in $requiredV005NonAgentGates) {
        $gateValue = $null
        if ($v005NonAgentGates.PSObject.Properties.Name -contains "gates") {
            $gateValue = $v005NonAgentGates.gates.$gateName
        }
        if (-not $gateValue -or [string]$gateValue.status -ne "pass" -or [string]::IsNullOrWhiteSpace([string]$gateValue.evidence_path)) {
            $v005NonAgentGatesPass = $false
            break
        }
        if (-not (Test-ReleaseEvidencePathAllowed $runRoot ([string]$gateValue.evidence_path))) {
            $v005NonAgentGatesPass = $false
            break
        }
        if ([string]::IsNullOrWhiteSpace([string]$gateValue.command) -or (Get-ReleaseInt $gateValue "exit_code" -999) -ne 0 -or [string]::IsNullOrWhiteSpace([string]$gateValue.generated_at) -or [string]::IsNullOrWhiteSpace([string]$gateValue.git_commit) -or [string]::IsNullOrWhiteSpace([string]$gateValue.profile_hash) -or [string]::IsNullOrWhiteSpace([string]$gateValue.task_list_hash) -or [string]::IsNullOrWhiteSpace([string]$gateValue.source_version) -or [string]::IsNullOrWhiteSpace([string]$gateValue.evidence_sha256)) {
            $v005NonAgentGatesPass = $false
            break
        }
        if ([string]::IsNullOrWhiteSpace([string]$currentHeadForRelease) -or [string]$gateValue.git_commit -ne [string]$currentHeadForRelease) {
            $v005NonAgentGatesPass = $false
            break
        }
        $evidenceSha = Get-ReleaseFileSha256 ([string]$gateValue.evidence_path)
        if ($evidenceSha -ne ([string]$gateValue.evidence_sha256).ToLowerInvariant() -or [string]$gateValue.profile_hash -ne $runRunnerProfileHash -or [string]$gateValue.task_list_hash -ne $runTaskListHash -or [string]$gateValue.source_version -ne $runSourceVersion) {
            $v005NonAgentGatesPass = $false
            break
        }
    }
}

$blockers = New-Object System.Collections.Generic.List[string]
foreach ($artifactName in $requiredArtifacts) {
    if (-not (Test-Path -LiteralPath (Join-Path $runRoot $artifactName))) {
        Add-ReleaseLine $blockers "required_artifact_missing:$artifactName"
    }
}
if (-not $qualityPass) { Add-ReleaseLine $blockers "quality_gate_failed" }
if ($costStatus -eq "MISSING") { Add-ReleaseLine $blockers "cost_gate_missing" }
elseif ($costStatus -eq "FAIL") { Add-ReleaseLine $blockers "cost_gate_failed" }
if (-not $projectionPass) { Add-ReleaseLine $blockers "projection_gate_failed" }
if (-not $mapPass) { Add-ReleaseLine $blockers "map_gate_failed" }
if (-not $routingPass) { Add-ReleaseLine $blockers "routing_gate_failed" }
if (-not $outputRefPass) { Add-ReleaseLine $blockers "output_ref_replay_failed" }
if (-not $runProvenancePass) { Add-ReleaseLine $blockers "run_provenance_gate_failed" }
if (-not $providerRequestPass) { Add-ReleaseLine $blockers "provider_request_event_missing" }
if (-not $budgetResponsePass) { Add-ReleaseLine $blockers "runtime_budget_response_gate_failed" }
if (-not $budgetQualityImpactPass) { Add-ReleaseLine $blockers "budget_quality_impact_gate_failed" }
if (-not $requestPhasePass) { Add-ReleaseLine $blockers "request_phase_attribution_missing" }
if (-not $activeReplacementPass) { Add-ReleaseLine $blockers "active_context_replacement_gate_failed" }
if (-not $stateCommitDisplacementPass) { Add-ReleaseLine $blockers "state_commit_displacement_gate_failed" }
if (-not $spawnNodeBudgetPass) { Add-ReleaseLine $blockers "spawn_budget_gate_failed" }
if (-not $v005NonAgentGatesPass) { Add-ReleaseLine $blockers "v005_non_agent_gates_failed" }
if (-not $formalE3IdentityPass) { Add-ReleaseLine $blockers "formal_e3_identity_gate_failed" }
if (-not $releaseTaskListDerivationPass) { Add-ReleaseLine $blockers "formal_e3_task_list_derivation_failed" }
if ($directInputOutputRatio -gt 3.0) { Add-ReleaseLine $blockers "formal_p0_direct_input_output_ratio_gate_failed" }
if ($walltimeRatio -gt 3.0) { Add-ReleaseLine $blockers "formal_p0_walltime_ratio_gate_failed" }
if ($modelRequestCountRatio -gt 2.5) { Add-ReleaseLine $blockers "formal_p0_request_ratio_gate_failed" }
if (-not $suiteProvenancePass) { Add-ReleaseLine $blockers "formal_e3_provenance_gate_failed" }
if (-not $suiteReceiptPass) { Add-ReleaseLine $blockers "suite_receipt_gate_failed" }
if (-not $suiteReceiptHashChainPass) { Add-ReleaseLine $blockers "suite_receipt_hash_chain_failed" }
if (-not $suiteRunnerAttestationPass) { Add-ReleaseLine $blockers "suite_runner_attestation_gate_failed" }
if (-not $codeCompleteMarkerPass) { Add-ReleaseLine $blockers "v005_code_complete_marker_failed" }
if (-not $userApprovalMarkerPass) { Add-ReleaseLine $blockers "v005_user_approval_marker_failed" }
if ($aggregate -and (Get-ReleaseInt $aggregate "excluded_pairs" 0) -gt 0) { Add-ReleaseLine $blockers "excluded_pairs_present" }

$decision = "fail"
$closeable = $false
if ($qualityPass -and $projectionPass -and $mapPass -and $routingPass -and $outputRefPass -and $runProvenancePass -and $formalE3IdentityPass -and $suiteProvenancePass -and $suiteReceiptPass -and $suiteRunnerAttestationPass -and $codeCompleteMarkerPass -and $userApprovalMarkerPass -and $providerRequestPass -and $budgetResponsePass -and $budgetQualityImpactPass -and $requestPhasePass -and $activeReplacementPass -and $stateCommitDisplacementPass -and $spawnNodeBudgetPass -and $v005NonAgentGatesPass) {
    if ($costStatus -eq "PASS" -and $formalP0CostCleanPass -and $blockers.Count -eq 0) {
        $decision = "release_pass"
        $closeable = $true
    } elseif ($costStatus -in @("PASS", "PARTIAL") -and $formalP0CostPartialPass -and $blockers.Count -eq 0) {
        $decision = "blocked_partial"
    }
}

$summary = [pscustomobject]@{
    schema_version = "taskspace-v005-release-decision-v1"
    decision = $decision
    closeable = [bool]$closeable
    run_dir = $runRoot
    cost_status = $costStatus
    quality_gate_pass = [bool]$qualityPass
    projection_gate_pass = [bool]$projectionPass
    map_gate_pass = [bool]$mapPass
    routing_gate_pass = [bool]$routingPass
    output_ref_gate_pass = [bool]$outputRefPass
    run_provenance_gate_pass = [bool]$runProvenancePass
    formal_e3_identity_gate_pass = [bool]$formalE3IdentityPass
    formal_e3_pair_sample_ledger_pass = [bool]$formalPairSampleLedgerPass
    formal_e3_pair_sample_repeat_count = [int]$pairSampleRepeatKeys.Count
    formal_e3_pair_counts_by_sample = [pscustomobject]$formalPairCountsBySample
    formal_e3_provenance_gate_pass = [bool]$suiteProvenancePass
    suite_receipt_gate_pass = [bool]$suiteReceiptPass
    suite_receipt_hash_chain_pass = [bool]$suiteReceiptHashChainPass
    suite_runner_attestation_gate_pass = [bool]$suiteRunnerAttestationPass
    suite_runner_attestation_command_line_pass = [bool]$attestationCommandLinePass
    suite_runner_attestation_suite_root_pass = [bool]$attestationSuiteRootPass
    suite_runner_attestation_receipt_chain_pass = [bool]$attestationReceiptChainPass
    suite_runner_attestation_receipt_event_count = [int]$attestationReceiptEvents.Count
    suite_runner_attestation_sha256 = $runSuiteRunnerAttestationSha256
    artifact_origin = $runArtifactOrigin
    sample_set_id = $runSampleSetId
    sample_names = @($runSampleNames)
    task_list_identity_source = $taskListIdentitySource
    task_list_derivation_gate_pass = [bool]$releaseTaskListDerivationPass
    derived_sample_set_id = $derivedSampleSetId
    derived_sample_names = @($derivedSampleNames)
    repeats_per_sample = [int]$runRepeatsPerSample
    benchmark_family = $runBenchmarkFamily
    runner_entrypoint = $runRunnerEntrypoint
    runner_profile_hash = $runRunnerProfileHash
    runner_script_sha256 = $runRunnerScriptSha256
    child_runner_sha256 = $runChildRunnerSha256
    task_list_sha256 = $runTaskListSha256
    suite_receipt_sha256 = $runSuiteReceiptSha256
    suite_receipt_formal_sample_coverage_pass = [bool]$suiteReceiptFormalSampleCoveragePass
    suite_receipt_scheduled_samples = @($receiptScheduledSamples)
    suite_receipt_completed_pairs_by_sample = [pscustomobject]$receiptCompletedPairsBySample
    approval_marker_sha256 = $runApprovalMarkerSha256
    code_complete_marker_sha256 = $runCodeCompleteMarkerSha256
    start_gate_decision_path = $gateDecisionPath
    code_complete_marker_pass = [bool]$codeCompleteMarkerPass
    user_approval_marker_pass = [bool]$userApprovalMarkerPass
    provider_request_gate_pass = [bool]$providerRequestPass
    budget_response_gate_pass = [bool]$budgetResponsePass
    budget_quality_impact_gate_pass = [bool]$budgetQualityImpactPass
    budget_quality_impact_event_count = [int]$validBudgetQualityImpactEvents.Count
    budget_quality_impact_missing_count = Get-ReleaseInt $budgetQualityImpactSummary "budget_quality_impact_missing_count" 0
    budget_induced_validation_skip_count = Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_validation_skip_count" 0
    budget_induced_score_ineligible_solved_count = Get-ReleaseInt $budgetQualityImpactSummary "budget_induced_score_ineligible_solved_count" 0
    blocked_by_budget_samples_count = Get-ReleaseInt $budgetQualityImpactSummary "blocked_by_budget_samples_count" 0
    manual_override_used_count = Get-ReleaseInt $budgetQualityImpactSummary "manual_override_used_count" 0
    budget_quality_impact_summary_matches_events = [bool]$budgetQualityImpactSummaryMatchesEvents
    formal_p0_cost_clean_pass = [bool]$formalP0CostCleanPass
    formal_p0_cost_partial_pass = [bool]$formalP0CostPartialPass
    direct_input_output_ratio = $directInputOutputRatio
    walltime_ratio = $walltimeRatio
    model_request_count_ratio = $modelRequestCountRatio
    derived_budget_induced_validation_skip_count = [int]$derivedBudgetValidationSkipCount
    derived_budget_induced_score_ineligible_solved_count = [int]$derivedBudgetScoreIneligibleSolvedCount
    derived_blocked_by_budget_count = [int]$derivedBlockedByBudgetCount
    derived_manual_override_count = [int]$derivedManualOverrideCount
    request_phase_gate_pass = [bool]$requestPhasePass
    active_replacement_gate_pass = [bool]$activeReplacementPass
    exact_payload_scan_gate_pass = [bool]$exactScanPass
    exact_payload_scan_matching_provider_event_count = [int]$matchingProviderPayloadEvents.Count
    state_commit_displacement_gate_pass = [bool]$stateCommitDisplacementPass
    spawn_node_budget_gate_pass = [bool]$spawnNodeBudgetPass
    v005_non_agent_gates_pass = [bool]$v005NonAgentGatesPass
    max_large_output_replay_count = [int]$maxLargeReplay
    runtime_output_ref_created_count = [int]$runtimeOutputRefs
    valid_output_ref_created_event_count = [int]$validOutputRefCreatedEvents.Count
    standard_metric_count = [int]$standardMetrics.Count
    taskspace_metric_count = [int]$taskspaceMetrics.Count
    completed_pair_count = [int]$completedPairs
    completed_pair_event_count = [int]$pairCompletedEvents.Count
    valid_pair_evidence_count = [int]$validPairEvidence.Count
    output_ref_correlated_pair_count = [int]$validOutputRefPairEvidence.Count
    accepted_pair_dirs = @($validPairEvidence | ForEach-Object { [string]$_.pair_dir })
    accepted_pair_repeats = @($validPairEvidence | ForEach-Object { [int]$_.repeat })
    routing_decision_event_count = [int]$routingDecisionEvents.Count
    latest_routing_decision_path = if ($latestRoutingDecisionEvent.Count -eq 1) { [string]$latestRoutingDecisionEvent[0].path } else { "" }
    latest_routing_decision_pass = [bool]$routingDecisionPass
    cost_root_cause = Get-ReleaseString $costDiagnostics "root_cause" ""
    cost_drivers = if ($costDiagnostics -and $costDiagnostics.PSObject.Properties.Name -contains "drivers") { @($costDiagnostics.drivers) } else { @() }
    blockers = @($blockers.ToArray())
    evidence = [pscustomobject]$evidence
}
$summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$lines = New-Object System.Collections.Generic.List[string]
Add-ReleaseLine $lines "# TaskSpace v0.0.5 Release Decision"
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "- decision: $decision"
Add-ReleaseLine $lines "- closeable: $closeable"
Add-ReleaseLine $lines "- run_dir: $runRoot"
Add-ReleaseLine $lines "- cost_status: $costStatus"
Add-ReleaseLine $lines "- quality_gate_pass: $qualityPass"
Add-ReleaseLine $lines "- projection_gate_pass: $projectionPass"
Add-ReleaseLine $lines "- map_gate_pass: $mapPass"
Add-ReleaseLine $lines "- routing_gate_pass: $routingPass"
Add-ReleaseLine $lines "- output_ref_gate_pass: $outputRefPass"
Add-ReleaseLine $lines "- run_provenance_gate_pass: $runProvenancePass"
Add-ReleaseLine $lines "- formal_e3_identity_gate_pass: $formalE3IdentityPass"
Add-ReleaseLine $lines "- formal_e3_provenance_gate_pass: $suiteProvenancePass"
Add-ReleaseLine $lines "- suite_receipt_gate_pass: $suiteReceiptPass"
Add-ReleaseLine $lines "- suite_receipt_hash_chain_pass: $suiteReceiptHashChainPass"
Add-ReleaseLine $lines "- suite_runner_attestation_gate_pass: $suiteRunnerAttestationPass"
Add-ReleaseLine $lines "- suite_runner_attestation_command_line_pass: $attestationCommandLinePass"
Add-ReleaseLine $lines "- suite_runner_attestation_suite_root_pass: $attestationSuiteRootPass"
Add-ReleaseLine $lines "- suite_runner_attestation_receipt_chain_pass: $attestationReceiptChainPass"
Add-ReleaseLine $lines "- suite_runner_attestation_receipt_event_count: $($attestationReceiptEvents.Count)"
Add-ReleaseLine $lines "- artifact_origin: $runArtifactOrigin"
Add-ReleaseLine $lines "- sample_set_id: $runSampleSetId"
Add-ReleaseLine $lines "- task_list_identity_source: $taskListIdentitySource"
Add-ReleaseLine $lines "- task_list_derivation_gate_pass: $releaseTaskListDerivationPass"
Add-ReleaseLine $lines "- derived_sample_set_id: $derivedSampleSetId"
Add-ReleaseLine $lines "- repeats_per_sample: $runRepeatsPerSample"
Add-ReleaseLine $lines "- runner_entrypoint: $runRunnerEntrypoint"
Add-ReleaseLine $lines "- runner_profile_hash: $runRunnerProfileHash"
Add-ReleaseLine $lines "- runner_script_sha256: $runRunnerScriptSha256"
Add-ReleaseLine $lines "- child_runner_sha256: $runChildRunnerSha256"
Add-ReleaseLine $lines "- task_list_sha256: $runTaskListSha256"
Add-ReleaseLine $lines "- suite_receipt_sha256: $runSuiteReceiptSha256"
Add-ReleaseLine $lines "- formal_p0_cost_clean_pass: $formalP0CostCleanPass"
Add-ReleaseLine $lines "- formal_p0_cost_partial_pass: $formalP0CostPartialPass"
Add-ReleaseLine $lines "- code_complete_marker_pass: $codeCompleteMarkerPass"
Add-ReleaseLine $lines "- user_approval_marker_pass: $userApprovalMarkerPass"
Add-ReleaseLine $lines "- start_gate_decision_path: $gateDecisionPath"
Add-ReleaseLine $lines "- max_large_output_replay_count: $maxLargeReplay"
Add-ReleaseLine $lines "- runtime_output_ref_created_count: $runtimeOutputRefs"
Add-ReleaseLine $lines "- valid_output_ref_created_event_count: $($validOutputRefCreatedEvents.Count)"
Add-ReleaseLine $lines "- standard_metric_count: $($standardMetrics.Count)"
Add-ReleaseLine $lines "- taskspace_metric_count: $($taskspaceMetrics.Count)"
Add-ReleaseLine $lines "- completed_pair_count: $completedPairs"
Add-ReleaseLine $lines "- completed_pair_event_count: $($pairCompletedEvents.Count)"
Add-ReleaseLine $lines "- valid_pair_evidence_count: $($validPairEvidence.Count)"
Add-ReleaseLine $lines "- output_ref_correlated_pair_count: $($validOutputRefPairEvidence.Count)"
Add-ReleaseLine $lines "- routing_decision_event_count: $($routingDecisionEvents.Count)"
Add-ReleaseLine $lines "- latest_routing_decision_pass: $routingDecisionPass"
Add-ReleaseLine $lines "- latest_routing_decision_path: $(if ($latestRoutingDecisionEvent.Count -eq 1) { [string]$latestRoutingDecisionEvent[0].path } else { '' })"
Add-ReleaseLine $lines "- blockers: $(if ($blockers.Count -eq 0) { 'none' } else { @($blockers.ToArray()) -join ', ' })"
if ($decision -eq "blocked_partial") {
    Add-ReleaseLine $lines "- release_note: blocked_partial is engineering progress only and cannot close v0.0.5."
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Evidence Paths"
foreach ($entry in $evidence.GetEnumerator()) {
    Add-ReleaseLine $lines "- $($entry.Key): $($entry.Value)"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Cost"
if ($cost) {
    Add-ReleaseLine $lines "- direct_input_output_ratio: $($cost.ratios.direct_input_output_ratio)"
    Add-ReleaseLine $lines "- walltime_ratio: $($cost.ratios.walltime_ratio)"
    Add-ReleaseLine $lines "- model_request_count_ratio: $($cost.ratios.model_request_count_ratio)"
    if ($costDiagnostics) {
        Add-ReleaseLine $lines "- root_cause: $($costDiagnostics.root_cause)"
        Add-ReleaseLine $lines "- drivers: $(@($costDiagnostics.drivers) -join ', ')"
        Add-ReleaseLine $lines "- rollout_trace_model_request_count_ratio: $($costDiagnostics.ratios.rollout_trace_model_request_count_ratio)"
        Add-ReleaseLine $lines "- uncached_input_ratio: $($costDiagnostics.ratios.uncached_input_ratio)"
        Add-ReleaseLine $lines "- projection_token_share_of_taskspace_input: $($costDiagnostics.ratios.projection_token_share_of_taskspace_input)"
    }
} else {
    Add-ReleaseLine $lines "- unavailable"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Quality"
if ($aggregate) {
    Add-ReleaseLine $lines "- score_valid: $($aggregate.score_valid)"
    Add-ReleaseLine $lines "- run_validity: $($aggregate.run_validity)"
    Add-ReleaseLine $lines "- both_success: $($aggregate.both_success)"
    Add-ReleaseLine $lines "- both_failed: $($aggregate.both_failed)"
    Add-ReleaseLine $lines "- excluded_pairs: $($aggregate.excluded_pairs)"
    Add-ReleaseLine $lines "- excluded_by_reason: $($aggregate.excluded_by_reason | ConvertTo-Json -Compress)"
} else {
    Add-ReleaseLine $lines "- unavailable"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Routing"
if ($routing) {
    Add-ReleaseLine $lines "- recommended_mode: $($routing.recommended_mode)"
    Add-ReleaseLine $lines "- router_status: $($routing.router_status)"
    Add-ReleaseLine $lines "- routing_mistake_count: $($routing.routing_mistake_count)"
    Add-ReleaseLine $lines "- verification_first_expected_format_count: $($routing.verification_first_expected_format_count)"
} else {
    Add-ReleaseLine $lines "- unavailable"
}
$lines | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Host "ReleaseDecision: $OutputPath"
Write-Host "ReleaseDecisionJson: $jsonPath"
if ($decision -eq "release_pass") { exit 0 }
if ($decision -eq "blocked_partial") { exit 2 }
exit 1
