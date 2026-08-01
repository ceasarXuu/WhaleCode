$ErrorActionPreference = "Stop"

function New-TaskspaceCacheRegressionFixtureGate {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Head,
        [Parameter(Mandatory = $true)][string]$TaskListHash,
        [Parameter(Mandatory = $true)][string]$SourceVersion,
        [Parameter(Mandatory = $true)][string]$ProfileHash,
        [Parameter(Mandatory = $true)][string]$GeneratedAt
    )
    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    $textPath = Join-Path $Root "cache_regression_surface.txt"
    $reportPath = Join-Path $Root "cache-regression-surface.json"
    "cache regression fixture pass" | Set-Content -LiteralPath $textPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = "whalecode-cache-regression-gate-v1"
        status = "pass"
        source = "head"
        subject_commit = $Head
        actual_surface_sha256 = "fixture-surface"
        expected_surface_sha256 = "fixture-surface"
        baseline_status = "accepted"
        accepted_baseline_validation = [pscustomobject]@{ valid = $true }
        require_live_baseline = $true
        require_clean_subject = $true
        contract_matches_worktree = $true
        relevant_source_matches_worktree = $true
        policy_changes = @()
        semantic_baseline_changes = @()
        validation_input_mismatches = @()
        release_relevant_changes = @()
        sensitive_changes = @()
        policy_baseline_conflict = $false
        policy_product_conflict = $false
        baseline_product_conflict = $false
        baseline_changed = $false
        candidate_transition = $false
        discovery_state = "unchanged"
        free_validation_required = $true
        free_validation = [pscustomobject]@{
            passed = $true
            commands = @([pscustomobject]@{
                    id = "fixture"
                    status = "pass"
                    exit_code = 0
                    timed_out = $false
                    change_report = [pscustomobject]@{ status = "unchanged" }
                })
        }
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $reportPath -Encoding UTF8
    [pscustomobject]@{
        status = "pass"
        producer = "build-v005-non-agent-gates.ps1"
        evidence_kind = "cache_regression_gate_v1"
        evidence_path = $textPath
        evidence_sha256 = (Get-FileHash -LiteralPath $textPath -Algorithm SHA256).Hash.ToLowerInvariant()
        structured_evidence_path = $reportPath
        structured_evidence_sha256 = (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToLowerInvariant()
        command = Get-CacheRegressionFormalGateCommand $reportPath
        exit_code = 0
        generated_at = $GeneratedAt
        git_commit = $Head
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
    }
}
