function Get-PerformanceCountIdentity {
    param(
        $Metrics,
        $Actions,
        $Map,
        $Cadence,
        $Patch,
        $Cache,
        [string]$LogicalMode,
        [bool]$Skipped
    )
    $facts = [ordered]@{
        tool_call_count = if ([string](Get-PerformanceProperty $Actions "protocol") -eq "taskspace_exec") {
            [int64](Get-PerformanceProperty $Actions "client_action_count") +
                [int64](Get-PerformanceProperty $Actions "provider_action_count")
        } else { Get-PerformanceProperty $Metrics "tool_call_count" }
        failed_tool_call_count = if ([string](Get-PerformanceProperty $Actions "protocol") -eq "taskspace_exec") {
            Get-PerformanceProperty $Actions "failed_action_count"
        } else { Get-PerformanceProperty $Metrics "failed_tool_call_count" }
        shell_calls = Get-PerformanceProperty $Actions "shell"
        patch_calls = Get-PerformanceProperty $Actions "patch"
        same_shape_zero_hit_count =
            Get-PerformanceProperty $Cache "same_shape_zero_hit_count"
    }
    if ([string](Get-PerformanceProperty $Actions "source") -eq "rollout") {
        $facts.provider_outer_tool_calls =
            Get-PerformanceProperty $Actions "provider_outer_tool_calls"
    }
    if ($LogicalMode -eq "taskspace") {
        if ([string](Get-PerformanceProperty $Actions "protocol") -eq "taskspace_exec") {
            foreach ($field in @(
                    "exec_count", "map_operation_count", "client_action_count",
                    "provider_action_count", "node_binding_count", "client_result_count",
                    "provider_result_count", "trace_event_count", "correlated_request_count",
                    "correlated_outer_call_count"
                )) {
                $facts[$field] = Get-PerformanceProperty $Actions $field
            }
            if ([string](Get-PerformanceProperty $Actions "availability") -ne "measured") {
                $facts.taskspace_exec_observation = $null
            }
        } else {
            $facts.taskspace_control_count = Get-PerformanceProperty $Map "control_count"
            $facts.initialize_and_execute_count = Get-PerformanceProperty $Map "initialize_and_execute_count"
            $facts.committed_initialize_and_execute_count = Get-PerformanceProperty $Map "committed_initialize_and_execute_count"
            $facts.failed_initialize_and_execute_count = Get-PerformanceProperty $Map "failed_initialize_and_execute_count"
            $facts.control_failure_count = Get-PerformanceProperty $Map "control_failure_count"
            $facts.control_protocol_failure_count = Get-PerformanceProperty $Map "control_protocol_failure_count"
            $facts.control_state_failure_count = Get-PerformanceProperty $Map "control_state_failure_count"
        }
        $facts.map_count = Get-PerformanceProperty $Map "map_count"
        $facts.node_count = Get-PerformanceProperty $Map "node_count"
        $facts.edge_count = Get-PerformanceProperty $Map "edge_count"
    }
    if ([string](Get-PerformanceProperty $Actions "protocol") -ne "taskspace_exec" -and
        [string](Get-PerformanceProperty $Cadence "availability") -ne "missing") {
        foreach ($field in @(
                "provider_tool_response_count",
                "control_response_count",
                "action_manifest_count",
                "action_manifest_pair_count",
                "action_manifest_violation_count"
            )) {
            $facts[$field] = Get-PerformanceProperty $Cadence $field
        }
    }
    if ([string](Get-PerformanceProperty $Patch "availability") -ne "missing") {
        $facts.request_multi_patch_attempt_count =
            Get-PerformanceProperty $Patch "request_multi_patch_attempt_count"
        $facts.patch_prepare_failure_count =
            Get-PerformanceProperty $Patch "patch_prepare_failure_count"
    }
    $values = [ordered]@{}
    $invalid = [Collections.Generic.List[string]]::new()
    foreach ($field in $facts.Keys) {
        $values[$field] = ConvertTo-R7NonnegativeInt64Fact $facts[$field]
        if (-not $Skipped -and $null -eq $values[$field]) {
            $invalid.Add($field)
        }
    }
    [pscustomobject]@{
        valid = $Skipped -or $invalid.Count -eq 0
        invalid_fields = @($invalid)
        values = [pscustomobject]$values
    }
}
