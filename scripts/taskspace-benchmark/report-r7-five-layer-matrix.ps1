param(
    [Parameter(Mandatory = $true)][string]$RunRoot
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")
. (Join-Path $PSScriptRoot "lib/r7-artifact-provenance.ps1")
if (-not [IO.Path]::IsPathRooted($RunRoot)) { $RunRoot = Join-Path $repoRoot $RunRoot }
$manifestPath = Join-Path $RunRoot "run-manifest.json"
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50

function Get-Value {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    $Default
}

function Get-Median {
    param([object[]]$Values)
    $numbers = @($Values | Where-Object { $null -ne $_ } | ForEach-Object { [double]$_ } | Sort-Object)
    if ($numbers.Count -eq 0) { return $null }
    $middle = [Math]::Floor($numbers.Count / 2)
    if ($numbers.Count % 2 -eq 1) { return $numbers[$middle] }
    ($numbers[$middle - 1] + $numbers[$middle]) / 2
}

function Get-AggregateRow {
    param([object[]]$Rows, [string]$Scope, [string]$Sample, [string]$Arm)
    $requestTotal = ($Rows | Measure-Object -Property provider_requests -Sum).Sum
    $inputTotal = ($Rows | Measure-Object -Property input_tokens -Sum).Sum
    $cachedTotal = ($Rows | Measure-Object -Property cached_input_tokens -Sum).Sum
    $uncachedTotal = ($Rows | Measure-Object -Property uncached_input_tokens -Sum).Sum
    $outputTotal = ($Rows | Measure-Object -Property output_tokens -Sum).Sum
    $wallTotal = ($Rows | Measure-Object -Property wall_time_ms -Sum).Sum
    $request2Input = ($Rows | Measure-Object -Property request_2_plus_input_tokens -Sum).Sum
    $request2Cached = ($Rows | Measure-Object -Property request_2_plus_cached_input_tokens -Sum).Sum
    [pscustomobject]@{
        scope = $Scope
        sample = $Sample
        arm = $Arm
        runs = $Rows.Count
        successes = @($Rows | Where-Object { [bool]$_.business_success }).Count
        requests_total = $requestTotal
        requests_mean = if ($Rows.Count) { [Math]::Round($requestTotal / $Rows.Count, 3) } else { $null }
        requests_median = Get-Median @($Rows.provider_requests)
        completed_provider_responses_total = (
            $Rows | Measure-Object -Property completed_provider_responses -Sum
        ).Sum
        failed_or_cancelled_provider_attempts_total = (
            $Rows | Measure-Object -Property failed_or_cancelled_provider_attempts -Sum
        ).Sum
        retried_logical_requests_total = (
            $Rows | Measure-Object -Property retried_logical_requests -Sum
        ).Sum
        tool_action_requests_total = ($Rows | Measure-Object -Property tool_action_requests -Sum).Sum
        assistant_only_requests_total = ($Rows | Measure-Object -Property assistant_only_requests -Sum).Sum
        multi_tool_requests_total = ($Rows | Measure-Object -Property multi_tool_requests -Sum).Sum
        no_failure_requests_total = ($Rows | Measure-Object -Property no_failure_requests -Sum).Sum
        tool_sequence_protocol_failure_requests_total = ($Rows | Measure-Object -Property tool_sequence_protocol_failure_requests -Sum).Sum
        taskspace_protocol_failure_requests_total = ($Rows | Measure-Object -Property taskspace_protocol_failure_requests -Sum).Sum
        taskspace_state_failure_requests_total = ($Rows | Measure-Object -Property taskspace_state_failure_requests -Sum).Sum
        taskspace_resource_failure_requests_total = ($Rows | Measure-Object -Property taskspace_resource_failure_requests -Sum).Sum
        ordinary_failure_requests_total = ($Rows | Measure-Object -Property ordinary_failure_requests -Sum).Sum
        sibling_failure_copy_count_total = ($Rows | Measure-Object -Property sibling_failure_copy_count -Sum).Sum
        classification_unreconciled_runs = @($Rows | Where-Object classification_reconciled -ne $true).Count
        echo_only_handoffs_total = ($Rows | Measure-Object -Property echo_only_handoffs -Sum).Sum
        initialize_and_execute_total = ($Rows | Measure-Object -Property initialize_and_execute -Sum).Sum
        committed_initialize_and_execute_total = ($Rows | Measure-Object -Property committed_initialize_and_execute -Sum).Sum
        failed_initialize_and_execute_total = ($Rows | Measure-Object -Property failed_initialize_and_execute -Sum).Sum
        first_request_initialization_total = ($Rows | Measure-Object -Property first_request_initialization -Sum).Sum
        first_request_initialization_commits_total = ($Rows | Measure-Object -Property first_request_initialization_commits -Sum).Sum
        direct_initialize_control_total = ($Rows | Measure-Object -Property direct_initialize_control -Sum).Sum
        no_task_path_rejections_total = ($Rows | Measure-Object -Property no_task_path_rejections -Sum).Sum
        input_tokens_total = $inputTotal
        input_tokens_mean = if ($Rows.Count) { [Math]::Round($inputTotal / $Rows.Count, 3) } else { $null }
        input_tokens_median = Get-Median @($Rows.input_tokens)
        cached_input_tokens_total = $cachedTotal
        uncached_input_tokens_total = $uncachedTotal
        output_tokens_total = $outputTotal
        output_tokens_mean = if ($Rows.Count) { [Math]::Round($outputTotal / $Rows.Count, 3) } else { $null }
        output_tokens_median = Get-Median @($Rows.output_tokens)
        request_2_plus_cache_hit_rate = if ($request2Input -gt 0) { [Math]::Round($request2Cached / $request2Input, 6) } else { $null }
        receipt_before_requests_total = ($Rows | Measure-Object -Property receipt_before_requests -Sum).Sum
        receipt_before_input_tokens_total = ($Rows | Measure-Object -Property receipt_before_input_tokens -Sum).Sum
        receipt_before_cached_input_tokens_total = ($Rows | Measure-Object -Property receipt_before_cached_input_tokens -Sum).Sum
        no_receipt_before_requests_total = ($Rows | Measure-Object -Property no_receipt_before_requests -Sum).Sum
        no_receipt_before_input_tokens_total = ($Rows | Measure-Object -Property no_receipt_before_input_tokens -Sum).Sum
        no_receipt_before_cached_input_tokens_total = ($Rows | Measure-Object -Property no_receipt_before_cached_input_tokens -Sum).Sum
        receipt_wire_role_unresolved_count_total = ($Rows | Measure-Object -Property receipt_wire_role_unresolved_count -Sum).Sum
        wall_time_ms_total = $wallTotal
        wall_time_ms_mean = if ($Rows.Count) { [Math]::Round($wallTotal / $Rows.Count, 3) } else { $null }
        wall_time_ms_median = Get-Median @($Rows.wall_time_ms)
        tool_calls_total = ($Rows | Measure-Object -Property ordinary_tools -Sum).Sum
        failed_tools_total = ($Rows | Measure-Object -Property failed_tools -Sum).Sum
        taskspace_control_total = ($Rows | Measure-Object -Property taskspace_control -Sum).Sum
        control_failures_total = ($Rows | Measure-Object -Property control_failures -Sum).Sum
        control_protocol_failures_total = ($Rows | Measure-Object -Property control_protocol_failures -Sum).Sum
        control_state_failures_total = ($Rows | Measure-Object -Property control_state_failures -Sum).Sum
        multi_patch_attempts_total = ($Rows | Measure-Object -Property multi_patch_attempts -Sum).Sum
        patch_prepare_failures_total = ($Rows | Measure-Object -Property patch_prepare_failures -Sum).Sum
        map_nodes_mean = [Math]::Round((($Rows | Measure-Object -Property map_nodes -Average).Average), 3)
        map_edges_mean = [Math]::Round((($Rows | Measure-Object -Property map_edges -Average).Average), 3)
        first_input_tokens_mean = [Math]::Round((($Rows | Measure-Object -Property first_input_tokens -Average).Average), 3)
        tools_section_tokens_mean = if ($requestTotal) { [Math]::Round((($Rows | Measure-Object -Property tools_section_tokens_total -Sum).Sum) / $requestTotal, 3) } else { 0 }
        projection_section_tokens_mean = if ($requestTotal) { [Math]::Round((($Rows | Measure-Object -Property projection_section_tokens_total -Sum).Sum) / $requestTotal, 3) } else { 0 }
    }
}

if ([string]$manifest.status -ne "completed") { throw "Matrix run is not completed: $($manifest.status)" }
$expectedRuns = [int]$manifest.repeats_per_arm_per_sample * @($manifest.samples).Count * 4
if ([int]$manifest.completed_run_count -ne $expectedRuns) { throw "Matrix run count is incomplete" }
$artifactProvenance = Get-R7MatrixArtifactProvenance $repoRoot $manifestPath $manifest $PSCommandPath
$artifactProvenancePath = Join-Path $RunRoot "artifact-provenance.json"
[IO.File]::WriteAllText(
    $artifactProvenancePath,
    (($artifactProvenance | ConvertTo-Json -Depth 50) + "`n"),
    [Text.UTF8Encoding]::new($false)
)
if ([string]$artifactProvenance.status -ne "valid") {
    $codes = @($artifactProvenance.findings | ForEach-Object { [string]$_.code } | Sort-Object -Unique) -join ","
    throw "Matrix artifact provenance is invalid: $codes"
}

$rows = [Collections.Generic.List[object]]::new()
$traceRuns = [Collections.Generic.List[object]]::new()
$imageDigests = [Collections.Generic.List[string]]::new()
foreach ($run in @($manifest.runs)) {
    $observationPath = Join-Path ([string]$run.run_dir) "performance-observation.json"
    if (-not (Test-Path -LiteralPath $observationPath -PathType Leaf)) {
        & (Join-Path $PSScriptRoot "write-performance-observation.ps1") -RunRoot ([string]$run.run_dir) | Out-Null
    }
    $observation = Get-Content -Raw -Encoding UTF8 -LiteralPath $observationPath | ConvertFrom-Json -Depth 100
    $actualRows = @($observation.rows | Where-Object { [string]$_.observation_status -eq "complete" -and [string]$_.logical_mode -eq [string]$run.logical_mode })
    if ($actualRows.Count -ne 1) { throw "Expected one complete row for $($run.sample) repeat $($run.repeat) $($run.arm)" }
    $row = $actualRows[0]
    $request2Input = [double](Get-Value $row.cache "request_2_plus_cached_input_tokens" 0) + [double](Get-Value $row.cache "request_2_plus_uncached_input_tokens" 0)
    $finishNode = @((Get-Value $row.map "nodes" @()) | Where-Object { [string]$_.kind -eq "finish" } | Select-Object -First 1)
    $rootNode = @((Get-Value $row.map "nodes" @()) | Where-Object { [string]$_.kind -eq "task_root" } | Select-Object -First 1)
    $resolvedPath = Join-Path ([string]$run.run_dir) "pair-001/manifest.resolved.json"
    $resolved = Get-Content -Raw -Encoding UTF8 -LiteralPath $resolvedPath | ConvertFrom-Json -Depth 50
    $imageDigests.Add([string]$resolved.container_image_digest)
    $wireTracePath = Join-Path ([string]$row.artifact_dir) "provider-wire-trace.jsonl"
    $rolloutPath = Join-Path ([string]$row.artifact_dir) "rollout.jsonl"
    $requestSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path ([string]$row.artifact_dir) "request-summary.json") | ConvertFrom-Json -Depth 50
    $sectionSummary = Get-R7WireSectionSummary $wireTracePath
    $wireInventory = @(Get-R7WireRequestInventory $wireTracePath)
    $wireAttemptSummary = Get-R7WireAttemptSummary $wireInventory
    $completedProviderResponses = [int](Get-Value $requestSummary "model_request_count" 0)
    if ($completedProviderResponses -lt 1) {
        throw "Completed provider response count is unavailable for $($run.sample) repeat $($run.repeat) $($run.arm)"
    }
    $flat = [pscustomobject]@{
        sample = [string]$run.sample
        repeat = [int]$run.repeat
        arm = [string]$run.arm
        logical_mode = [string]$run.logical_mode
        projection_policy = [string]$run.projection_policy
        observation_status = [string]$row.observation_status
        business_success = [bool]$row.result.business_success
        agent_completion_status = [string]$row.result.agent_completion_status
        provider_requests = [double](Get-Value $row.actions "provider_requests" 0)
        completed_provider_responses = [double]$completedProviderResponses
        failed_or_cancelled_provider_attempts = [double]$wireAttemptSummary.failed_or_cancelled_attempt_count
        retried_logical_requests = [double]$wireAttemptSummary.retried_logical_request_count
        ordinary_tools = [double](Get-Value $row.actions "ordinary_tools" 0)
        failed_tools = [double](Get-Value $row.actions "failed_tools" 0)
        taskspace_control = [double](Get-Value $row.actions "taskspace_control" 0)
        initialize_and_execute = [double](Get-Value $row.actions "initialize_and_execute" 0)
        committed_initialize_and_execute = [double](Get-Value $row.actions "committed_initialize_and_execute" 0)
        failed_initialize_and_execute = [double](Get-Value $row.actions "failed_initialize_and_execute" 0)
        control_failures = [double](Get-Value $row.actions "control_failures" 0)
        control_protocol_failures = [double](Get-Value $row.actions "control_protocol_failures" 0)
        control_state_failures = [double](Get-Value $row.actions "control_state_failures" 0)
        multi_patch_attempts = [double](Get-Value $row.patch "request_multi_patch_attempt_count" 0)
        patch_prepare_failures = [double](Get-Value $row.patch "patch_prepare_failure_count" 0)
        input_tokens = [double](Get-Value $row.cost "input_tokens" 0)
        cached_input_tokens = [double](Get-Value $row.cost "cached_input_tokens" 0)
        uncached_input_tokens = [double](Get-Value $row.cost "uncached_input_tokens" 0)
        output_tokens = [double](Get-Value $row.cost "output_tokens" 0)
        wall_time_ms = [double](Get-Value $row.cost "wall_time_ms" 0)
        request_2_plus_input_tokens = $request2Input
        request_2_plus_cached_input_tokens = [double](Get-Value $row.cache "request_2_plus_cached_input_tokens" 0)
        request_2_plus_cache_hit_rate = Get-Value $row.cache "request_2_plus_hit_rate"
        cache_prefix_preserved_rate = Get-Value $row.cache "prefix_preserved_rate"
        same_shape_zero_hit_count = [double](Get-Value $row.cache "same_shape_zero_hit_count" 0)
        map_count = [double](Get-Value $row.map "map_count" 0)
        map_nodes = [double](Get-Value $row.map "node_count" 0)
        map_edges = [double](Get-Value $row.map "edge_count" 0)
        map_open_leaves = [double](Get-Value $row.map "open_leaf_nodes" 0)
        map_root_status = if ($rootNode.Count) { [string]$rootNode[0].status } else { "" }
        map_observer_root_task_status = [string](Get-Value $row.map "root_task_status" "")
        map_finish_status = if ($finishNode.Count) { [string]$finishNode[0].status } else { "" }
        first_input_tokens = [double](Get-Value $requestSummary "first_input_tokens_per_request" 0)
        tools_section_tokens_mean = [double](Get-Value $sectionSummary.estimated_tokens_mean "tools" 0)
        projection_section_tokens_mean = [double](Get-Value $sectionSummary.estimated_tokens_mean "active_projection" 0)
        tools_section_tokens_total = [double](Get-Value $sectionSummary.estimated_tokens_total "tools" 0)
        projection_section_tokens_total = [double](Get-Value $sectionSummary.estimated_tokens_total "active_projection" 0)
        natural_history_section_tokens_mean = [double](Get-Value $sectionSummary.estimated_tokens_mean "natural_history" 0)
        ordinary_feedback_section_tokens_mean = [double](Get-Value $sectionSummary.estimated_tokens_mean "ordinary_tool_feedback" 0)
        system_section_tokens_mean = [double](Get-Value $sectionSummary.estimated_tokens_mean "system_messages" 0)
        artifact_dir = [string]$row.artifact_dir
        run_dir = [string]$run.run_dir
    }
    $requestPath = if ($flat.logical_mode -eq "standard") {
        Get-R7StandardRequestPath $rolloutPath $completedProviderResponses
    } else {
        Get-R7TaskspaceRequestPath $rolloutPath $completedProviderResponses
    }
    $requestPath = @(
        Add-R7WireFactsToRequestPath `
            @($requestPath) `
            $wireTracePath `
            ([int]$flat.provider_requests)
    )
    $requestObservability = Get-R7RequestObservabilitySummary @($requestPath)
    if (-not [bool]$requestObservability.classification_reconciled) {
        throw "Request failure taxonomy does not reconcile for $($run.sample) repeat $($run.repeat) $($run.arm)"
    }
    $requestCalls = @($requestPath | ForEach-Object { @($_.calls) })
    $flat | Add-Member -NotePropertyName tool_action_requests -NotePropertyValue @($requestPath | Where-Object action_kind -eq "tool_calls").Count
    $flat | Add-Member -NotePropertyName assistant_only_requests -NotePropertyValue @($requestPath | Where-Object action_kind -eq "assistant_only").Count
    $flat | Add-Member -NotePropertyName multi_tool_requests -NotePropertyValue @($requestPath | Where-Object { @($_.calls).Count -gt 1 }).Count
    $flat | Add-Member -NotePropertyName no_failure_requests -NotePropertyValue ([int]$requestObservability.primary_failure_counts.none)
    $flat | Add-Member -NotePropertyName tool_sequence_protocol_failure_requests -NotePropertyValue ([int]$requestObservability.primary_failure_counts.tool_sequence_protocol)
    $flat | Add-Member -NotePropertyName taskspace_protocol_failure_requests -NotePropertyValue (
        [int]$requestObservability.primary_failure_counts.taskspace_protocol +
        [int]$requestObservability.primary_failure_counts.taskspace
    )
    $flat | Add-Member -NotePropertyName taskspace_state_failure_requests -NotePropertyValue ([int]$requestObservability.primary_failure_counts.taskspace_state_machine)
    $flat | Add-Member -NotePropertyName taskspace_resource_failure_requests -NotePropertyValue ([int]$requestObservability.primary_failure_counts.taskspace_resource)
    $flat | Add-Member -NotePropertyName ordinary_failure_requests -NotePropertyValue ([int]$requestObservability.primary_failure_counts.ordinary_tool)
    $flat | Add-Member -NotePropertyName sibling_failure_copy_count -NotePropertyValue ([int]$requestObservability.sibling_failure_copy_count)
    $flat | Add-Member -NotePropertyName classification_reconciled -NotePropertyValue ([bool]$requestObservability.classification_reconciled)
    $flat | Add-Member -NotePropertyName receipt_before_requests -NotePropertyValue ([int]$requestObservability.receipt_before_requests)
    $flat | Add-Member -NotePropertyName receipt_before_input_tokens -NotePropertyValue ([double]$requestObservability.receipt_before_input_tokens)
    $flat | Add-Member -NotePropertyName receipt_before_cached_input_tokens -NotePropertyValue ([double]$requestObservability.receipt_before_cached_input_tokens)
    $flat | Add-Member -NotePropertyName receipt_before_cache_hit_rate -NotePropertyValue $requestObservability.receipt_before_cache_hit_rate
    $flat | Add-Member -NotePropertyName no_receipt_before_requests -NotePropertyValue ([int]$requestObservability.no_receipt_before_requests)
    $flat | Add-Member -NotePropertyName no_receipt_before_input_tokens -NotePropertyValue ([double]$requestObservability.no_receipt_before_input_tokens)
    $flat | Add-Member -NotePropertyName no_receipt_before_cached_input_tokens -NotePropertyValue ([double]$requestObservability.no_receipt_before_cached_input_tokens)
    $flat | Add-Member -NotePropertyName no_receipt_before_cache_hit_rate -NotePropertyValue $requestObservability.no_receipt_before_cache_hit_rate
    $flat | Add-Member -NotePropertyName receipt_original_roles -NotePropertyValue (@($requestObservability.receipt_original_roles) -join ",")
    $flat | Add-Member -NotePropertyName receipt_wire_roles -NotePropertyValue (@($requestObservability.receipt_wire_roles) -join ",")
    $flat | Add-Member -NotePropertyName receipt_wire_role_unresolved_count -NotePropertyValue ([int]$requestObservability.receipt_wire_role_unresolved_count)
    $firstRequestInitialization = if ($requestPath.Count) {
        @($requestPath[0].calls | Where-Object {
                $_.tool -eq "taskspace_control" -and $_.control_action -eq "initialize_and_execute"
            }).Count
    } else {
        0
    }
    $firstRequestInitializationCommits = if ($requestPath.Count) {
        @($requestPath[0].calls | Where-Object {
                $_.tool -eq "taskspace_control" -and
                $_.control_action -eq "initialize_and_execute" -and
                $_.success -eq $true -and
                $_.state_commit -eq $true
            }).Count
    } else {
        0
    }
    $directInitializeControl = @($requestCalls | Where-Object {
            $_.tool -eq "taskspace_control" -and $_.control_action -eq "initialize_and_execute"
        }).Count
    $noTaskPathRejections = @($requestCalls | Where-Object failure_code -eq "no_task_path").Count
    $flat | Add-Member -NotePropertyName first_request_initialization -NotePropertyValue $firstRequestInitialization
    $flat | Add-Member -NotePropertyName first_request_initialization_commits -NotePropertyValue $firstRequestInitializationCommits
    $flat | Add-Member -NotePropertyName direct_initialize_control -NotePropertyValue $directInitializeControl
    $flat | Add-Member -NotePropertyName no_task_path_rejections -NotePropertyValue $noTaskPathRejections
    $soloNonterminal = 0
    foreach ($requestRow in $requestPath) {
        $calls = @($requestRow.calls)
        for ($index = 0; $index -lt $calls.Count; $index++) {
            $call = $calls[$index]
            if ($call.tool -ne "taskspace_control" -or
                $call.control_action -notin @("initialize_and_execute", "execute", "reopen_map")) {
                continue
            }
            $paired = @($calls | Where-Object {
                    $_.tool -ne "taskspace_control" -and -not [string]::IsNullOrWhiteSpace([string]$_.declared_node_id)
                }).Count -gt 0
            if (-not $paired) { $soloNonterminal++ }
        }
    }
    $echoOnlyHandoffs = 0
    $flat | Add-Member -NotePropertyName echo_only_handoffs -NotePropertyValue $echoOnlyHandoffs
    $rows.Add($flat)
    $anomalies = [Collections.Generic.List[string]]::new()
    if (-not $flat.business_success) { $anomalies.Add("scenario_failed") }
    if (@($requestCalls | Where-Object failure_class -eq "taskspace_protocol").Count) { $anomalies.Add("taskspace_protocol_failure") }
    if (@($requestCalls | Where-Object failure_class -eq "taskspace_state_machine").Count) { $anomalies.Add("taskspace_state_machine_failure") }
    if (@($requestCalls | Where-Object failure_class -eq "tool_sequence_protocol").Count) { $anomalies.Add("tool_sequence_protocol_failure") }
    if (@($requestCalls | Where-Object failure_class -eq "ordinary_tool").Count) { $anomalies.Add("ordinary_tool_failure") }
    if ($flat.same_shape_zero_hit_count -gt 0) { $anomalies.Add("same_shape_zero_cache_hit") }
    if ($echoOnlyHandoffs -gt 0) { $anomalies.Add("echo_only_lifecycle_handoff") }
    $traceRuns.Add([pscustomobject]@{
            sample = $flat.sample
            repeat = $flat.repeat
            arm = $flat.arm
            provider_requests = $flat.provider_requests
            anomalies = @($anomalies)
            solo_nonterminal_transition_count = $soloNonterminal
            echo_only_handoff_count = $echoOnlyHandoffs
            request_observability = $requestObservability
            request_path = @($requestPath)
            provider_attempts = $wireAttemptSummary
            wire_sections = $sectionSummary
            observation_json = $observationPath
            provider_wire_trace = $wireTracePath
            rollout = $rolloutPath
            whale_exec = Join-Path $flat.artifact_dir "whale-exec.jsonl"
        })
}

$uniqueImages = @($imageDigests | Sort-Object -Unique)
if ($uniqueImages.Count -ne 1) { throw "Four-arm matrix used multiple Docker image digests" }
$rowsPath = Join-Path $RunRoot "summary.csv"
$rows | Export-Csv -NoTypeInformation -Encoding UTF8 -LiteralPath $rowsPath

$aggregates = [Collections.Generic.List[object]]::new()
foreach ($group in @($rows | Group-Object sample, arm)) {
    $first = @($group.Group)[0]
    $aggregates.Add((Get-AggregateRow @($group.Group) "sample_arm" $first.sample $first.arm))
}
foreach ($group in @($rows | Group-Object arm)) {
    $first = @($group.Group)[0]
    $aggregates.Add((Get-AggregateRow @($group.Group) "all_samples_arm" "all" $first.arm))
}
$aggregatePath = Join-Path $RunRoot "aggregate.csv"
$aggregates | Export-Csv -NoTypeInformation -Encoding UTF8 -LiteralPath $aggregatePath

$traceAnalysis = [ordered]@{
    schema_version = 3
    contract_id = [string]$manifest.contract_id
    status = "initial_observation_only_no_policy_claim"
    docker_image_digest = $uniqueImages[0]
    artifact_provenance = $artifactProvenance
    run_count = $rows.Count
    runs = @($traceRuns)
}
$tracePath = Join-Path $RunRoot "trace-analysis.json"
[IO.File]::WriteAllText($tracePath, (($traceAnalysis | ConvertTo-Json -Depth 50) + "`n"), [Text.UTF8Encoding]::new($false))

$overall = @($aggregates | Where-Object scope -eq "all_samples_arm")
$lines = [Collections.Generic.List[string]]::new()
$lines.Add("# R7 五层改造后四臂首轮观测")
$lines.Add("")
$lines.Add("- 运行：$($rows.Count)/$expectedRuns")
$lines.Add("- Docker image：``$($uniqueImages[0])``")
$lines.Add("- 工件来源：``$($artifactProvenance.status)``，commit ``$($artifactProvenance.repo_commit)``，binary ``$($artifactProvenance.whale_binary_sha256)``。")
$lines.Add("- 结论边界：repeat 3 只用于发现回归和执行路径差异，不选择默认 policy。")
$lines.Add("")
$lines.Add("| arm | 成功 | request 总/均/中位 | input 总/均/中位 | cached | uncached | req2+ cache | output 总/均 | wall ms 总/均/中位 |")
$lines.Add("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
foreach ($row in $overall) {
    $cacheRate = if ($null -eq $row.request_2_plus_cache_hit_rate) { "N/A" } else { "{0:P2}" -f [double]$row.request_2_plus_cache_hit_rate }
    $lines.Add("| $($row.arm) | $($row.successes)/$($row.runs) | $($row.requests_total) / $($row.requests_mean) / $($row.requests_median) | $($row.input_tokens_total) / $($row.input_tokens_mean) / $($row.input_tokens_median) | $($row.cached_input_tokens_total) | $($row.uncached_input_tokens_total) | $cacheRate | $($row.output_tokens_total) / $($row.output_tokens_mean) | $($row.wall_time_ms_total) / $($row.wall_time_ms_mean) / $($row.wall_time_ms_median) |")
}
$lines.Add("")
$lines.Add("| arm | 工作/终答 request | 多工具 request | 无失败 request | sequence 协议失败 | TS 协议失败 | TS 状态失败 | TS 资源失败 | ordinary 失败 | sibling 复制 | 分类未对账 run | multi-patch | Map nodes/edges 均值 |")
$lines.Add("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
foreach ($row in $overall) {
    $lines.Add("| $($row.arm) | $($row.tool_action_requests_total) / $($row.assistant_only_requests_total) | $($row.multi_tool_requests_total) | $($row.no_failure_requests_total) | $($row.tool_sequence_protocol_failure_requests_total) | $($row.taskspace_protocol_failure_requests_total) | $($row.taskspace_state_failure_requests_total) | $($row.taskspace_resource_failure_requests_total) | $($row.ordinary_failure_requests_total) | $($row.sibling_failure_copy_count_total) | $($row.classification_unreconciled_runs) | $($row.multi_patch_attempts_total) | $($row.map_nodes_mean) / $($row.map_edges_mean) |")
}
$lines.Add("")
$lines.Add("| arm | receipt-before request | receipt input/cached/hit | no-receipt request | no-receipt input/cached/hit | unresolved wire role |")
$lines.Add("|---|---:|---:|---:|---:|---:|")
foreach ($row in $overall) {
    $receiptRate = if ([double]$row.receipt_before_input_tokens_total -gt 0) {
        "{0:P2}" -f ([double]$row.receipt_before_cached_input_tokens_total / [double]$row.receipt_before_input_tokens_total)
    } else { "N/A" }
    $otherRate = if ([double]$row.no_receipt_before_input_tokens_total -gt 0) {
        "{0:P2}" -f ([double]$row.no_receipt_before_cached_input_tokens_total / [double]$row.no_receipt_before_input_tokens_total)
    } else { "N/A" }
    $lines.Add("| $($row.arm) | $($row.receipt_before_requests_total) | $($row.receipt_before_input_tokens_total) / $($row.receipt_before_cached_input_tokens_total) / $receiptRate | $($row.no_receipt_before_requests_total) | $($row.no_receipt_before_input_tokens_total) / $($row.no_receipt_before_cached_input_tokens_total) / $otherRate | $($row.receipt_wire_role_unresolved_count_total) |")
}
$lines.Add("")
$lines.Add("| arm | initialize_and_execute 提交/总/失败 | 首请求初始化 尝试/提交 | 直接 control 初始化 | no_task_path |")
$lines.Add("|---|---:|---:|---:|---:|")
foreach ($row in $overall) {
    $lines.Add("| $($row.arm) | $($row.committed_initialize_and_execute_total) / $($row.initialize_and_execute_total) / $($row.failed_initialize_and_execute_total) | $($row.first_request_initialization_total) / $($row.first_request_initialization_commits_total) | $($row.direct_initialize_control_total) | $($row.no_task_path_rejections_total) |")
}
$lines.Add("")
$lines.Add("逐运行明细：``summary.csv``；分样本和全局聚合：``aggregate.csv``；trace 索引与初筛异常：``trace-analysis.json``。")
$reportPath = Join-Path $RunRoot "report.md"
$lines | Set-Content -Encoding UTF8 -LiteralPath $reportPath
Write-Output "R7FiveLayerReport: $reportPath"
Write-Output "R7FiveLayerSummary: $rowsPath"
Write-Output "R7FiveLayerAggregate: $aggregatePath"
Write-Output "R7FiveLayerTraceAnalysis: $tracePath"
Write-Output "R7FiveLayerArtifactProvenance: $artifactProvenancePath"
