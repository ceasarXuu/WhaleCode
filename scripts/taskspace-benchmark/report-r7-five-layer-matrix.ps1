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
    $requestTotal = Get-R7ExactPropertyInt64Sum $Rows "provider_requests" "matrix aggregate"
    $inputTotal = Get-R7ExactInt64Sum @($Rows.input_tokens) "input_tokens"
    $cachedTotal = Get-R7ExactInt64Sum @($Rows.cached_input_tokens) "cached_input_tokens"
    $uncachedTotal = Get-R7ExactInt64Sum @($Rows.uncached_input_tokens) "uncached_input_tokens"
    $outputTotal = Get-R7ExactInt64Sum @($Rows.output_tokens) "output_tokens"
    $reasoningTotal = Get-R7ExactInt64Sum @($Rows.reasoning_output_tokens) "reasoning_output_tokens"
    $tokenTotal = Get-R7ExactInt64Sum @($Rows.total_tokens) "total_tokens"
    $wallTotal = ($Rows | Measure-Object -Property wall_time_ms -Sum).Sum
    $request2Input = Get-R7ExactInt64Sum @($Rows.request_2_plus_input_tokens) "request_2_plus_input_tokens"
    $request2Cached = Get-R7ExactInt64Sum @($Rows.request_2_plus_cached_input_tokens) "request_2_plus_cached_input_tokens"
    [pscustomobject]@{
        scope = $Scope
        sample = $Sample
        arm = $Arm
        runs = [int64]$Rows.Count
        successes = [int64]@($Rows | Where-Object { [bool]$_.business_success }).Count
        requests_total = $requestTotal
        requests_mean = if ($Rows.Count) { [Math]::Round($requestTotal / $Rows.Count, 3) } else { $null }
        requests_median = Get-Median @($Rows.provider_requests)
        completed_provider_responses_total = Get-R7ExactPropertyInt64Sum $Rows "completed_provider_responses" "matrix aggregate"
        failed_or_cancelled_provider_attempts_total = Get-R7ExactPropertyInt64Sum $Rows "failed_or_cancelled_provider_attempts" "matrix aggregate"
        retried_logical_requests_total = Get-R7ExactPropertyInt64Sum $Rows "retried_logical_requests" "matrix aggregate"
        tool_action_requests_total = Get-R7ExactPropertyInt64Sum $Rows "tool_action_requests" "matrix aggregate"
        assistant_only_requests_total = Get-R7ExactPropertyInt64Sum $Rows "assistant_only_requests" "matrix aggregate"
        multi_tool_requests_total = Get-R7ExactPropertyInt64Sum $Rows "multi_tool_requests" "matrix aggregate"
        no_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "no_failure_requests" "matrix aggregate"
        tool_sequence_protocol_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "tool_sequence_protocol_failure_requests" "matrix aggregate"
        taskspace_protocol_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "taskspace_protocol_failure_requests" "matrix aggregate"
        taskspace_state_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "taskspace_state_failure_requests" "matrix aggregate"
        taskspace_resource_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "taskspace_resource_failure_requests" "matrix aggregate"
        ordinary_failure_requests_total = Get-R7ExactPropertyInt64Sum $Rows "ordinary_failure_requests" "matrix aggregate"
        sibling_failure_copy_count_total = Get-R7ExactPropertyInt64Sum $Rows "sibling_failure_copy_count" "matrix aggregate"
        classification_unreconciled_runs = [int64]@(
            $Rows | Where-Object classification_reconciled -ne $true
        ).Count
        echo_only_handoffs_total = Get-R7ExactPropertyInt64Sum $Rows "echo_only_handoffs" "matrix aggregate"
        initialize_and_execute_total = Get-R7ExactPropertyInt64Sum $Rows "initialize_and_execute" "matrix aggregate"
        committed_initialize_and_execute_total = Get-R7ExactPropertyInt64Sum $Rows "committed_initialize_and_execute" "matrix aggregate"
        failed_initialize_and_execute_total = Get-R7ExactPropertyInt64Sum $Rows "failed_initialize_and_execute" "matrix aggregate"
        first_request_initialization_total = Get-R7ExactPropertyInt64Sum $Rows "first_request_initialization" "matrix aggregate"
        first_request_initialization_commits_total = Get-R7ExactPropertyInt64Sum $Rows "first_request_initialization_commits" "matrix aggregate"
        direct_initialize_control_total = Get-R7ExactPropertyInt64Sum $Rows "direct_initialize_control" "matrix aggregate"
        no_task_path_rejections_total = Get-R7ExactPropertyInt64Sum $Rows "no_task_path_rejections" "matrix aggregate"
        input_tokens_total = $inputTotal
        input_tokens_mean = if ($Rows.Count) { [Math]::Round($inputTotal / $Rows.Count, 3) } else { $null }
        input_tokens_median = Get-Median @($Rows.input_tokens)
        cached_input_tokens_total = $cachedTotal
        uncached_input_tokens_total = $uncachedTotal
        output_tokens_total = $outputTotal
        output_tokens_mean = if ($Rows.Count) { [Math]::Round($outputTotal / $Rows.Count, 3) } else { $null }
        output_tokens_median = Get-Median @($Rows.output_tokens)
        reasoning_output_tokens_total = $reasoningTotal
        reasoning_output_tokens_mean = if ($Rows.Count) { [Math]::Round($reasoningTotal / $Rows.Count, 3) } else { $null }
        total_tokens_total = $tokenTotal
        total_tokens_mean = if ($Rows.Count) { [Math]::Round($tokenTotal / $Rows.Count, 3) } else { $null }
        request_2_plus_cache_hit_rate = if ($request2Input -gt 0) { [Math]::Round($request2Cached / $request2Input, 6) } else { $null }
        receipt_before_requests_total = Get-R7ExactPropertyInt64Sum $Rows "receipt_before_requests" "matrix aggregate"
        receipt_before_input_tokens_total = Get-R7ExactPropertyInt64Sum $Rows "receipt_before_input_tokens" "matrix aggregate"
        receipt_before_cached_input_tokens_total = Get-R7ExactPropertyInt64Sum $Rows "receipt_before_cached_input_tokens" "matrix aggregate"
        no_receipt_before_requests_total = Get-R7ExactPropertyInt64Sum $Rows "no_receipt_before_requests" "matrix aggregate"
        no_receipt_before_input_tokens_total = Get-R7ExactPropertyInt64Sum $Rows "no_receipt_before_input_tokens" "matrix aggregate"
        no_receipt_before_cached_input_tokens_total = Get-R7ExactPropertyInt64Sum $Rows "no_receipt_before_cached_input_tokens" "matrix aggregate"
        receipt_wire_role_unresolved_count_total = Get-R7ExactPropertyInt64Sum $Rows "receipt_wire_role_unresolved_count" "matrix aggregate"
        wall_time_ms_total = $wallTotal
        wall_time_ms_mean = if ($Rows.Count) { [Math]::Round($wallTotal / $Rows.Count, 3) } else { $null }
        wall_time_ms_median = Get-Median @($Rows.wall_time_ms)
        tool_calls_total = Get-R7ExactPropertyInt64Sum $Rows "ordinary_tools" "matrix aggregate"
        failed_tools_total = Get-R7ExactPropertyInt64Sum $Rows "failed_tools" "matrix aggregate"
        taskspace_control_total = Get-R7ExactPropertyInt64Sum $Rows "taskspace_control" "matrix aggregate"
        control_failures_total = Get-R7ExactPropertyInt64Sum $Rows "control_failures" "matrix aggregate"
        control_protocol_failures_total = Get-R7ExactPropertyInt64Sum $Rows "control_protocol_failures" "matrix aggregate"
        control_state_failures_total = Get-R7ExactPropertyInt64Sum $Rows "control_state_failures" "matrix aggregate"
        multi_patch_attempts_total = Get-R7ExactPropertyInt64Sum $Rows "multi_patch_attempts" "matrix aggregate"
        patch_prepare_failures_total = Get-R7ExactPropertyInt64Sum $Rows "patch_prepare_failures" "matrix aggregate"
        map_nodes_mean = [Math]::Round((($Rows | Measure-Object -Property map_nodes -Average).Average), 3)
        map_edges_mean = [Math]::Round((($Rows | Measure-Object -Property map_edges -Average).Average), 3)
        first_input_tokens_mean = [Math]::Round((($Rows | Measure-Object -Property first_input_tokens -Average).Average), 3)
        tools_section_tokens_mean = if ($requestTotal) { [Math]::Round((($Rows | Measure-Object -Property tools_section_tokens_total -Sum).Sum) / $requestTotal, 3) } else { 0 }
        projection_section_tokens_mean = if ($requestTotal) { [Math]::Round((($Rows | Measure-Object -Property projection_section_tokens_total -Sum).Sum) / $requestTotal, 3) } else { 0 }
    }
}

if ([string]$manifest.status -ne "completed") { throw "Matrix run is not completed: $($manifest.status)" }
$repeatCount = Get-R7RequiredNonnegativeInt64Fact `
    $manifest "repeats_per_arm_per_sample" "matrix manifest"
[bigint]$expectedRunsBig = [bigint]$repeatCount * @($manifest.samples).Count * 4
if ($expectedRunsBig -gt [int64]::MaxValue) {
    throw "Matrix expected run count exceeds Int64"
}
$expectedRuns = [int64]$expectedRunsBig
$completedRuns = Get-R7RequiredNonnegativeInt64Fact $manifest "completed_run_count" "matrix manifest"
if ($completedRuns -ne $expectedRuns) { throw "Matrix run count is incomplete" }
$artifactProvenancePath = Join-Path $RunRoot "artifact-provenance.json"
$inputProvenance = Get-R7MatrixArtifactProvenance `
    $repoRoot `
    $manifestPath `
    $manifest `
    $PSCommandPath
if ([string]$inputProvenance.status -ne "valid") {
    [IO.File]::WriteAllText(
        $artifactProvenancePath,
        (($inputProvenance | ConvertTo-Json -Depth 100) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    $codes = @($inputProvenance.findings | ForEach-Object { [string]$_.code } | Sort-Object -Unique) -join ","
    throw "Matrix artifact provenance is invalid: $codes"
}

$rows = [Collections.Generic.List[object]]::new()
$traceRuns = [Collections.Generic.List[object]]::new()
$imageDigests = [Collections.Generic.List[string]]::new()
foreach ($run in @($manifest.runs)) {
    $observationPath = Join-Path ([string]$run.run_dir) "performance-observation.json"
    $observation = Get-Content -Raw -Encoding UTF8 -LiteralPath $observationPath | ConvertFrom-Json -Depth 100
    $actualRows = @($observation.rows | Where-Object {
            [string]$_.observation_status -in @("complete", "incomplete") -and
            [string]$_.logical_mode -eq [string]$run.logical_mode
        })
    if ($actualRows.Count -ne 1) {
        throw "Expected one observed row for $($run.sample) repeat $($run.repeat) $($run.arm)"
    }
    $row = $actualRows[0]
    $request2Input = Get-R7ExactInt64Sum @(
        (Get-Value $row.cache "request_2_plus_cached_input_tokens"),
        (Get-Value $row.cache "request_2_plus_uncached_input_tokens")
    ) "request_2_plus_input_tokens"
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
    $completedProviderResponses = Get-R7RequiredNonnegativeInt64Fact `
        $requestSummary "model_request_count" "request summary"
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
        comparison_eligible = [bool]$row.comparison_eligible
        business_success = [bool]$row.result.business_success
        agent_completion_status = [string]$row.result.agent_completion_status
        provider_requests = Get-R7RequiredNonnegativeInt64Fact $row.actions "provider_requests" "performance observation"
        completed_provider_responses = $completedProviderResponses
        failed_or_cancelled_provider_attempts = Get-R7RequiredNonnegativeInt64Fact $wireAttemptSummary "failed_or_cancelled_attempt_count" "wire attempt summary"
        retried_logical_requests = Get-R7RequiredNonnegativeInt64Fact $wireAttemptSummary "retried_logical_request_count" "wire attempt summary"
        ordinary_tools = Get-R7RequiredNonnegativeInt64Fact $row.actions "ordinary_tools" "performance observation"
        failed_tools = Get-R7RequiredNonnegativeInt64Fact $row.actions "failed_tools" "performance observation"
        taskspace_control = Get-R7RequiredNonnegativeInt64Fact $row.actions "taskspace_control" "performance observation"
        initialize_and_execute = Get-R7RequiredNonnegativeInt64Fact $row.actions "initialize_and_execute" "performance observation"
        committed_initialize_and_execute = Get-R7RequiredNonnegativeInt64Fact $row.actions "committed_initialize_and_execute" "performance observation"
        failed_initialize_and_execute = Get-R7RequiredNonnegativeInt64Fact $row.actions "failed_initialize_and_execute" "performance observation"
        control_failures = Get-R7RequiredNonnegativeInt64Fact $row.actions "control_failures" "performance observation"
        control_protocol_failures = Get-R7RequiredNonnegativeInt64Fact $row.actions "control_protocol_failures" "performance observation"
        control_state_failures = Get-R7RequiredNonnegativeInt64Fact $row.actions "control_state_failures" "performance observation"
        multi_patch_attempts = Get-R7RequiredNonnegativeInt64Fact $row.patch "request_multi_patch_attempt_count" "performance observation"
        patch_prepare_failures = Get-R7RequiredNonnegativeInt64Fact $row.patch "patch_prepare_failure_count" "performance observation"
        input_tokens = Get-R7ExactInt64Sum @((Get-Value $row.cost "input_tokens")) "input_tokens"
        cached_input_tokens = Get-R7ExactInt64Sum @((Get-Value $row.cost "cached_input_tokens")) "cached_input_tokens"
        uncached_input_tokens = Get-R7ExactInt64Sum @((Get-Value $row.cost "uncached_input_tokens")) "uncached_input_tokens"
        output_tokens = Get-R7ExactInt64Sum @((Get-Value $row.cost "output_tokens")) "output_tokens"
        wall_time_ms = [double](Get-Value $row.cost "wall_time_ms" 0)
        request_2_plus_input_tokens = $request2Input
        request_2_plus_cached_input_tokens = Get-R7ExactInt64Sum @((Get-Value $row.cache "request_2_plus_cached_input_tokens")) "request_2_plus_cached_input_tokens"
        request_2_plus_cache_hit_rate = Get-Value $row.cache "request_2_plus_hit_rate"
        cache_prefix_preserved_rate = Get-Value $row.cache "prefix_preserved_rate"
        same_shape_zero_hit_count = Get-R7RequiredNonnegativeInt64Fact $row.cache "same_shape_zero_hit_count" "performance observation"
        map_count = Get-R7OptionalNonnegativeInt64Fact $row.map "map_count" "performance observation"
        map_nodes = Get-R7OptionalNonnegativeInt64Fact $row.map "node_count" "performance observation"
        map_edges = Get-R7OptionalNonnegativeInt64Fact $row.map "edge_count" "performance observation"
        map_open_leaves = Get-R7OptionalNonnegativeInt64Fact $row.map "open_leaf_nodes" "performance observation"
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
    $requestInputTokens = Get-R7ExactInt64Sum @(
        $requestObservability.receipt_before_input_tokens,
        $requestObservability.no_receipt_before_input_tokens
    ) "input_tokens"
    $requestCachedTokens = Get-R7ExactInt64Sum @(
        $requestObservability.receipt_before_cached_input_tokens,
        $requestObservability.no_receipt_before_cached_input_tokens
    ) "cached_input_tokens"
    if ($requestInputTokens -ne [int64]$flat.input_tokens -or
        $requestCachedTokens -ne [int64]$flat.cached_input_tokens -or
        [int64]$requestObservability.output_tokens -ne [int64]$flat.output_tokens) {
        throw "Request token facts do not reconcile with the run aggregate for $($run.sample) repeat $($run.repeat) $($run.arm)"
    }
    $flat | Add-Member -NotePropertyName reasoning_output_tokens -NotePropertyValue (
        [int64]$requestObservability.reasoning_output_tokens
    )
    $flat | Add-Member -NotePropertyName total_tokens -NotePropertyValue (
        [int64]$requestObservability.total_tokens
    )
    $requestCalls = @($requestPath | ForEach-Object { @($_.calls) })
    $flat | Add-Member -NotePropertyName tool_action_requests -NotePropertyValue ([int64]@($requestPath | Where-Object action_kind -eq "tool_calls").Count)
    $flat | Add-Member -NotePropertyName assistant_only_requests -NotePropertyValue ([int64]@($requestPath | Where-Object action_kind -eq "assistant_only").Count)
    $flat | Add-Member -NotePropertyName multi_tool_requests -NotePropertyValue ([int64]@($requestPath | Where-Object { @($_.calls).Count -gt 1 }).Count)
    $flat | Add-Member -NotePropertyName no_failure_requests -NotePropertyValue ([int64]$requestObservability.primary_failure_counts.none)
    $flat | Add-Member -NotePropertyName tool_sequence_protocol_failure_requests -NotePropertyValue ([int64]$requestObservability.primary_failure_counts.tool_sequence_protocol)
    $taskspaceProtocolRequests = Get-R7ExactInt64Sum @($requestObservability.primary_failure_counts.taskspace_protocol, $requestObservability.primary_failure_counts.taskspace) "taskspace_protocol_failure_requests"
    $flat | Add-Member -NotePropertyName taskspace_protocol_failure_requests -NotePropertyValue $taskspaceProtocolRequests
    $flat | Add-Member -NotePropertyName taskspace_state_failure_requests -NotePropertyValue ([int64]$requestObservability.primary_failure_counts.taskspace_state_machine)
    $flat | Add-Member -NotePropertyName taskspace_resource_failure_requests -NotePropertyValue ([int64]$requestObservability.primary_failure_counts.taskspace_resource)
    $flat | Add-Member -NotePropertyName ordinary_failure_requests -NotePropertyValue ([int64]$requestObservability.primary_failure_counts.ordinary_tool)
    $flat | Add-Member -NotePropertyName sibling_failure_copy_count -NotePropertyValue ([int64]$requestObservability.sibling_failure_copy_count)
    $flat | Add-Member -NotePropertyName classification_reconciled -NotePropertyValue ([bool]$requestObservability.classification_reconciled)
    $flat | Add-Member -NotePropertyName receipt_before_requests -NotePropertyValue ([int64]$requestObservability.receipt_before_requests)
    $flat | Add-Member -NotePropertyName receipt_before_input_tokens -NotePropertyValue ([int64]$requestObservability.receipt_before_input_tokens)
    $flat | Add-Member -NotePropertyName receipt_before_cached_input_tokens -NotePropertyValue ([int64]$requestObservability.receipt_before_cached_input_tokens)
    $flat | Add-Member -NotePropertyName receipt_before_cache_hit_rate -NotePropertyValue $requestObservability.receipt_before_cache_hit_rate
    $flat | Add-Member -NotePropertyName no_receipt_before_requests -NotePropertyValue ([int64]$requestObservability.no_receipt_before_requests)
    $flat | Add-Member -NotePropertyName no_receipt_before_input_tokens -NotePropertyValue ([int64]$requestObservability.no_receipt_before_input_tokens)
    $flat | Add-Member -NotePropertyName no_receipt_before_cached_input_tokens -NotePropertyValue ([int64]$requestObservability.no_receipt_before_cached_input_tokens)
    $flat | Add-Member -NotePropertyName no_receipt_before_cache_hit_rate -NotePropertyValue $requestObservability.no_receipt_before_cache_hit_rate
    $flat | Add-Member -NotePropertyName receipt_original_roles -NotePropertyValue (@($requestObservability.receipt_original_roles) -join ",")
    $flat | Add-Member -NotePropertyName receipt_wire_roles -NotePropertyValue (@($requestObservability.receipt_wire_roles) -join ",")
    $flat | Add-Member -NotePropertyName receipt_wire_role_unresolved_count -NotePropertyValue ([int64]$requestObservability.receipt_wire_role_unresolved_count)
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
    $noTaskPathRejections = [int64]@(
        $requestCalls | Where-Object failure_code -eq "no_task_path"
    ).Count
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

$nodeStateRejections = Get-R7NodeStateRejectionSummary @($traceRuns)
$traceAnalysis = [ordered]@{
    schema_version = 4
    contract_id = [string]$manifest.contract_id
    status = "initial_observation_only_no_policy_claim"
    docker_image_digest = $uniqueImages[0]
    input_artifact_provenance = $inputProvenance
    run_count = [int64]$rows.Count
    node_state_rejections = $nodeStateRejections
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
$lines.Add("- 输入工件来源：``$($inputProvenance.status)``，commit ``$($inputProvenance.repo_commit)``，binary ``$($inputProvenance.whale_binary_sha256)``。")
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
$lines.Add("| arm | node_state_invalid 请求 | 违规事实 | 状态对（违规数） | 下一请求 read_map |")
$lines.Add("|---|---:|---:|---|---:|")
foreach ($row in @($nodeStateRejections.by_arm)) {
    $pairs = @(
        $row.state_pairs |
            ForEach-Object {
                "$($_.canonical_state)->$($_.candidate_state):$($_.violation_count)"
            }
    ) -join ", "
    $lines.Add("| $($row.arm) | $($row.request_count) | $($row.violation_count) | $pairs | $($row.next_read_map_request_count) |")
}
$lines.Add("")
$lines.Add("逐运行明细：``summary.csv``；分样本和全局聚合：``aggregate.csv``；trace 索引与初筛异常：``trace-analysis.json``。")
$reportPath = Join-Path $RunRoot "report.md"
$lines | Set-Content -Encoding UTF8 -LiteralPath $reportPath

$matrixStatusPath = Join-Path $RunRoot "matrix-final-status.json"
$matrixOutputs = @(
    $rowsPath,
    $aggregatePath,
    $tracePath,
    $reportPath
) | ForEach-Object {
    $item = Get-Item -LiteralPath $_
    [pscustomobject]@{
        path = $item.FullName
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash.ToLowerInvariant()
        bytes = [int64]$item.Length
    }
}
$matrixStatus = [ordered]@{
    schema_version = 1
    status = "finalized"
    final_aggregate_ready = $true
    repo_commit = [string]$manifest.repo_commit
    run_count = [int64]$rows.Count
    inputs = @(
        (New-R7ProvenanceFileFact $manifestPath "run_manifest"),
        (New-R7ProvenanceFileFact $inputProvenance.evaluation_contract_path "evaluation_contract")
    )
    outputs = @($matrixOutputs)
    finalized_at = (Get-Date).ToString("o")
}
[IO.File]::WriteAllText(
    $matrixStatusPath,
    (($matrixStatus | ConvertTo-Json -Depth 20) + "`n"),
    [Text.UTF8Encoding]::new($false)
)

$artifactProvenance = Get-R7MatrixArtifactProvenance `
    $repoRoot `
    $manifestPath `
    $manifest `
    $PSCommandPath `
    $matrixStatusPath
[IO.File]::WriteAllText(
    $artifactProvenancePath,
    (($artifactProvenance | ConvertTo-Json -Depth 100) + "`n"),
    [Text.UTF8Encoding]::new($false)
)
if ([string]$artifactProvenance.status -ne "valid") {
    $codes = @(
        $artifactProvenance.findings |
            ForEach-Object { [string]$_.code } |
            Sort-Object -Unique
    ) -join ","
    throw "Matrix final artifact provenance is invalid: $codes"
}
Write-Output "R7FiveLayerReport: $reportPath"
Write-Output "R7FiveLayerSummary: $rowsPath"
Write-Output "R7FiveLayerAggregate: $aggregatePath"
Write-Output "R7FiveLayerTraceAnalysis: $tracePath"
Write-Output "R7FiveLayerMatrixFinalStatus: $matrixStatusPath"
Write-Output "R7FiveLayerArtifactProvenance: $artifactProvenancePath"
