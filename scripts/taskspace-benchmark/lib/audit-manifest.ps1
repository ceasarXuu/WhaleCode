function Get-TaskspaceSideAuditRecord {
    param(
        [Parameter(Mandatory = $true)]$Metrics,
        [Parameter(Mandatory = $true)][string]$PairDir
    )
    $sideName = [string]$Metrics.mode
    $artifactRoot = "$sideName/artifacts"
    [ordered]@{
        mode = $sideName
        logical_mode = [string]$Metrics.logical_mode
        success = [bool]$Metrics.business_success
        exec_exit_code = [int]$Metrics.exec_exit_code
        exec_timed_out = [bool]$Metrics.exec_timed_out
        public_validation_exit_code = [int]$Metrics.public_validation_exit_code
        hidden_oracle_exit_code = [int]$Metrics.hidden_oracle_exit_code
        wall_time_ms = [int64]$Metrics.wall_time_ms
        changed_files = @($Metrics.changed_paths)
        diff_ref = "$artifactRoot/git-diff.patch"
        validator_stdout_ref = "$artifactRoot/validation.stdout.log"
        validator_stderr_ref = "$artifactRoot/validation.stderr.log"
        metrics_ref = "$artifactRoot/metrics.json"
        cleanup_ok = (@($Metrics.validator_environment_failures | Where-Object { [string]$_ -match "cleanup" }).Count -eq 0)
        graph_ref = if ([string]$Metrics.logical_mode -eq "taskspace" -and $Metrics.PSObject.Properties.Name -contains "observability_json") { [string]$Metrics.observability_json } else { "" }
        graph_health_ref = if ($Metrics.PSObject.Properties.Name -contains "graph_health_path") { [string]$Metrics.graph_health_path } else { "" }
        result_validity_summary = [ordered]@{
            accepted = if ($Metrics.PSObject.Properties.Name -contains "accepted_results") { [int]$Metrics.accepted_results } else { 0 }
            unreviewed = if ($Metrics.PSObject.Properties.Name -contains "unreviewed_results") { [int]$Metrics.unreviewed_results } else { 0 }
            questioned_or_invalid = if ($Metrics.PSObject.Properties.Name -contains "questioned_or_invalid_results") { [int]$Metrics.questioned_or_invalid_results } else { 0 }
            adoption_metric_state = if ($Metrics.PSObject.Properties.Name -contains "result_adoption_metric_state") { [string]$Metrics.result_adoption_metric_state } else { "unknown" }
        }
        decision_summary = [ordered]@{
            decision_count = if ($Metrics.PSObject.Properties.Name -contains "decision_count") { [int]$Metrics.decision_count } else { 0 }
            decision_density = if ($Metrics.PSObject.Properties.Name -contains "decision_density") { [double]$Metrics.decision_density } else { 0.0 }
        }
        validator_environment_failures = @($Metrics.validator_environment_failures)
        metrics_taints = @($Metrics.metrics_taints)
    }
}

function Write-TaskspaceAuditYaml {
    param(
        [Parameter(Mandatory = $true)]$Audit,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("audit_version: $($Audit.audit_version)")
    $lines.Add("pair_id: $($Audit.pair_id)")
    $lines.Add("sample_name: $($Audit.sample_name)")
    $lines.Add("included_in_utility: $($Audit.classification.included_in_utility)")
    $lines.Add("utility_direction: $($Audit.classification.utility_direction)")
    $lines.Add("audit_status: $($Audit.classification.audit_status)")
    $lines.Add("exclusion_reason: $($Audit.classification.exclusion_reason)")
    $lines.Add("failure_taxonomy:")
    foreach ($class in @($Audit.classification.failure_taxonomy)) {
        $lines.Add("  - $class")
    }
    if (@($Audit.classification.failure_taxonomy).Count -eq 0) {
        $lines.Add("  []")
    }
    $lines.Add("proof:")
    $lines.Add("  oracle_isolation_ok: $($Audit.proof.oracle_isolation_ok)")
    $lines.Add("  cleanup_ok: $($Audit.proof.cleanup_ok)")
    $lines.Add("  human_review_required: $($Audit.proof.human_review_required)")
    $lines.Add("  human_review_completed: $($Audit.proof.human_review_completed)")
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Write-TaskspaceAuditManifest {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)]$ManifestResolved,
        [Parameter(Mandatory = $true)]$LeftMetrics,
        [Parameter(Mandatory = $true)]$RightMetrics,
        [Parameter(Mandatory = $true)]$Evidence,
        [Parameter(Mandatory = $true)]$VariableControl,
        $AuditReview = $null
    )
    $standardMetrics = @($LeftMetrics, $RightMetrics) | Where-Object { [string]$_.logical_mode -eq "standard" } | Select-Object -First 1
    $taskspaceMetrics = @($LeftMetrics, $RightMetrics) | Where-Object { [string]$_.logical_mode -eq "taskspace" } | Select-Object -First 1
    $failureClasses = @(Get-TaskspaceFailureTaxonomy $standardMetrics $taskspaceMetrics $Evidence $AuditReview $VariableControl)
    $direction = Get-TaskspaceUtilityDirection $standardMetrics $taskspaceMetrics $failureClasses
    $gateFailures = @(@($Evidence.evidence_gate_failures) + @($Evidence.e3_gate_failures))
    $auditStatus = if ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "completed" -and [bool]$AuditReview.completed) {
        "completed"
    } elseif ($ManifestResolved.PSObject.Properties.Name -contains "human_review_required" -and [bool]$ManifestResolved.human_review_required) {
        "required_pending"
    } else {
        "not_required"
    }
    $exclusionReason = if ($Evidence.included_in_utility_aggregate -or $Evidence.included_in_e3_aggregate) {
        ""
    } elseif (@($gateFailures).Count -gt 0) {
        @($gateFailures) -join ","
    } elseif ($failureClasses -contains "audit_unclean") {
        "audit_unclean"
    } else {
        "not_included_by_gate"
    }
    $oracleOk = -not (@($gateFailures | Where-Object { [string]$_ -match "oracle_isolation" }).Count -gt 0)
    $cleanupOk = -not (@($failureClasses | Where-Object { [string]$_ -match "environment_noise" }).Count -gt 0)
    $audit = [ordered]@{
        audit_version = "taskspace-e3-audit-v1"
        pair_id = "pair-{0:000}" -f [int]$ManifestResolved.repeat
        sample_name = [string]$ManifestResolved.scenario
        standard = Get-TaskspaceSideAuditRecord $standardMetrics $PairDir
        taskspace = Get-TaskspaceSideAuditRecord $taskspaceMetrics $PairDir
        classification = [ordered]@{
            included_in_utility = [bool]($Evidence.included_in_utility_aggregate -or $Evidence.included_in_e3_aggregate)
            exclusion_reason = $exclusionReason
            failure_taxonomy = @($failureClasses)
            utility_direction = $direction
            audit_status = $auditStatus
        }
        proof = [ordered]@{
            oracle_isolation_ok = $oracleOk
            remote_asset_ok = -not ($failureClasses -contains "remote_asset_unavailable" -or $failureClasses -contains "remote_asset_equivalence_unproven")
            cleanup_ok = $cleanupOk
            validator_equivalence_ok = -not (@($gateFailures | Where-Object { [string]$_ -match "validator.*unproven|validator.*eligible" }).Count -gt 0)
            human_review_required = if ($ManifestResolved.PSObject.Properties.Name -contains "human_review_required") { [bool]$ManifestResolved.human_review_required } else { $false }
            human_review_completed = ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "completed" -and [bool]$AuditReview.completed)
        }
        gate_failures = @($gateFailures)
        artifact_audit = [ordered]@{
            review_source_path = if ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "source_path") { [string]$AuditReview.source_path } else { "" }
            failures = if ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "failures") { @($AuditReview.failures) } else { @() }
        }
        generated_at = (Get-Date).ToString("o")
    }
    $jsonPath = Join-Path $PairDir "audit.json"
    $yamlPath = Join-Path $PairDir "audit.yaml"
    ($audit | ConvertTo-Json -Depth 30) | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    Write-TaskspaceAuditYaml $audit $yamlPath
    [pscustomobject]@{
        audit = $audit
        json_path = $jsonPath
        yaml_path = $yamlPath
        failure_taxonomy = @($failureClasses)
        utility_direction = $direction
        audit_status = $auditStatus
    }
}
