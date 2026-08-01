$ErrorActionPreference = "Stop"

function Get-CacheRegressionFormalGateCommand {
    param([Parameter(Mandatory = $true)][string]$StructuredEvidencePath)
    "python3 scripts\cache-regression\check_cache_regression_gate.py --source head --require-live-baseline --require-clean-subject --json-output '$StructuredEvidencePath'"
}

function Test-CacheRegressionFormalGateEvidence {
    param(
        [Parameter(Mandatory = $true)]$Gate,
        [Parameter(Mandatory = $true)][string]$ExpectedHead
    )
    if ([string]$Gate.producer -ne "build-v005-non-agent-gates.ps1" `
        -or [string]$Gate.evidence_kind -ne "cache_regression_gate_v1" `
        -or [string]::IsNullOrWhiteSpace([string]$Gate.structured_evidence_path) `
        -or [string]::IsNullOrWhiteSpace([string]$Gate.structured_evidence_sha256)) {
        return $false
    }
    $path = [string]$Gate.structured_evidence_path
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $false }
    $expectedCommand = Get-CacheRegressionFormalGateCommand $path
    if ([string]$Gate.command -ne $expectedCommand) { return $false }
    $actualSha = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualSha -ne ([string]$Gate.structured_evidence_sha256).ToLowerInvariant()) {
        return $false
    }
    try {
        $report = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    } catch {
        return $false
    }
    $emptyCollections = @(
        @($report.policy_changes).Count,
        @($report.semantic_baseline_changes).Count,
        @($report.validation_input_mismatches).Count,
        @($report.release_relevant_changes).Count,
        @($report.sensitive_changes).Count
    )
    if (@($emptyCollections | Where-Object { [int]$_ -ne 0 }).Count -gt 0) {
        return $false
    }
    if ([string]$report.schema_version -ne "whalecode-cache-regression-gate-v1" `
        -or [string]$report.status -ne "pass" `
        -or [string]$report.source -ne "head" `
        -or [string]$report.subject_commit -ne $ExpectedHead `
        -or [string]$report.baseline_status -ne "accepted" `
        -or -not [bool]$report.accepted_baseline_validation.valid `
        -or -not [bool]$report.require_live_baseline `
        -or -not [bool]$report.require_clean_subject `
        -or -not [bool]$report.contract_matches_worktree `
        -or -not [bool]$report.relevant_source_matches_worktree `
        -or [bool]$report.policy_baseline_conflict `
        -or [bool]$report.policy_product_conflict `
        -or [bool]$report.baseline_product_conflict `
        -or [bool]$report.baseline_changed `
        -or [bool]$report.candidate_transition `
        -or [string]$report.discovery_state -ne "unchanged" `
        -or -not [bool]$report.free_validation_required `
        -or -not $report.free_validation `
        -or -not [bool]$report.free_validation.passed) {
        return $false
    }
    $commands = @($report.free_validation.commands)
    if ($commands.Count -eq 0) { return $false }
    foreach ($command in $commands) {
        if ([string]$command.status -ne "pass" `
            -or [int]$command.exit_code -ne 0 `
            -or [bool]$command.timed_out) {
            return $false
        }
        if ($command.change_report -and [string]$command.change_report.status -ne "unchanged") {
            return $false
        }
    }
    return $true
}
