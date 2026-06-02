param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\audit-report.ps1")
. (Join-Path $PSScriptRoot "lib\e3-proof.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-proof-selftest" }
$runDir = New-Dir (Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function New-TestFile([string]$Path, [string]$Text = "artifact") {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    Set-Content -LiteralPath $Path -Encoding UTF8 -Value $Text
}

function New-ProofLog([string]$Path, [bool]$WithWrapper = $true) {
    $lines = @(
        "validator_proof_nonce=0123456789abcdef0123456789abcdef",
        "validator_wrapper_sha256=$('a' * 64)",
        "validator_entry_sha256=$('b' * 64)",
        "validator_runtime=terminal_bench_docker_app",
        "container_workdir=/app",
        "docker_inspect_available=True",
        "test_dir=/tbench-validator/tests",
        "validator_mount=/tbench-validator",
        "validator_mount_readonly=true"
    )
    if ($WithWrapper) { $lines = @("validator_runtime_probe=terminal_bench_docker_wrapper") + $lines }
    New-TestFile $Path ($lines -join "`n")
}

function New-Metrics([string]$ArtifactDir, [bool]$WithWrapper = $true, [bool]$BusinessSuccess = $true) {
    $validationOut = Join-Path $ArtifactDir "validation.stdout.log"
    New-ProofLog $validationOut $WithWrapper
    New-TestFile (Join-Path $ArtifactDir "validation.stderr.log") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.jsonl") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.stderr.log") ""
    New-TestFile (Join-Path $ArtifactDir "last-message.md") ""
    [pscustomobject]@{
        logical_mode = "standard"
        public_validation_exit_code = $(if ($BusinessSuccess) { 0 } else { 1 })
        validation_stdout_path = $validationOut
        validation_stderr_path = Join-Path $ArtifactDir "validation.stderr.log"
        jsonl_path = Join-Path $ArtifactDir "whale-exec.jsonl"
        stderr_path = Join-Path $ArtifactDir "whale-exec.stderr.log"
        last_message_path = Join-Path $ArtifactDir "last-message.md"
        business_success = $BusinessSuccess
    }
}

$scenarioRoot = New-Dir (Join-Path $runDir "scenario")
$validatorSource = New-Dir (Join-Path $scenarioRoot "external-validator-source")
New-TestFile (Join-Path $validatorSource "verify.sh") "echo ok"
$validatorSha = Get-TaskspaceDirectorySha256 $validatorSource
$manifest = [pscustomobject]@{
    ScenarioRoot = $scenarioRoot
    SampleOrigin = [pscustomobject]@{ original_validator_sha256 = $validatorSha }
    ExternalBenchmark = [pscustomobject]@{
        name = "terminal-bench"
        adapter_version = "test"
        validator_source_dir = "external-validator-source"
        validator_fidelity = [pscustomobject]@{
            official_runner_or_equivalent = $true
            agent_cannot_read_validator_source = $false
            e3_eligible = $true
        }
    }
}
$pairDir = New-Dir (Join-Path $runDir "pair")
$pair = [pscustomobject]@{
    PairDir = $pairDir
    left = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $pairDir "left\repo") }
    right = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $pairDir "right\repo") }
}
$metricsBySide = @{
    left = (New-Metrics (New-Dir (Join-Path $pairDir "left\artifacts")) $true)
    right = (New-Metrics (New-Dir (Join-Path $pairDir "right\artifacts")) $true)
}
$proof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide
Assert-True ($proof.validator_fidelity.runtime_proven) "runtime proof did not accept complete structured markers"
Assert-True (-not $proof.validator_fidelity.agent_cannot_read_validator_source) "declared false source isolation was promoted from placement proof alone"

$manifestMissingSource = $manifest.PSObject.Copy()
$manifestMissingSource.ExternalBenchmark = $manifest.ExternalBenchmark.PSObject.Copy()
$manifestMissingSource.ExternalBenchmark.validator_source_dir = "missing-source"
$missingProof = New-TaskspaceExternalEvidenceProof $pair $manifestMissingSource $metricsBySide
Assert-True (-not $missingProof.validator_fidelity.agent_cannot_read_validator_source) "missing validator source proved isolation"

$manifestHashMismatch = $manifest.PSObject.Copy()
$manifestHashMismatch.SampleOrigin = [pscustomobject]@{ original_validator_sha256 = ("0" * 64) }
$hashMismatchProof = New-TaskspaceExternalEvidenceProof $pair $manifestHashMismatch $metricsBySide
Assert-True (-not $hashMismatchProof.validator_fidelity.agent_cannot_read_validator_source) "validator source hash mismatch proved isolation"

$noReadonlyArtifact = New-Dir (Join-Path $pairDir "left-no-readonly\artifacts")
New-ProofLog (Join-Path $noReadonlyArtifact "validation.stdout.log") $true
(Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $noReadonlyArtifact "validation.stdout.log")).Replace("validator_mount_readonly=true", "validator_mount_readonly=false") |
    Set-Content -LiteralPath (Join-Path $noReadonlyArtifact "validation.stdout.log") -Encoding UTF8
New-TestFile (Join-Path $noReadonlyArtifact "validation.stderr.log") ""
New-TestFile (Join-Path $noReadonlyArtifact "whale-exec.jsonl") ""
New-TestFile (Join-Path $noReadonlyArtifact "whale-exec.stderr.log") ""
New-TestFile (Join-Path $noReadonlyArtifact "last-message.md") ""
$metricsBySide.left = [pscustomobject]@{
    logical_mode = "standard"; public_validation_exit_code = 0
    validation_stdout_path = Join-Path $noReadonlyArtifact "validation.stdout.log"
    validation_stderr_path = Join-Path $noReadonlyArtifact "validation.stderr.log"
    jsonl_path = Join-Path $noReadonlyArtifact "whale-exec.jsonl"
    stderr_path = Join-Path $noReadonlyArtifact "whale-exec.stderr.log"
    last_message_path = Join-Path $noReadonlyArtifact "last-message.md"
    business_success = $true
}
$noReadonlyProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide
Assert-True (-not $noReadonlyProof.validator_fidelity.official_runner_or_equivalent) "missing readonly marker kept official runner equivalence"

$metricsBySide.left = New-Metrics (New-Dir (Join-Path $pairDir "left2\artifacts")) $false
$spoofProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide
Assert-True (-not $spoofProof.validator_fidelity.runtime_proven) "runtime proof accepted logs without wrapper marker"

$promptGuardOk = [pscustomobject]@{ invalid_prompt = $false; manual_review_required = $false }
$e3Config = [pscustomobject]@{ claim_scope = "scope"; minimum_repeats = 5 }
$origin = [pscustomobject]@{
    type = "external_benchmark"; source = "terminal-bench"; source_version = "rev"
    source_url = "https://example.invalid"; license = "license"; data_policy = "policy"
    sample_id = "sample"; original_prompt_sha256 = "abc"; original_validator_sha256 = "def"
}
$external = [pscustomobject]@{ name = "terminal-bench"; adapter_version = "test"; validator_fidelity = [pscustomobject]@{} }
$sideOutcomes = [pscustomobject]@{ standard_success = $false; taskspace_success = $true }
$badDecision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $origin $external $e3Config $true $true 5 "include_no_clear_delta" $false $proof $sideOutcomes
Assert-True (@($badDecision.e3_gate_failures) -contains "e3_human_review_decision_inconsistent_with_outcome") "one-sided outcome matched include_no_clear_delta"

$auditPair = New-Dir (Join-Path $runDir "audit-pair")
foreach ($relative in @(Get-TaskspaceRequiredAuditArtifacts $pairDir)) {
    $source = Join-Path $pairDir $relative
    if (Test-Path -LiteralPath $source) {
        $dest = Join-Path $auditPair $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $dest) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $dest -Force
    } else {
        New-TestFile (Join-Path $auditPair $relative)
    }
}
New-TestFile (Join-Path $auditPair "manifest.resolved.json") (@{ external_benchmark = @{ name = "terminal-bench" } } | ConvertTo-Json -Depth 5)
New-TestFile (Join-Path $auditPair "right\artifacts\observability\action-map-observability.json") "{}"
$basis = @(Get-TaskspaceRequiredAuditArtifacts $auditPair)
$hashes = [ordered]@{}
foreach ($relative in $basis) { $hashes[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $auditPair $relative)).Hash.ToLowerInvariant() }
@{
    reviewer = "codex"; date = "2026-06-02"; artifact_basis = $basis; artifact_hashes = $hashes
    decision = "include_no_clear_delta"; claim_scope = "scope"; disagreement = $false
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $auditPair "audit-review.json") -Encoding UTF8
$weakAudit = Get-TaskspaceAuditReview $auditPair "" 0 "scope"
Assert-True (@($weakAudit.failures | Where-Object { $_ -like "audit_attestation_missing_or_false:*" }).Count -gt 0) "external audit without attestations was accepted"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace E3 proof harness self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace E3 proof harness self-test: PASS"
Write-Host "RunRoot: $runDir"
