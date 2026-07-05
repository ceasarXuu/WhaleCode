$ErrorActionPreference = "Stop"

function Test-TaskspaceRunSideSelected {
    param([Parameter(Mandatory = $true)][string]$SideName, [Parameter(Mandatory = $true)][string]$RunSide)
    $RunSide -eq "both" -or $SideName -eq $RunSide
}

function New-TaskspaceSideSelectionSkipMetrics {
    param(
        [Parameter(Mandatory = $true)]$Side,
        [Parameter(Mandatory = $true)][string]$RunSide,
        [Parameter(Mandatory = $true)][datetime]$Timestamp
    )
    New-Item -ItemType Directory -Path $Side.ArtifactDir -Force | Out-Null
    $jsonlPath = Join-Path $Side.ArtifactDir "whale-exec.jsonl"
    $stderrPath = Join-Path $Side.ArtifactDir "whale-exec.stderr.log"
    $lastMessagePath = Join-Path $Side.ArtifactDir "last-message.md"
    $diffPath = Join-Path $Side.ArtifactDir "git-diff.patch"
    $validationStdout = Join-Path $Side.ArtifactDir "validation.stdout.log"
    $validationStderr = Join-Path $Side.ArtifactDir "validation.stderr.log"
    $oracleStdout = Join-Path $Side.ArtifactDir "hidden-oracle.stdout.log"
    $oracleStderr = Join-Path $Side.ArtifactDir "hidden-oracle.stderr.log"
    $skipPath = Join-Path $Side.ArtifactDir "side-selection-skip.json"
    $taint = "side_selection_skipped:$($Side.Name):run_side=$RunSide"
    Write-Text $jsonlPath ""
    Write-Text $stderrPath "side_selection_skipped=true`nrun_side=$RunSide`nside=$($Side.Name)`n"
    Write-Text $lastMessagePath "Skipped by -RunSide $RunSide."
    Write-Text $diffPath ""
    Write-Text $validationStdout "public_validation_skipped=true`npublic_validation_skip_reason=side_selection`n"
    Write-Text $validationStderr ""
    Write-Text $oracleStdout "hidden_oracle_skipped=true`nhidden_oracle_skip_reason=side_selection`n"
    Write-Text $oracleStderr ""
    Write-TaskspaceJson ([pscustomobject]@{
            schema_version = 1
            skipped = $true
            reason = "side_selection"
            run_side = $RunSide
            side = [string]$Side.Name
            logical_mode = [string]$Side.LogicalMode
            generated_at = $Timestamp.ToString("o")
        }) $skipPath
    [pscustomobject]@{
        mode = [string]$Side.Name
        logical_mode = [string]$Side.LogicalMode
        exec_exit_code = 0
        exec_timed_out = $false
        public_validation_exit_code = 0
        hidden_oracle_exit_code = 0
        wall_time_ms = 0
        tool_call_count = 0
        failed_tool_call_count = 0
        rollout_tool_call_count = 0
        rollout_failed_tool_call_count = 0
        rollout_control_tool_call_count = 0
        rollout_tool_call_availability = "skipped"
        observability_tool_call_count = 0
        observability_failed_tool_call_count = 0
        observability_tool_call_availability = "skipped"
        token_summary_path = ""
        cost_scan_policy_path = ""
        rollout_scan_mode = "skipped"
        rollout_bytes = 0
        rollout_scan_max_bytes = 0
        request_summary_path = ""
        provider_input_visibility_path = ""
        taskspace_control_usage_path = ""
        context_projection_summary_path = ""
        projection_events_path = ""
        token_summary_availability = "skipped"
        jsonl_bytes = 0
        provider_input_tokens_per_jsonl_kb = 0
        provider_total_tokens_per_jsonl_kb = 0
        model_request_count = 0
        input_tokens = 0
        output_tokens = 0
        cached_input_tokens = 0
        uncached_input_tokens = 0
        avg_input_tokens_per_request = 0
        avg_output_tokens_per_request = 0
        max_input_tokens_per_request = 0
        p95_input_tokens_per_request = 0
        first_input_tokens_per_request = 0
        last_input_tokens_per_request = 0
        max_output_tokens_per_request = 0
        p95_output_tokens_per_request = 0
        first_output_tokens_per_request = 0
        last_output_tokens_per_request = 0
        rollout_trace_request_availability = "skipped"
        rollout_trace_model_request_count = 0
        rollout_trace_input_tokens = 0
        rollout_trace_output_tokens = 0
        rollout_trace_max_input_tokens_per_request = 0
        rollout_trace_p95_input_tokens_per_request = 0
        rollout_trace_first_input_tokens_per_request = 0
        rollout_trace_last_input_tokens_per_request = 0
        taskspace_control_count = 0
        native_taskspace_control_count = 0
        action_contract_taskspace_control_count = 0
        state_commit_count = 0
        runtime_state_commit_count = 0
        runtime_output_ref_created_count = 0
        runtime_output_ref_slice_read_count = 0
        taskspace_runtime_event_count = 0
        active_sentinel_warning_count = 0
        active_sentinel_warning_types = @()
        context_projection_availability = "skipped"
        projection_count = 0
        projection_tokens = 0
        projection_tokens_max = 0
        projection_protected_miss_count = 0
        large_output_replay_count = 0
        largest_tool_output_bytes = 0
        raw_output_in_prompt_violation = $false
        changed_file_inventory = @()
        changed_paths = @()
        metrics_warnings = @()
        metrics_taints = @($taint)
        docker_build_result_path = ""
        docker_cache_enabled = $false
        docker_cache_eligible = $false
        docker_cache_hit = $false
        docker_cache_bypass_reason = "side_selection"
        docker_cache_key = ""
        docker_cache_image = ""
        docker_cache_lock_wait_ms = $null
        docker_cache_manifest_path = ""
        dockerfile_from_images = @()
        validation_cleanup_result_path = ""
        validator_environment_failures = @()
        validator_environment_mismatch = $false
        validator_probe_result_path = ""
        validator_probe_status = "skipped"
        tests_started_seen = $false
        tests_completed_seen = $false
        validation_lifecycle_stage = "skipped"
        validation_timeout_phase = ""
        tests_started_at = ""
        tests_completed_at = ""
        public_validation_reached_tests = $false
        pretest_failure = $false
        infra_signature = $null
        business_success = $false
        invalid_prompt = $false
        invalid_pair = $false
        harness_failure = $false
        diff_path = $diffPath
        jsonl_path = $jsonlPath
        last_message_path = $lastMessagePath
        stderr_path = $stderrPath
        validation_stdout_path = $validationStdout
        validation_stderr_path = $validationStderr
        oracle_stdout_path = $oracleStdout
        oracle_stderr_path = $oracleStderr
        oracle_isolation_level = "side_selection_skipped"
        maps = 0
        nodes = 0
        edges = 0
        edge_order_violations = 0
        spawn_agent_calls = 0
        subagent_results = 0
        open_leaf_nodes = 0
        ordinary_before_binding = $false
        accepted_results = 0
        unreviewed_results = 0
        questioned_or_invalid_results = 0
        result_adoption_metric_state = "skipped"
        decision_count = 0
        decision_density = 0.0
        graph_health_path = ""
        observability_json = ""
        public_validation_skipped = $true
        public_validation_skip_reason = "side_selection"
        pre_agent_validator_probe_status = "skipped"
        pre_agent_validator_probe_hash = ""
        model_queue_wait_ms = 0
        model_retry_backoff_ms = 0
        model_request_duration_ms = 0
        model_timing_event_count = 0
        model_timing_source_status = "skipped"
        model_timing_source_path = ""
        model_timing_parse_errors = @()
        process_launch_wait_ms = 0
        map_management_summary_path = ""
        compaction_events_path = ""
        map_management_availability = "skipped"
        map_retention_coverage_ratio = 0
        map_salience_coverage_ratio = 0
        map_protected_miss_count = 0
        map_archived_item_count = 0
        map_audit_only_item_count = 0
        map_semantic_replacement_rate = 0
        map_compaction_event_count = 0
    }
}
