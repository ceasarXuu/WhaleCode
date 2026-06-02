param(
    [Parameter(Mandatory = $true)][string]$PairDir,
    [Parameter(Mandatory = $true)][string]$Reviewer,
    [Parameter(Mandatory = $true)]
    [ValidateSet(
        "include_taskspace_better",
        "include_standard_better",
        "include_no_clear_delta",
        "exclude_harness_failure",
        "exclude_invalid_prompt",
        "exclude_validator_unclear",
        "exclude_privacy_or_sample_risk"
    )]
    [string]$Decision,
    [Parameter(Mandatory = $true)][string]$ClaimScope,
    [Parameter(Mandatory = $true)][string]$DecisionRationale,
    [switch]$RuntimeProofReviewed,
    [switch]$RunnerEquivalenceReviewed,
    [switch]$IsolationProofReviewed,
    [switch]$SourceGuardReviewed,
    [switch]$SourcePinReviewed,
    [switch]$HashFreshnessReviewed,
    [switch]$SideOutcomesReviewed,
    [switch]$Disagreement,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\audit-report.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\pair-report.ps1")

$pair = (Resolve-Path -LiteralPath $PairDir).Path
$auditPath = Join-Path $pair "audit-review.json"
if ((Test-Path -LiteralPath $auditPath) -and -not $Force) {
    throw "audit-review.json already exists. Use -Force to replace it after re-reviewing artifacts."
}

foreach ($text in @($Reviewer, $ClaimScope, $DecisionRationale)) {
    if ([string]::IsNullOrWhiteSpace($text)) { throw "Reviewer, ClaimScope, and DecisionRationale are required." }
}

$required = @(Get-TaskspaceRequiredAuditArtifacts $pair)
$hashes = [ordered]@{}
foreach ($relative in $required) {
    if (-not (Test-TaskspaceAuditArtifactPath $pair $relative)) {
        throw "Required audit artifact is missing: $relative"
    }
    $hashes[$relative] = Get-TaskspaceAuditArtifactSha256 $pair $relative
}

$review = [ordered]@{
    reviewer = $Reviewer
    date = (Get-Date -Format "yyyy-MM-dd")
    artifact_basis = @($required)
    artifact_hashes = $hashes
    decision = $Decision
    claim_scope = $ClaimScope
    disagreement = [bool]$Disagreement
    attestations = [ordered]@{
        runtime_proof_reviewed = [bool]$RuntimeProofReviewed
        runner_equivalence_reviewed = [bool]$RunnerEquivalenceReviewed
        isolation_proof_reviewed = [bool]$IsolationProofReviewed
        source_guard_reviewed = [bool]$SourceGuardReviewed
        source_pin_reviewed = [bool]$SourcePinReviewed
        hash_freshness_reviewed = [bool]$HashFreshnessReviewed
        side_outcomes_reviewed = [bool]$SideOutcomesReviewed
    }
    decision_rationale = $DecisionRationale
    notes = "Written by explicit reviewer command; benchmark runner does not auto-generate completed audit reviews."
}

($review | ConvertTo-Json -Depth 12) | Set-Content -LiteralPath $auditPath -Encoding UTF8
$result = Get-TaskspaceAuditReview $pair "" 0 $ClaimScope
if (-not $result.completed) {
    throw "Written audit review is incomplete: $(@($result.failures) -join ', ')"
}
Write-Host "AuditReview: $auditPath"
